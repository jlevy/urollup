//! Process-wide memory admission: one byte ledger per invocation (scalable-ingestion
//! plan, "Process-Wide Admission").
//!
//! The ledger holds `E`, the modeled heap of the current phase, and refuses as soon as
//! `F + H × E` would exceed the budget `B`: `F` is the process baseline and `H` the
//! headroom ([`ProcessModel`]). It bounds an estimate; it does not cap RSS or physical
//! footprint.
//!
//! - **Parallel phases** (decode) [`MemoryAdmission::charge`] one monotone counter.
//!   Charges are read-modify-write operations on that counter, so they are totally
//!   ordered, and a charge is refused exactly when the running total would pass the
//!   limit. That happens if and only if the phase's charges sum past it, whatever the
//!   worker count or scheduling. The first refusal sets the stop flag, and workers stop at
//!   their next record. Each charge names its [`Component`], for statistics.
//! - **Transitions** run only on the coordinating thread after workers join, so holds and
//!   phases never change while a parallel phase charges, which would make refusal depend
//!   on the schedule. [`MemoryAdmission::checkpoint`] replaces the phase's charges with an
//!   estimate from exact counts; [`MemoryAdmission::commit`] replaces an agent's phase
//!   charges with its retained ledger; [`MemoryAdmission::hold`] changes holds and keeps
//!   the phase's charges. Each changes the counter in one read-modify-write and is refused
//!   only when its final total passes the limit, whatever the order of the holds it raises
//!   and lowers. As defense in depth, a transition that breaks the contract still never
//!   overwrites a charge made meanwhile.
//! - **Holds** ([`Hold`]) are state that outlives a phase, such as discovery metadata or a
//!   committed ledger; charges made with [`MemoryAdmission::charge_until_exit`], such as
//!   new process interns, survive every checkpoint.
//! - **Rows:** the per-agent ceiling on request-bearing observations (`--max-rows`, and
//!   until process-wide admission is wired in, the row ceiling derived from `--max-ram`)
//!   is counted here too, separately from bytes.
//!
//! [`model`] prices each unit of retained state and each phase, and [`deep_size`] measures
//! committed state. Nothing in ingestion charges bytes yet; adapters use the row ceiling
//! only.

pub mod deep_size;
pub mod model;

use std::collections::BTreeMap;
use std::fmt;
use std::num::NonZeroUsize;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock, PoisonError};

use super::capacity::{MemoryBudget, ObservationCapacity, Rounding, format_memory};
use crate::selection::Agent;

/// The headroom `H`, as an exact ratio so admission never depends on floating point.
///
/// It covers allocator size-class rounding, memory the allocator keeps after frees,
/// pages a footprint or RSS counts that urollup never requested, and zstd's C allocations.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Headroom {
    numerator: u64,
    denominator: u64,
}

impl Headroom {
    /// `numerator / denominator`, or `None` unless it is a ratio of at least one.
    pub const fn ratio(numerator: u64, denominator: u64) -> Option<Self> {
        if denominator == 0 || numerator < denominator {
            None
        } else {
            Some(Self { numerator, denominator })
        }
    }

    /// `H × heap`, rounded up.
    pub fn apply(self, heap: u64) -> u64 {
        let scaled = u128::from(heap) * u128::from(self.numerator);
        u64::try_from(scaled.div_ceil(u128::from(self.denominator))).unwrap_or(u64::MAX)
    }

    /// The largest heap `E` with `H × E ≤ available`.
    pub fn heap_within(self, available: u64) -> u64 {
        let heap =
            u128::from(available) * u128::from(self.denominator) / u128::from(self.numerator);
        u64::try_from(heap).unwrap_or(u64::MAX)
    }
}

/// The process constants of the budget check `F + H × E ≤ B`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProcessModel {
    /// `F`: binary, runtime, timezone data and C-library state, measured rather than
    /// charged, so it takes no headroom.
    pub baseline: u64,
    /// `H`.
    pub headroom: Headroom,
}

impl ProcessModel {
    /// The measured baseline [`model::BASELINE_BYTES`] and the placeholder `H` of 1.5,
    /// until calibration replaces it.
    pub const DEFAULT: Self = Self {
        baseline: model::BASELINE_BYTES,
        headroom: Headroom { numerator: 3, denominator: 2 },
    };

    /// `F + H × heap`: the whole-process estimate a heap term implies.
    pub fn whole_process(self, heap: u64) -> u64 {
        self.baseline.saturating_add(self.headroom.apply(heap))
    }
}

/// The phases of an invocation, in order, each with the agent it works on, so a refusal
/// can only name a phase and agent that go together. Decode through the session index
/// run once per agent.
///
/// `Discovery` is the initial phase, for the charges and holds made while both agents'
/// sources are discovered and cataloged. The checkpoint before an agent's decode, which
/// charges that agent's worker slots, starts `Decode` of that agent, so its refusal and
/// every refused worker charge read, for example, `reading Codex rollouts`.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Phase {
    /// Discovery and cataloging of both agents' sources.
    Discovery,
    /// Parallel decode of one agent's sources.
    Decode(Agent),
    /// Building one agent's observations (`normalize`, `reconcile_input`).
    Construction(Agent),
    /// Grouping one agent's observations into requests.
    Grouping(Agent),
    /// Sorting one agent's requests, reconciling its limits and compacting its
    /// diagnostics.
    Finalize(Agent),
    /// Indexing one agent's sessions.
    SessionIndex(Agent),
    /// Querying the ledgers and rendering the output.
    Query,
}

impl Phase {
    /// The agent the phase works on; discovery and query cover both agents.
    pub const fn agent(self) -> Option<Agent> {
        match self {
            Self::Discovery | Self::Query => None,
            Self::Decode(agent)
            | Self::Construction(agent)
            | Self::Grouping(agent)
            | Self::Finalize(agent)
            | Self::SessionIndex(agent) => Some(agent),
        }
    }

    /// What the invocation was doing, as a refusal names it.
    fn activity(self) -> String {
        match self {
            Self::Discovery => "cataloging discovered sources".to_owned(),
            Self::Decode(agent) => format!("reading {} {}", agent_name(agent), source_noun(agent)),
            Self::Construction(agent) => format!("building {} observations", agent_name(agent)),
            Self::Grouping(agent) => format!("reconciling {} requests", agent_name(agent)),
            Self::Finalize(agent) => format!("finalizing the {} ledger", agent_name(agent)),
            Self::SessionIndex(agent) => format!("indexing {} sessions", agent_name(agent)),
            Self::Query => "querying and rendering the output".to_owned(),
        }
    }

    /// The stable token for statistics.
    pub const fn token(self) -> &'static str {
        match self {
            Self::Discovery => "discovery",
            Self::Decode(_) => "decode",
            Self::Construction(_) => "construction",
            Self::Grouping(_) => "grouping",
            Self::Finalize(_) => "finalize",
            Self::SessionIndex(_) => "session_index",
            Self::Query => "query",
        }
    }
}

const fn agent_name(agent: Agent) -> &'static str {
    match agent {
        Agent::Claude => "Claude Code",
        Agent::Codex => "Codex",
        Agent::Pi => "Pi",
    }
}

const fn source_noun(agent: Agent) -> &'static str {
    match agent {
        Agent::Claude => "transcripts",
        Agent::Codex => "rollouts",
        Agent::Pi => "sessions",
    }
}

/// The advice every memory refusal ends with. It names no `--max-ram` value, since an
/// example could be at or below the budget it refused; slice 7 adds the smallest
/// sufficient percent of the effective memory where one exists.
const ADVICE: &str = "raise --max-ram, select fewer sessions with --session, or pass narrower --source roots with --no-default-sources";

/// What a charge pays for. Every charge names one, and the ledger keeps a running total
/// per component, from which the statistics line breaks down a phase's estimate.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Component {
    /// Decoded records, request-bearing or not.
    Records,
    /// The request structure κ of a request-bearing record.
    Kappa,
    /// Record payloads: strings, extras, tool uses, counts, spilled keys and diagnostics.
    Payloads,
    /// Provider limit rows.
    Limits,
    /// New process interns, charged until exit by [`MemoryAdmission::charge_until_exit`].
    Interns,
    /// Source and thread rows.
    Sources,
    /// The query and session-index reserves.
    Reserves,
    /// Worker slots and the large-record permit, which
    /// [`MemoryAdmission::charge_large_record`] charges.
    Slots,
}

impl Component {
    /// Every component, in the statistics line's order.
    pub const ALL: [Self; 8] = [
        Self::Records,
        Self::Kappa,
        Self::Payloads,
        Self::Limits,
        Self::Interns,
        Self::Sources,
        Self::Reserves,
        Self::Slots,
    ];

    /// The stable token for statistics.
    pub const fn token(self) -> &'static str {
        match self {
            Self::Records => "records",
            Self::Kappa => "kappa",
            Self::Payloads => "payloads",
            Self::Limits => "limits",
            Self::Interns => "interns",
            Self::Sources => "sources",
            Self::Reserves => "reserves",
            Self::Slots => "slots",
        }
    }

    const fn index(self) -> usize {
        match self {
            Self::Records => 0,
            Self::Kappa => 1,
            Self::Payloads => 2,
            Self::Limits => 3,
            Self::Interns => 4,
            Self::Sources => 5,
            Self::Reserves => 6,
            Self::Slots => 7,
        }
    }
}

/// Why admission refused an invocation.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum CapacityError {
    /// More request-bearing observations than the per-agent row ceiling. The text is the
    /// row ceiling's existing message.
    #[error(
        "{observations} request observations exceed the reconciliation capacity of {maximum} compact rows ({limit})"
    )]
    Rows {
        /// The agent whose rows passed the ceiling.
        agent: Agent,
        /// The first refused row: the ceiling plus one, a lower bound rather than a count
        /// of the whole input.
        observations: usize,
        /// The ceiling.
        maximum: usize,
        /// What the ceiling was derived from.
        limit: String,
    },
    /// The estimated whole-process memory would exceed the budget.
    #[error("{}", memory_refusal(*.phase, *.estimate, .label))]
    Memory {
        /// The phase that refused, with its agent.
        phase: Phase,
        /// `F + H × E` for a refused transition; a refused charge stops at the first
        /// charge that passes the limit and gives none.
        estimate: Option<u64>,
        /// The budget in bytes.
        budget: u64,
        /// What the budget is.
        label: String,
    },
    /// The budget cannot hold the baseline and one decoding worker.
    #[error(
        "the memory budget of {label} is below the {} minimum for the process baseline and one decoding worker; raise --max-ram",
        format_memory(*.floor, Rounding::Up)
    )]
    BelowFloor {
        /// `F + H × b` for one worker slot `b`.
        floor: u64,
        /// The budget in bytes.
        budget: u64,
        /// What the budget is.
        label: String,
    },
}

fn memory_refusal(phase: Phase, estimate: Option<u64>, label: &str) -> String {
    let activity = phase.activity();
    match estimate {
        Some(estimate) => format!(
            "estimated memory for {activity} ({}) exceeds the budget of {label}; {ADVICE}",
            format_memory(estimate, Rounding::Up)
        ),
        None => {
            format!("estimated memory exceeds the budget of {label} while {activity}; {ADVICE}")
        }
    }
}

/// State whose charge outlives the phase that created it.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Hold {
    /// An agent's discovery metadata, until that agent's ingest returns.
    Discovery(Agent),
    /// An agent's retained ledger, from its commit until exit; it shrinks when the
    /// discovery tables are released.
    Ledger(Agent),
    /// The session index.
    SessionIndex,
}

/// What bounds `E`.
#[derive(Debug)]
enum Limit {
    /// No byte budget: nothing is refused, and `E` saturates rather than overflows.
    Unlimited,
    /// A budget and the largest `E` it admits, `⌊(B − F) / H⌋`, or `None` when `B` is
    /// below `F` and not even an empty heap fits.
    Budget { budget: MemoryBudget, heap: Option<u64> },
}

impl Limit {
    const fn is_unlimited(&self) -> bool {
        match self {
            Self::Unlimited => true,
            Self::Budget { .. } => false,
        }
    }

    /// `total` as the next value of `E`, or the budget that refuses it.
    fn admit(&self, total: u128) -> Result<u64, &MemoryBudget> {
        match self {
            Self::Unlimited => Ok(u64::try_from(total).unwrap_or(u64::MAX)),
            Self::Budget { budget, heap } => heap
                .and_then(|heap| u64::try_from(total).ok().filter(|total| *total <= heap))
                .ok_or(budget),
        }
    }
}

/// A refused move of `E`: the budget that refused it and the total it would have reached.
struct Refused<'a> {
    budget: &'a MemoryBudget,
    total: u128,
}

/// One invocation's memory and row admission; see the module documentation.
#[derive(Debug)]
pub struct MemoryAdmission {
    limit: Limit,
    model: ProcessModel,
    /// `E`: every hold, every charge until exit, and the current phase's charges.
    charged: AtomicU64,
    /// The part of `charged` made by [`Self::charge_until_exit`].
    until_exit: AtomicU64,
    /// The largest `E` on either side of a transition. Only charges move `E` between
    /// transitions, and they only add, so the largest `E` so far is the larger of this and
    /// the current `E`.
    largest: AtomicU64,
    /// The bytes each [`Component`] has been charged, over every phase.
    components: [AtomicU64; Component::ALL.len()],
    /// The holds; the lock also orders transitions.
    holds: Mutex<BTreeMap<Hold, u64>>,
    /// The phase charges are attributed to.
    phase: Mutex<Phase>,
    /// The large-record permit's charge: the largest need seen in the current decode.
    large_record: Mutex<u64>,
    stopped: AtomicBool,
    refusal: OnceLock<CapacityError>,
    rows: [AtomicUsize; 3],
    row_ceiling: Option<ObservationCapacity>,
}

impl MemoryAdmission {
    /// A ledger for `budget` under `model`'s baseline and headroom, with no row ceiling.
    /// A budget below the baseline admits nothing.
    pub fn new(budget: MemoryBudget, model: ProcessModel) -> Self {
        let heap = budget
            .bytes()
            .checked_sub(model.baseline)
            .map(|available| model.headroom.heap_within(available));
        Self::with_limit(Limit::Budget { budget, heap }, model)
    }

    /// A ledger that never refuses bytes: `E` saturates instead of overflowing, and stays
    /// saturated through hold changes until a checkpoint or commit recomputes it. Rows
    /// still count against a row ceiling.
    pub fn unlimited() -> Self {
        Self::with_limit(Limit::Unlimited, ProcessModel::DEFAULT)
    }

    fn with_limit(limit: Limit, model: ProcessModel) -> Self {
        Self {
            limit,
            model,
            charged: AtomicU64::new(0),
            until_exit: AtomicU64::new(0),
            largest: AtomicU64::new(0),
            components: std::array::from_fn(|_| AtomicU64::new(0)),
            holds: Mutex::new(BTreeMap::new()),
            phase: Mutex::new(Phase::Discovery),
            large_record: Mutex::new(0),
            stopped: AtomicBool::new(false),
            refusal: OnceLock::new(),
            rows: std::array::from_fn(|_| AtomicUsize::new(0)),
            row_ceiling: None,
        }
    }

    /// Adds a per-agent ceiling on request-bearing observations.
    #[must_use]
    pub fn with_row_ceiling(mut self, ceiling: ObservationCapacity) -> Self {
        self.row_ceiling = Some(ceiling);
        self
    }

    /// The byte budget, or `None` for an unlimited ledger.
    pub fn budget(&self) -> Option<&MemoryBudget> {
        match &self.limit {
            Limit::Unlimited => None,
            Limit::Budget { budget, .. } => Some(budget),
        }
    }

    /// The baseline and headroom the budget check uses.
    pub const fn model(&self) -> ProcessModel {
        self.model
    }

    /// Refuses a budget below `F + H × slot_bytes`, the baseline and one decoding worker,
    /// before any work. A budget below `F` alone refuses whatever the slot size.
    pub fn ensure_floor(&self, slot_bytes: u64) -> Result<(), CapacityError> {
        let Limit::Budget { budget, heap } = &self.limit else { return Ok(()) };
        if heap.is_some_and(|heap| heap >= slot_bytes) {
            return Ok(());
        }
        Err(self.refuse(CapacityError::BelowFloor {
            floor: self.model.whole_process(slot_bytes),
            budget: budget.bytes(),
            label: budget.label().to_owned(),
        }))
    }

    /// How many decoding threads to run: `clamp(⌊B / (8 × H × slot_bytes)⌋, 1,
    /// max(8, jobs))`, where `jobs` is an explicit `UROLLUP_JOBS`.
    ///
    /// The count depends on the budget and on an explicit `jobs` above 8, never on the
    /// default worker count, so admission charges the same for one to eight workers.
    pub fn worker_slots(&self, slot_bytes: u64, jobs: Option<NonZeroUsize>) -> NonZeroUsize {
        let ceiling = jobs.map_or(8, NonZeroUsize::get).max(8);
        let slots = match self.budget() {
            None => ceiling,
            Some(budget) => {
                let per_slot = u128::from(slot_bytes)
                    .saturating_mul(8)
                    .saturating_mul(u128::from(self.model.headroom.numerator))
                    .max(1);
                let slots = u128::from(budget.bytes())
                    .saturating_mul(u128::from(self.model.headroom.denominator))
                    / per_slot;
                usize::try_from(slots).unwrap_or(usize::MAX).clamp(1, ceiling)
            }
        };
        NonZeroUsize::new(slots).unwrap_or(NonZeroUsize::MIN)
    }

    /// Whether any charge or reservation has been refused.
    pub fn stopped(&self) -> bool {
        self.stopped.load(Ordering::Acquire)
    }

    /// The first refusal, once there is one.
    pub fn refusal(&self) -> Option<CapacityError> {
        self.refusal.get().cloned()
    }

    /// Charges `bytes` of `component` to the current phase before they are allocated;
    /// `false` refuses, and every later charge and reservation is refused too.
    #[must_use]
    pub fn charge(&self, component: Component, bytes: u64) -> bool {
        if self.stopped() {
            return false;
        }
        if let Err(refused) = self.update(|total| u128::from(total) + u128::from(bytes)) {
            let phase = *lock(&self.phase);
            self.refuse(memory_error(refused.budget, phase, None));
            return false;
        }
        // Statistics only: no admitted total under a budget comes near 2^64.
        self.components[component.index()].fetch_add(bytes, Ordering::Relaxed);
        true
    }

    /// Charges `bytes` of a new process intern, which stays allocated until the process
    /// exits; checkpoints and commits keep it.
    #[must_use]
    pub fn charge_until_exit(&self, bytes: u64) -> bool {
        if !self.charge(Component::Interns, bytes) {
            return false;
        }
        saturating_add(&self.until_exit, bytes);
        true
    }

    /// Raises the large-record permit's charge to `need` when it is larger than any need
    /// seen in this decode, charging only the increase to [`Component::Slots`], so the
    /// final charge is the input's largest need in any order.
    #[must_use]
    pub fn charge_large_record(&self, need: u64) -> bool {
        let mut level = lock(&self.large_record);
        if need <= *level {
            return !self.stopped();
        }
        let charged = self.charge(Component::Slots, need - *level);
        if charged {
            *level = need;
        }
        charged
    }

    /// Reserves one request-bearing observation of `agent` against the row ceiling.
    ///
    /// Completed workers keep their reservations until the agent's ingest ends. No more
    /// than the ceiling's maximum rows per agent are admitted.
    #[must_use]
    pub fn reserve_row(&self, agent: Agent) -> bool {
        if self.stopped() {
            return false;
        }
        let Some(ceiling) = &self.row_ceiling else { return true };
        let maximum = ceiling.maximum();
        if self.rows[row_index(agent)]
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |count| {
                (count < maximum).then(|| count + 1)
            })
            .is_err()
        {
            self.refuse(CapacityError::Rows {
                agent,
                observations: maximum.saturating_add(1),
                maximum,
                limit: ceiling.label().to_owned(),
            });
            return false;
        }
        true
    }

    /// The request-bearing observations of `agent` reserved so far.
    pub fn rows(&self, agent: Agent) -> usize {
        self.rows[row_index(agent)].load(Ordering::Relaxed)
    }

    /// Starts `phase` with `estimate` as its modeled heap and sets each hold in `holds`
    /// (zero releases one), replacing the previous phase's charges and releasing the
    /// large-record permit's charge.
    ///
    /// Charges until exit and the other holds stay. Call it on the coordinating thread
    /// after workers join: it replaces the phase's charges, so a charge made meanwhile
    /// belongs to no defined phase. Refuses, with the exact whole-process estimate, only
    /// when the final total would pass the budget, and then changes nothing; after any
    /// refusal, returns the first one.
    pub fn checkpoint(
        &self,
        phase: Phase,
        estimate: u64,
        holds: &[(Hold, u64)],
    ) -> Result<(), CapacityError> {
        self.transition(Some(phase), Some(estimate), holds)
    }

    /// Sets each hold in `changes` (zero releases one), keeping the current phase's
    /// charges.
    ///
    /// Like [`Self::checkpoint`], call it on the coordinating thread after workers join.
    /// A hold that changes while workers charge makes refusal depend on whether a charge
    /// lands before or after it, which admission's determinism forbids; the change is
    /// still one read-modify-write that loses no charge, but callers must not rely on it.
    /// The changes apply together, so raising one hold while lowering another refuses
    /// only when the final total passes the budget, in either order. A refusal names the
    /// current phase, changes nothing, and is final.
    pub fn hold(&self, changes: &[(Hold, u64)]) -> Result<(), CapacityError> {
        self.transition(None, None, changes)
    }

    /// Ends `agent`'s ingest in one transition: its phase charges and discovery metadata
    /// are released, and its retained ledger is held at `retained`, its deep size after
    /// finalize. Charges are then attributed to finalizing that agent's ledger until the
    /// next checkpoint. Call it as [`Self::checkpoint`] is called.
    pub fn commit(&self, agent: Agent, retained: u64) -> Result<(), CapacityError> {
        self.transition(
            Some(Phase::Finalize(agent)),
            Some(0),
            &[(Hold::Discovery(agent), 0), (Hold::Ledger(agent), retained)],
        )
    }

    /// The current modeled heap `E`.
    pub fn charged(&self) -> u64 {
        self.charged.load(Ordering::Relaxed)
    }

    /// The bytes charged to `component` so far, over every phase; statistics take the
    /// difference across a phase. Checkpoint estimates and holds are not included: their
    /// callers compute them from the cost model, component by component.
    pub fn charged_by(&self, component: Component) -> u64 {
        self.components[component.index()].load(Ordering::Relaxed)
    }

    /// The largest modeled heap reached so far.
    ///
    /// A successful run's largest gives the threshold budget `F + H × largest` only for
    /// budgets that leave [`Self::worker_slots`] unchanged: the slot count depends on the
    /// budget, and so do the slot charges.
    pub fn largest(&self) -> u64 {
        self.largest.load(Ordering::Relaxed).max(self.charged())
    }

    /// Moves `E` from its current value `e` to `next(e)` in one read-modify-write, when
    /// the limit admits it, and returns the previous and new values.
    fn update(&self, next: impl Fn(u64) -> u128) -> Result<(u64, u64), Refused<'_>> {
        let mut current = self.charged.load(Ordering::Relaxed);
        loop {
            let total = next(current);
            let admitted = self.limit.admit(total).map_err(|budget| Refused { budget, total })?;
            match self.charged.compare_exchange_weak(
                current,
                admitted,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(previous) => return Ok((previous, admitted)),
                Err(actual) => current = actual,
            }
        }
    }

    /// Sets the holds in `changes` and, with an `estimate`, replaces the phase's charges
    /// with it and starts `phase`, all in one move of `E`: `E − released + held`.
    fn transition(
        &self,
        phase: Option<Phase>,
        estimate: Option<u64>,
        changes: &[(Hold, u64)],
    ) -> Result<(), CapacityError> {
        let mut holds = lock(&self.holds);
        if let Some(refusal) = self.refusal() {
            return Err(refusal);
        }
        let mut next_holds = holds.clone();
        for (hold, bytes) in changes {
            if *bytes == 0 {
                next_holds.remove(hold);
            } else {
                next_holds.insert(*hold, *bytes);
            }
        }
        let (held_before, held_after) = (sum(&holds), sum(&next_holds));
        let unlimited = self.limit.is_unlimited();
        let moved = self.update(|current| {
            let until_exit = || u128::from(self.until_exit.load(Ordering::Relaxed));
            match estimate {
                Some(estimate) => {
                    // After workers join, a budget's `E` is exactly every hold, every charge
                    // until exit and the phase's charges, which the estimate replaces.
                    // Without a budget `E` may have saturated, and a `hold` overlapping an
                    // intern charge can see it in `E` before `until_exit`, so only this
                    // case is checked.
                    debug_assert!(
                        unlimited || u128::from(current) >= held_before + until_exit(),
                        "`E` must cover every hold and charge until exit once workers join"
                    );
                    until_exit() + held_after + u128::from(estimate)
                }
                // Once `E` saturates without a budget its true value is unknown, so a hold
                // change keeps it saturated until a checkpoint recomputes it.
                None if unlimited && current == u64::MAX => u128::from(u64::MAX),
                None => u128::from(current).saturating_sub(held_before) + held_after,
            }
        });
        match moved {
            Ok((previous, next)) => {
                *holds = next_holds;
                if let Some(phase) = phase {
                    *lock(&self.phase) = phase;
                }
                if estimate.is_some() {
                    *lock(&self.large_record) = 0;
                }
                self.largest.fetch_max(previous.max(next), Ordering::Relaxed);
                Ok(())
            }
            Err(Refused { budget, total }) => {
                let phase = phase.unwrap_or_else(|| *lock(&self.phase));
                let heap = u64::try_from(total).unwrap_or(u64::MAX);
                let estimate = self.model.whole_process(heap);
                Err(self.refuse(memory_error(budget, phase, Some(estimate))))
            }
        }
    }

    /// Records `error` unless an earlier refusal is already recorded, stops every
    /// worker, and returns the refusal the invocation reports.
    fn refuse(&self, error: CapacityError) -> CapacityError {
        let first = self.refusal.get_or_init(|| error).clone();
        self.stopped.store(true, Ordering::Release);
        first
    }
}

fn memory_error(budget: &MemoryBudget, phase: Phase, estimate: Option<u64>) -> CapacityError {
    CapacityError::Memory {
        phase,
        estimate,
        budget: budget.bytes(),
        label: budget.label().to_owned(),
    }
}

fn sum(holds: &BTreeMap<Hold, u64>) -> u128 {
    holds.values().map(|bytes| u128::from(*bytes)).sum()
}

/// Adds `bytes` to `counter`, saturating rather than wrapping.
fn saturating_add(counter: &AtomicU64, bytes: u64) {
    let mut current = counter.load(Ordering::Relaxed);
    while let Err(actual) = counter.compare_exchange_weak(
        current,
        current.saturating_add(bytes),
        Ordering::Relaxed,
        Ordering::Relaxed,
    ) {
        current = actual;
    }
}

impl fmt::Display for Hold {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Discovery(agent) => write!(f, "{} discovery", agent.token()),
            Self::Ledger(agent) => write!(f, "{} ledger", agent.token()),
            Self::SessionIndex => f.write_str("session index"),
        }
    }
}

const fn row_index(agent: Agent) -> usize {
    match agent {
        Agent::Claude => 0,
        Agent::Codex => 1,
        Agent::Pi => 2,
    }
}

/// Locks `mutex`, recovering the data after a panicking holder: every critical section
/// leaves the guarded value consistent.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ledger::reconcile::ReconcileError;

    const MIB: u64 = 1 << 20;

    /// A ledger whose heap limit is exactly `limit`: no baseline, a headroom of one.
    fn ledger(limit: u64) -> MemoryAdmission {
        let model = ProcessModel { baseline: 0, headroom: Headroom::ratio(1, 1).unwrap() };
        MemoryAdmission::new(MemoryBudget::exact(limit, format!("--max-ram {limit}")), model)
    }

    /// The default baseline `F`, which calibration may change.
    const BASELINE: u64 = ProcessModel::DEFAULT.baseline;

    /// A budget one byte below the default baseline.
    fn below_baseline() -> MemoryAdmission {
        MemoryAdmission::new(MemoryBudget::exact(BASELINE - 1, "below F"), ProcessModel::DEFAULT)
    }

    const ADVICE_TEXT: &str = "; raise --max-ram, select fewer sessions with --session, or pass narrower --source roots with --no-default-sources";

    #[test]
    fn concurrent_charges_never_pass_the_budget() {
        for workers in [1, 2, 3, 8] {
            let admission = ledger(10_007);
            std::thread::scope(|scope| {
                for worker in 0..workers {
                    let admission = &admission;
                    scope.spawn(move || {
                        let mut amount = 1 + worker % 5;
                        while admission.charge(Component::Records, amount) {
                            amount = 1 + (amount * 7 + 3) % 13;
                        }
                    });
                }
            });
            assert!(admission.charged() <= 10_007, "{workers} workers");
            assert!(admission.stopped());
            assert!(!admission.charge(Component::Records, 0));
            assert!(matches!(
                admission.refusal(),
                Some(CapacityError::Memory { phase: Phase::Discovery, estimate: None, .. })
            ));
        }
    }

    #[test]
    fn a_phase_is_refused_exactly_when_its_charges_sum_past_the_budget() {
        // Each worker charges a fixed list, so the phase's sum is the same in every
        // schedule; the outcome must not depend on the schedule or worker count.
        let lists: Vec<Vec<u64>> = (0..8)
            .map(|worker| (0..200).map(|index| 1 + (worker * 31 + index * 17) % 23).collect())
            .collect();
        let sum: u64 = lists.iter().flatten().sum();
        for limit in [sum - 1, sum, sum + 1] {
            for workers in [1, 2, 4, 8] {
                for _ in 0..20 {
                    let admission = ledger(limit);
                    std::thread::scope(|scope| {
                        for chunk in lists.chunks(8 / workers) {
                            let admission = &admission;
                            scope.spawn(move || {
                                for amount in chunk.iter().flatten() {
                                    if !admission.charge(Component::Kappa, *amount) {
                                        return;
                                    }
                                }
                            });
                        }
                    });
                    assert_eq!(
                        admission.stopped(),
                        sum > limit,
                        "limit {limit}, {workers} workers"
                    );
                    assert!(admission.charged() <= limit);
                    if sum <= limit {
                        assert_eq!(admission.charged(), sum);
                        assert_eq!(admission.largest(), sum);
                    }
                }
            }
        }
    }

    #[test]
    fn a_charge_that_would_pass_the_budget_is_not_added() {
        let admission = ledger(100);
        assert!(admission.charge(Component::Records, 60));
        assert!(!admission.charge(Component::Records, 41));
        assert_eq!(admission.charged(), 60);
        // Once stopped, even a charge that would fit is refused.
        assert!(!admission.charge(Component::Records, 1));
        assert!(!admission.reserve_row(Agent::Codex));
    }

    #[test]
    fn checkpoints_replace_phase_charges_and_keep_holds_and_charges_until_exit() {
        let admission = ledger(1000);
        admission
            .hold(&[(Hold::Discovery(Agent::Codex), 100), (Hold::Discovery(Agent::Claude), 50)])
            .unwrap();
        admission.checkpoint(Phase::Decode(Agent::Codex), 200, &[]).unwrap();
        assert_eq!(admission.charged(), 350);
        assert!(admission.charge(Component::Records, 300));
        assert!(admission.charge_until_exit(20));
        assert_eq!(admission.charged(), 670);
        // The checkpoint's exact estimate replaces the forward charges, not the intern.
        admission.checkpoint(Phase::Construction(Agent::Codex), 400, &[]).unwrap();
        assert_eq!(admission.charged(), 570);
        admission.checkpoint(Phase::Grouping(Agent::Codex), 600, &[]).unwrap();
        assert_eq!(admission.charged(), 770);
        // Commit releases the agent's phase charges and discovery, and holds its ledger.
        admission.commit(Agent::Codex, 250).unwrap();
        assert_eq!(admission.charged(), 50 + 20 + 250);
        // Releasing discovery tables shrinks the held ledger; a hold keeps phase charges.
        admission.checkpoint(Phase::SessionIndex(Agent::Codex), 30, &[]).unwrap();
        admission.hold(&[(Hold::Ledger(Agent::Codex), 200)]).unwrap();
        assert_eq!(admission.charged(), 50 + 20 + 200 + 30);
        admission.hold(&[(Hold::Discovery(Agent::Claude), 0)]).unwrap();
        assert_eq!(admission.charged(), 20 + 200 + 30);
        assert_eq!(admission.largest(), 770);
    }

    #[test]
    fn a_checkpoint_sets_holds_together_with_its_estimate() {
        let admission = ledger(1000);
        admission.commit(Agent::Codex, 700).unwrap();
        // The session-index checkpoint takes the index reserve out of the held ledger.
        admission
            .checkpoint(
                Phase::SessionIndex(Agent::Codex),
                100,
                &[(Hold::Ledger(Agent::Codex), 600)],
            )
            .unwrap();
        assert_eq!(admission.charged(), 700);
        // The next checkpoint holds the built index and replaces the index estimate.
        admission
            .checkpoint(Phase::Decode(Agent::Claude), 50, &[(Hold::SessionIndex, 90)])
            .unwrap();
        assert_eq!(admission.charged(), 600 + 90 + 50);
    }

    /// Holds change only after workers join. As defense in depth, a hold changed while
    /// workers charge and intern still loses no charge, and no debug check fires.
    #[test]
    fn a_hold_change_against_the_contract_keeps_charges_made_meanwhile() {
        for _ in 0..4 {
            let admission = ledger(u64::MAX / 4);
            std::thread::scope(|scope| {
                scope.spawn(|| {
                    for _ in 0..200_000 {
                        assert!(admission.charge(Component::Records, 1));
                    }
                });
                for _ in 0..3 {
                    scope.spawn(|| {
                        for _ in 0..100_000 {
                            assert!(admission.charge_until_exit(1));
                        }
                    });
                }
                scope.spawn(|| {
                    for _ in 0..20_000 {
                        admission.hold(&[(Hold::SessionIndex, 5)]).unwrap();
                        admission.hold(&[(Hold::SessionIndex, 0)]).unwrap();
                    }
                });
            });
            assert_eq!(admission.charged(), 500_000);
            // The charges until exit survive a checkpoint.
            admission.checkpoint(Phase::Query, 0, &[]).unwrap();
            assert_eq!(admission.charged(), 300_000);
        }
    }

    /// Runs a two-agent invocation on `workers` threads: every transition kind, parallel
    /// decode charges with permit needs and interns, and a session-index checkpoint that
    /// lowers a hold. Returns whether it completed.
    fn run_invocation(admission: &MemoryAdmission, workers: usize, seed: u64) -> bool {
        let discovery =
            [(Hold::Discovery(Agent::Codex), 4000), (Hold::Discovery(Agent::Claude), 3000)];
        if admission.hold(&discovery).is_err() {
            return false;
        }
        for agent in [Agent::Codex, Agent::Claude] {
            if admission.checkpoint(Phase::Decode(agent), 2000, &[]).is_err() {
                return false;
            }
            let charges: Vec<Vec<u64>> = (0..8)
                .map(|worker| {
                    (0..400).map(|index| 1 + (worker * 31 + index * 17 + seed) % 23).collect()
                })
                .collect();
            let permit_needs: Vec<u64> = (0..8).map(|worker| 100 + worker * 97).collect();
            let shares = charges.chunks(8 / workers).zip(permit_needs.chunks(8 / workers));
            std::thread::scope(|scope| {
                for (worker_charges, worker_needs) in shares {
                    scope.spawn(move || {
                        for (amounts, need) in worker_charges.iter().zip(worker_needs) {
                            for bytes in amounts {
                                if !admission.charge(Component::Records, *bytes) {
                                    return;
                                }
                            }
                            if !admission.charge_large_record(*need)
                                || !admission.charge_until_exit(3)
                            {
                                return;
                            }
                        }
                    });
                }
            });
            if admission.stopped()
                || admission.checkpoint(Phase::Construction(agent), 9000, &[]).is_err()
                || admission.checkpoint(Phase::Grouping(agent), 7000, &[]).is_err()
                || admission.commit(agent, 2500).is_err()
                || admission
                    .checkpoint(Phase::SessionIndex(agent), 600, &[(Hold::Ledger(agent), 2400)])
                    .is_err()
                || admission.hold(&[(Hold::SessionIndex, 500)]).is_err()
            {
                return false;
            }
        }
        admission.checkpoint(Phase::Query, 1200, &[(Hold::SessionIndex, 0)]).is_ok()
    }

    #[test]
    fn the_largest_heap_of_a_successful_run_is_its_exact_threshold() {
        for workers in [1, 2, 4, 8] {
            for seed in 0..10 {
                let probe = ledger(u64::MAX / 4);
                assert!(run_invocation(&probe, workers, seed));
                let threshold = probe.largest();
                for _ in 0..5 {
                    assert!(run_invocation(&ledger(threshold), workers, seed), "{threshold}");
                    let below = ledger(threshold - 1);
                    assert!(!run_invocation(&below, workers, seed), "{threshold} − 1");
                    assert!(below.charged() < threshold);
                }
                // The same threshold as a whole-process budget `F + H × largest`.
                let model = ProcessModel::DEFAULT;
                let budget = model.whole_process(threshold);
                let at = MemoryAdmission::new(MemoryBudget::exact(budget, "T"), model);
                assert!(run_invocation(&at, workers, seed));
                let under = MemoryAdmission::new(MemoryBudget::exact(budget - 1, "T − 1"), model);
                assert!(!run_invocation(&under, workers, seed));
            }
        }
    }

    #[test]
    fn holds_raised_and_lowered_together_are_checked_on_their_final_total() {
        let raise = (Hold::SessionIndex, 400);
        let lower = (Hold::Ledger(Agent::Codex), 300);
        for changes in [[raise, lower], [lower, raise]] {
            let admission = ledger(1000);
            admission.commit(Agent::Codex, 700).unwrap();
            admission.hold(&changes).unwrap();
            assert_eq!(admission.charged(), 700);
            assert_eq!(admission.largest(), 700);
        }
    }

    #[test]
    fn a_refused_hold_names_the_current_phase_and_changes_nothing() {
        let admission = ledger(1000);
        admission.commit(Agent::Codex, 700).unwrap();
        admission.checkpoint(Phase::SessionIndex(Agent::Codex), 100, &[]).unwrap();
        let error = admission.hold(&[(Hold::SessionIndex, 201)]).unwrap_err();
        assert_eq!(
            error,
            CapacityError::Memory {
                phase: Phase::SessionIndex(Agent::Codex),
                estimate: Some(1001),
                budget: 1000,
                label: "--max-ram 1000".into(),
            }
        );
        assert_eq!(admission.charged(), 800);
        assert!(admission.stopped());
        assert_eq!(admission.hold(&[(Hold::SessionIndex, 0)]).unwrap_err(), error);
        assert_eq!(admission.charged(), 800);
    }

    #[test]
    fn a_refused_commit_names_the_agents_ledger_and_changes_nothing() {
        let admission = ledger(1000);
        admission.hold(&[(Hold::Ledger(Agent::Codex), 600)]).unwrap();
        admission.checkpoint(Phase::Finalize(Agent::Claude), 300, &[]).unwrap();
        let error = admission.commit(Agent::Claude, 401).unwrap_err();
        assert_eq!(
            error.to_string(),
            format!(
                "estimated memory for finalizing the Claude Code ledger (1001 bytes) exceeds the budget of --max-ram 1000{ADVICE_TEXT}"
            )
        );
        assert_eq!(admission.charged(), 900);
        // A successful commit attributes later charges to that agent's finalize.
        let admission = ledger(100);
        admission.commit(Agent::Codex, 10).unwrap();
        assert!(!admission.charge(Component::Payloads, 91));
        assert!(matches!(
            admission.refusal(),
            Some(CapacityError::Memory { phase: Phase::Finalize(Agent::Codex), .. })
        ));
    }

    #[test]
    fn a_checkpoint_refuses_with_its_exact_whole_process_estimate() {
        let model = ProcessModel { baseline: 16 * MIB, headroom: Headroom::ratio(3, 2).unwrap() };
        let budget = MemoryBudget::exact(64 * MIB, "--max-ram 64M");
        let admission = MemoryAdmission::new(budget, model);
        // (64 − 16) MiB / 1.5 is a heap limit of exactly 32 MiB.
        admission.checkpoint(Phase::Grouping(Agent::Codex), 32 * MIB, &[]).unwrap();
        let error =
            admission.checkpoint(Phase::Grouping(Agent::Codex), 32 * MIB + 1, &[]).unwrap_err();
        let CapacityError::Memory { estimate: Some(estimate), .. } = &error else {
            unreachable!("expected a memory refusal, got {error:?}");
        };
        assert_eq!(*estimate, 16 * MIB + (3 * (32 * MIB + 1)).div_ceil(2));
        assert!(admission.stopped());
        assert_eq!(admission.charged(), 32 * MIB);
        // Later checkpoints report the first refusal.
        assert_eq!(admission.checkpoint(Phase::Query, 0, &[]).unwrap_err(), error);
        assert_eq!(admission.commit(Agent::Codex, 0).unwrap_err(), error);
    }

    #[test]
    fn refusal_text_is_fixed_per_phase() {
        let budget = MemoryBudget::exact(8 << 30, "25% of 32 GiB physical RAM (8 GiB)");
        let refusal = |phase, estimate| {
            CapacityError::Memory {
                phase,
                estimate,
                budget: budget.bytes(),
                label: budget.label().to_owned(),
            }
            .to_string()
        };
        // 9.31 GiB, which an estimate prints rounded up.
        let estimate = Some(10_000_000_000);
        for (phase, activity) in [
            (Phase::Discovery, "cataloging discovered sources"),
            (Phase::Decode(Agent::Claude), "reading Claude Code transcripts"),
            (Phase::Decode(Agent::Pi), "reading Pi sessions"),
            (Phase::Construction(Agent::Codex), "building Codex observations"),
            (Phase::Grouping(Agent::Codex), "reconciling Codex requests"),
            (Phase::Finalize(Agent::Claude), "finalizing the Claude Code ledger"),
            (Phase::SessionIndex(Agent::Codex), "indexing Codex sessions"),
            (Phase::Query, "querying and rendering the output"),
        ] {
            assert_eq!(
                refusal(phase, estimate),
                format!(
                    "estimated memory for {activity} (9.4 GiB) exceeds the budget of 25% of 32 GiB physical RAM (8 GiB){ADVICE_TEXT}"
                )
            );
        }
        assert_eq!(
            refusal(Phase::Decode(Agent::Codex), None),
            format!(
                "estimated memory exceeds the budget of 25% of 32 GiB physical RAM (8 GiB) while reading Codex rollouts{ADVICE_TEXT}"
            )
        );
        // The row refusal keeps the row ceiling's message, which the adapters convert it to.
        let rows = CapacityError::Rows {
            agent: Agent::Codex,
            observations: 3,
            maximum: 2,
            limit: "2 rows".into(),
        };
        let reconcile = ReconcileError::CapacityExceeded {
            observations: 3,
            maximum: 2,
            limit: "2 rows".into(),
        };
        assert_eq!(rows.to_string(), reconcile.to_string());
        assert_eq!(
            rows.to_string(),
            "3 request observations exceed the reconciliation capacity of 2 compact rows (2 rows)"
        );
    }

    #[test]
    fn a_charge_refused_in_decode_names_the_agent_of_the_decode_checkpoint() {
        let admission = ledger(100);
        // The checkpoint before Codex decode charges its worker slots.
        admission.checkpoint(Phase::Decode(Agent::Codex), 40, &[]).unwrap();
        assert!(!admission.charge(Component::Records, 61));
        let error = admission.refusal().unwrap();
        assert_eq!(
            error,
            CapacityError::Memory {
                phase: Phase::Decode(Agent::Codex),
                estimate: None,
                budget: 100,
                label: "--max-ram 100".into(),
            }
        );
        assert_eq!(
            error.to_string(),
            format!(
                "estimated memory exceeds the budget of --max-ram 100 while reading Codex rollouts{ADVICE_TEXT}"
            )
        );
    }

    #[test]
    fn the_large_record_permit_charges_only_the_largest_need() {
        let admission = ledger(1000);
        assert!(admission.charge_large_record(300));
        assert!(admission.charge_large_record(200));
        assert!(admission.charge_large_record(500));
        assert_eq!(admission.charged(), 500);
        // A checkpoint ends the decode: the permit's charge is replaced, and the next
        // decode charges its own largest need again.
        admission.checkpoint(Phase::Decode(Agent::Claude), 0, &[]).unwrap();
        assert!(admission.charge_large_record(100));
        assert_eq!(admission.charged(), 100);
        assert_eq!(admission.charged_by(Component::Slots), 600);
        assert!(!admission.charge_large_record(1001));
    }

    #[test]
    fn charges_are_totaled_by_component_across_phases() {
        let admission = ledger(1000);
        assert!(admission.charge(Component::Records, 30));
        assert!(admission.charge(Component::Kappa, 20));
        admission.checkpoint(Phase::Construction(Agent::Codex), 10, &[]).unwrap();
        assert!(admission.charge(Component::Records, 5));
        assert!(admission.charge_until_exit(7));
        assert!(admission.charge_large_record(11));
        // A refused charge adds nothing.
        assert!(!admission.charge(Component::Payloads, 1000));
        assert_eq!(
            Component::ALL.map(|component| admission.charged_by(component)),
            [35, 20, 0, 0, 7, 0, 0, 11]
        );
        assert_eq!(
            Component::ALL.map(Component::token),
            ["records", "kappa", "payloads", "limits", "interns", "sources", "reserves", "slots"]
        );
    }

    #[test]
    fn concurrent_reservations_never_exceed_the_per_agent_ceiling() {
        let admission =
            MemoryAdmission::unlimited().with_row_ceiling(ObservationCapacity::from_rows(97));
        std::thread::scope(|scope| {
            for _ in 0..8 {
                let admission = &admission;
                scope.spawn(move || while admission.reserve_row(Agent::Codex) {});
            }
        });
        assert_eq!(admission.rows(Agent::Codex), 97);
        assert_eq!(admission.rows(Agent::Claude), 0);
        assert!(admission.stopped());
        assert!(!admission.reserve_row(Agent::Claude));
        assert_eq!(
            admission.refusal(),
            Some(CapacityError::Rows {
                agent: Agent::Codex,
                observations: 98,
                maximum: 97,
                limit: "97 rows".into(),
            })
        );
        let empty =
            MemoryAdmission::unlimited().with_row_ceiling(ObservationCapacity::from_rows(0));
        assert!(!empty.reserve_row(Agent::Claude));
    }

    #[test]
    fn rows_are_counted_per_agent() {
        let admission =
            MemoryAdmission::unlimited().with_row_ceiling(ObservationCapacity::from_rows(1));
        assert!(admission.reserve_row(Agent::Codex));
        assert!(admission.reserve_row(Agent::Claude));
        assert!(!admission.reserve_row(Agent::Codex));
        assert!(MemoryAdmission::unlimited().reserve_row(Agent::Codex));
    }

    #[test]
    fn an_unlimited_ledger_never_refuses_bytes() {
        let admission = MemoryAdmission::unlimited();
        assert!(admission.charge(Component::Records, u64::MAX / 2));
        assert!(admission.charge(Component::Records, u64::MAX / 2));
        assert!(admission.ensure_floor(u64::MAX).is_ok());
        admission.checkpoint(Phase::Decode(Agent::Codex), u64::MAX, &[]).unwrap();
        // `E` saturates rather than overflowing into a refusal.
        assert!(admission.charge(Component::Records, 1));
        assert_eq!(admission.charged(), u64::MAX);
        assert!(admission.charge_until_exit(u64::MAX));
        assert!(admission.charge_large_record(u64::MAX));
        admission.hold(&[(Hold::SessionIndex, u64::MAX)]).unwrap();
        admission.commit(Agent::Codex, u64::MAX).unwrap();
        admission.checkpoint(Phase::Query, u64::MAX, &[]).unwrap();
        assert_eq!(admission.charged(), u64::MAX);
        assert!(!admission.stopped());
        assert_eq!(admission.refusal(), None);
        assert_eq!(admission.budget(), None);
        assert_eq!(admission.worker_slots(u64::MAX, None).get(), 8);
    }

    #[test]
    fn a_saturated_unlimited_ledger_stays_saturated_until_a_checkpoint() {
        let admission = MemoryAdmission::unlimited();
        assert!(admission.charge_until_exit(1));
        admission.hold(&[(Hold::SessionIndex, u64::MAX)]).unwrap();
        assert_eq!(admission.charged(), u64::MAX);
        // The true total is unknown once `E` saturates, so releasing the hold keeps it
        // saturated rather than dropping below the charge until exit.
        admission.hold(&[(Hold::SessionIndex, 0)]).unwrap();
        assert_eq!(admission.charged(), u64::MAX);
        admission.hold(&[]).unwrap();
        // A checkpoint recomputes `E` from the holds, the charges until exit and its estimate.
        admission.checkpoint(Phase::Query, 10, &[]).unwrap();
        assert_eq!(admission.charged(), 11);
        assert!(!admission.stopped());
    }

    #[test]
    fn worker_slots_follow_the_budget_and_an_explicit_job_count_above_eight() {
        let model = ProcessModel::DEFAULT;
        let slot = 12 * MIB;
        let slots = |budget: u64, jobs: Option<usize>| {
            MemoryAdmission::new(MemoryBudget::exact(budget, "test"), model)
                .worker_slots(slot, jobs.and_then(NonZeroUsize::new))
                .get()
        };
        // ⌊B / (8 × 1.5 × 12 MiB)⌋ = ⌊B / 144 MiB⌋.
        assert_eq!(slots(144 * MIB - 1, None), 1);
        assert_eq!(slots(3 * 144 * MIB, None), 3);
        assert_eq!(slots(8 << 30, None), 8);
        assert_eq!(slots(8 << 30, Some(4)), 8);
        assert_eq!(slots(8 << 30, Some(32)), 32);
        assert_eq!(slots(8 << 30, Some(64)), 56);
    }

    #[test]
    fn a_budget_below_one_worker_slot_refuses_before_any_work() {
        let model = ProcessModel { baseline: 16 * MIB, headroom: Headroom::ratio(3, 2).unwrap() };
        let slot = 12 * MIB;
        // F + H × b = 16 + 18 = 34 MiB.
        let at_floor = MemoryAdmission::new(MemoryBudget::exact(34 * MIB, "--max-ram 34M"), model);
        assert_eq!(at_floor.ensure_floor(slot), Ok(()));
        let below =
            MemoryAdmission::new(MemoryBudget::exact(34 * MIB - 1, "--max-ram tiny"), model);
        let error = below.ensure_floor(slot).unwrap_err();
        assert_eq!(
            error.to_string(),
            "the memory budget of --max-ram tiny is below the 34 MiB minimum for the process baseline and one decoding worker; raise --max-ram"
        );
        assert!(below.stopped());
    }

    #[test]
    fn a_budget_below_the_baseline_admits_nothing() {
        // Not even an empty slot or an empty heap fits below `F`.
        assert_eq!(
            below_baseline().ensure_floor(0),
            Err(CapacityError::BelowFloor {
                floor: BASELINE,
                budget: BASELINE - 1,
                label: "below F".into(),
            })
        );
        assert_eq!(
            below_baseline().checkpoint(Phase::Query, 0, &[]),
            Err(CapacityError::Memory {
                phase: Phase::Query,
                estimate: Some(BASELINE),
                budget: BASELINE - 1,
                label: "below F".into(),
            })
        );
        assert!(below_baseline().hold(&[]).is_err());
        let admission = below_baseline();
        assert!(!admission.charge(Component::Records, 0));
        assert!(admission.stopped());
        // A budget of exactly `F` admits an empty heap and nothing more.
        let at_baseline =
            MemoryAdmission::new(MemoryBudget::exact(BASELINE, "F"), ProcessModel::DEFAULT);
        assert_eq!(at_baseline.ensure_floor(0), Ok(()));
        at_baseline.checkpoint(Phase::Query, 0, &[]).unwrap();
        assert!(!at_baseline.charge(Component::Records, 1));
    }

    #[test]
    fn headroom_rounds_toward_the_conservative_side() {
        let half = Headroom::ratio(3, 2).unwrap();
        assert_eq!(half.apply(3), 5);
        assert_eq!(half.heap_within(5), 3);
        assert_eq!(half.apply(u64::MAX), u64::MAX);
        assert_eq!(Headroom::ratio(1, 2), None);
        assert_eq!(Headroom::ratio(1, 0), None);
        assert_eq!(ProcessModel::DEFAULT.whole_process(2 * MIB), model::BASELINE_BYTES + 3 * MIB);
    }
}
