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
//!   their next record.
//! - **Checkpoints** run on the coordinating thread after workers join.
//!   [`MemoryAdmission::checkpoint`] replaces the phase's forward charges with an
//!   estimate from exact counts, and [`MemoryAdmission::commit`] replaces an agent's
//!   decode and construction charges with its retained ledger.
//! - **Holds** ([`Hold`]) are state that outlives a phase, such as discovery metadata or a
//!   committed ledger; charges made with [`MemoryAdmission::charge_until_exit`], such as
//!   new process interns, survive every checkpoint.
//! - **Rows:** the per-agent ceiling on request-bearing observations (`--max-rows`, and
//!   until process-wide admission is wired in, the row ceiling derived from `--max-ram`)
//!   is counted here too, separately from bytes.
//!
//! Nothing in ingestion charges bytes yet; adapters use the row ceiling only.

use std::collections::BTreeMap;
use std::fmt;
use std::num::NonZeroUsize;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock, PoisonError};

use super::capacity::{MemoryBudget, ObservationCapacity, format_memory};
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
    /// The placeholder constants until calibration: `F` of 16 MiB and `H` of 1.5.
    pub const DEFAULT: Self =
        Self { baseline: 16 << 20, headroom: Headroom { numerator: 3, denominator: 2 } };

    /// `F + H × heap`: the whole-process estimate a heap term implies.
    pub fn whole_process(self, heap: u64) -> u64 {
        self.baseline.saturating_add(self.headroom.apply(heap))
    }
}

/// The phases of an invocation, in order. Decode and construction through finalize run
/// once per agent.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Phase {
    /// Discovery and cataloging of both agents' sources.
    Discovery,
    /// Parallel decode of one agent's sources.
    Decode,
    /// Building one agent's observations (`normalize`, `reconcile_input`).
    Construction,
    /// Grouping one agent's observations into requests.
    Grouping,
    /// Sorting requests, reconciling limits and compacting diagnostics.
    Finalize,
    /// Indexing one agent's sessions.
    SessionIndex,
    /// Querying the ledgers and rendering the output.
    Query,
}

impl Phase {
    /// What the invocation was doing, as a refusal names it.
    fn activity(self, agent: Option<Agent>) -> String {
        let agent = agent.map_or("", agent_name);
        match self {
            Self::Discovery => "cataloging discovered sources".to_owned(),
            Self::Decode => format!("reading {agent} {}", source_noun(agent)),
            Self::Construction => format!("building {agent} observations"),
            Self::Grouping => format!("reconciling {agent} requests"),
            Self::Finalize => format!("finalizing the {agent} ledger"),
            Self::SessionIndex => format!("indexing {agent} sessions"),
            Self::Query => "querying and rendering the output".to_owned(),
        }
    }

    /// The stable token for statistics.
    pub const fn token(self) -> &'static str {
        match self {
            Self::Discovery => "discovery",
            Self::Decode => "decode",
            Self::Construction => "construction",
            Self::Grouping => "grouping",
            Self::Finalize => "finalize",
            Self::SessionIndex => "session_index",
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

fn source_noun(agent: &str) -> &'static str {
    match agent {
        "Codex" => "rollouts",
        "Claude Code" => "transcripts",
        _ => "sessions",
    }
}

const ADVICE: &str = "raise --max-ram (for example --max-ram 50%), select fewer sessions with --session, or pass narrower --source roots with --no-default-sources";

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
    #[error("{}", memory_refusal(*.phase, *.agent, *.estimate, .label))]
    Memory {
        /// The phase that refused.
        phase: Phase,
        /// The agent it was working on, when it was working on one.
        agent: Option<Agent>,
        /// `F + H × E` at a checkpoint; a decode refusal stops at the first charge that
        /// passes the limit and gives none.
        estimate: Option<u64>,
        /// The budget in bytes.
        budget: u64,
        /// What the budget is.
        label: String,
    },
    /// The budget cannot hold the baseline and one decoding worker.
    #[error(
        "the memory budget of {label} is below the {} that one decoding worker needs; raise --max-ram",
        format_memory(*.floor)
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

fn memory_refusal(
    phase: Phase,
    agent: Option<Agent>,
    estimate: Option<u64>,
    label: &str,
) -> String {
    let activity = phase.activity(agent);
    match estimate {
        Some(estimate) => format!(
            "estimated memory for {activity} ({}) exceeds the budget of {label}; {ADVICE}",
            format_memory(estimate)
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

/// The phase and agent that charges are attributed to.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Stage {
    phase: Phase,
    agent: Option<Agent>,
}

/// One invocation's memory and row admission; see the module documentation.
#[derive(Debug)]
pub struct MemoryAdmission {
    budget: Option<MemoryBudget>,
    model: ProcessModel,
    /// The largest heap term `E` within the budget, or `u64::MAX` without one.
    heap_limit: u64,
    /// `E`: every hold, every charge until exit, and the current phase's charges.
    charged: AtomicU64,
    /// The part of `charged` made by [`Self::charge_until_exit`].
    until_exit: AtomicU64,
    /// The largest `E` reached so far.
    largest: AtomicU64,
    holds: Mutex<BTreeMap<Hold, u64>>,
    stage: Mutex<Stage>,
    /// The large-record permit's charge: the largest need seen in the current decode.
    large_record: Mutex<u64>,
    stopped: AtomicBool,
    refusal: OnceLock<CapacityError>,
    rows: [AtomicUsize; 3],
    row_ceiling: Option<ObservationCapacity>,
}

impl MemoryAdmission {
    /// A ledger for `budget` under `model`'s baseline and headroom, with no row ceiling.
    pub fn new(budget: MemoryBudget, model: ProcessModel) -> Self {
        let available = budget.bytes().saturating_sub(model.baseline);
        let heap_limit = model.headroom.heap_within(available);
        Self::with_limit(Some(budget), model, heap_limit)
    }

    /// A ledger that never refuses bytes. Rows still count against a row ceiling.
    pub fn unlimited() -> Self {
        Self::with_limit(None, ProcessModel::DEFAULT, u64::MAX)
    }

    fn with_limit(budget: Option<MemoryBudget>, model: ProcessModel, heap_limit: u64) -> Self {
        Self {
            budget,
            model,
            heap_limit,
            charged: AtomicU64::new(0),
            until_exit: AtomicU64::new(0),
            largest: AtomicU64::new(0),
            holds: Mutex::new(BTreeMap::new()),
            stage: Mutex::new(Stage { phase: Phase::Discovery, agent: None }),
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
        self.budget.as_ref()
    }

    /// The baseline and headroom the budget check uses.
    pub const fn model(&self) -> ProcessModel {
        self.model
    }

    /// Refuses a budget below `F + H × slot_bytes`, one decoding worker, before any work.
    pub fn ensure_floor(&self, slot_bytes: u64) -> Result<(), CapacityError> {
        let Some(budget) = &self.budget else { return Ok(()) };
        if self.heap_limit >= slot_bytes {
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
        let slots = match &self.budget {
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

    /// Charges `bytes` to the current phase before they are allocated; `false` refuses,
    /// and every later charge and reservation is refused too.
    pub fn charge(&self, bytes: u64) -> bool {
        if self.stopped() {
            return false;
        }
        let limit = self.heap_limit;
        let admitted = self.charged.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |total| {
            total.checked_add(bytes).filter(|next| *next <= limit)
        });
        if let Ok(previous) = admitted {
            self.largest.fetch_max(previous.saturating_add(bytes), Ordering::Relaxed);
            return true;
        }
        let stage = *lock(&self.stage);
        self.refuse(self.memory_error(stage, None));
        false
    }

    /// Charges `bytes` that stay allocated until the process exits, such as a new
    /// process intern; checkpoints and commits keep them.
    pub fn charge_until_exit(&self, bytes: u64) -> bool {
        let charged = self.charge(bytes);
        if charged {
            self.until_exit.fetch_add(bytes, Ordering::Relaxed);
        }
        charged
    }

    /// Raises the large-record permit's charge to `need` when it is larger than any need
    /// seen in this decode, charging only the increase, so the final charge is the
    /// input's largest need in any order.
    pub fn charge_large_record(&self, need: u64) -> bool {
        let mut level = lock(&self.large_record);
        if need <= *level {
            return !self.stopped();
        }
        let charged = self.charge(need - *level);
        if charged {
            *level = need;
        }
        charged
    }

    /// Reserves one request-bearing observation of `agent` against the row ceiling.
    ///
    /// Completed workers keep their reservations until the agent's ingest ends. No more
    /// than the ceiling's maximum rows per agent are admitted.
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

    /// Starts `phase` of `agent` with `estimate` as its modeled heap, replacing the
    /// previous phase's charges and releasing the large-record permit's charge.
    ///
    /// Holds and charges until exit stay. Call it on the coordinating thread, with no
    /// worker charging. Refuses, with the exact whole-process estimate, when the total
    /// would pass the budget; after any refusal, returns the first one.
    pub fn checkpoint(
        &self,
        phase: Phase,
        agent: Option<Agent>,
        estimate: u64,
    ) -> Result<(), CapacityError> {
        let holds = lock(&self.holds);
        *lock(&self.stage) = Stage { phase, agent };
        *lock(&self.large_record) = 0;
        let total = self.settled(&holds).saturating_add(estimate);
        self.settle(total, Stage { phase, agent })
    }

    /// Sets `hold` to `bytes`, keeping the current phase's charges; zero releases it.
    pub fn hold(&self, hold: Hold, bytes: u64) -> Result<(), CapacityError> {
        let mut holds = lock(&self.holds);
        let phase_charges = self.phase_charges(&holds);
        if bytes == 0 {
            holds.remove(&hold);
        } else {
            holds.insert(hold, bytes);
        }
        let total = self.settled(&holds).saturating_add(phase_charges);
        let stage = *lock(&self.stage);
        self.settle(total, stage)
    }

    /// Ends `agent`'s ingest: its decode and construction charges and its discovery
    /// metadata are released, and its retained ledger is held at `retained`, its deep
    /// size after finalize.
    pub fn commit(&self, agent: Agent, retained: u64) -> Result<(), CapacityError> {
        let mut holds = lock(&self.holds);
        holds.remove(&Hold::Discovery(agent));
        if retained == 0 {
            holds.remove(&Hold::Ledger(agent));
        } else {
            holds.insert(Hold::Ledger(agent), retained);
        }
        *lock(&self.large_record) = 0;
        let total = self.settled(&holds);
        self.settle(total, Stage { phase: Phase::Finalize, agent: Some(agent) })
    }

    /// The current modeled heap `E`.
    pub fn charged(&self) -> u64 {
        self.charged.load(Ordering::Relaxed)
    }

    /// The largest modeled heap reached so far; a successful run's largest gives the
    /// budget threshold `F + H × largest`.
    pub fn largest(&self) -> u64 {
        self.largest.load(Ordering::Relaxed)
    }

    fn settled(&self, holds: &BTreeMap<Hold, u64>) -> u64 {
        holds.values().fold(self.until_exit.load(Ordering::Relaxed), |total, bytes| {
            total.saturating_add(*bytes)
        })
    }

    fn phase_charges(&self, holds: &BTreeMap<Hold, u64>) -> u64 {
        self.charged().saturating_sub(self.settled(holds))
    }

    /// Makes `total` the modeled heap, or refuses at `stage` when it passes the limit.
    fn settle(&self, total: u64, stage: Stage) -> Result<(), CapacityError> {
        if let Some(refusal) = self.refusal() {
            return Err(refusal);
        }
        if total > self.heap_limit {
            let estimate = self.model.whole_process(total);
            return Err(self.refuse(self.memory_error(stage, Some(estimate))));
        }
        self.charged.store(total, Ordering::Relaxed);
        self.largest.fetch_max(total, Ordering::Relaxed);
        Ok(())
    }

    fn memory_error(&self, stage: Stage, estimate: Option<u64>) -> CapacityError {
        let (budget, label) =
            self.budget.as_ref().map_or((u64::MAX, ""), |budget| (budget.bytes(), budget.label()));
        CapacityError::Memory {
            phase: stage.phase,
            agent: stage.agent,
            estimate,
            budget,
            label: label.to_owned(),
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

    const MIB: u64 = 1 << 20;

    /// A ledger whose heap limit is exactly `limit`: no baseline, a headroom of one.
    fn ledger(limit: u64) -> MemoryAdmission {
        let model = ProcessModel { baseline: 0, headroom: Headroom::ratio(1, 1).unwrap() };
        MemoryAdmission::new(MemoryBudget::exact(limit, format!("--max-ram {limit}")), model)
    }

    #[test]
    fn concurrent_charges_never_pass_the_budget() {
        for workers in [1, 2, 3, 8] {
            let admission = ledger(10_007);
            std::thread::scope(|scope| {
                for worker in 0..workers {
                    let admission = &admission;
                    scope.spawn(move || {
                        let mut amount = 1 + worker % 5;
                        while admission.charge(amount) {
                            amount = 1 + (amount * 7 + 3) % 13;
                        }
                    });
                }
            });
            assert!(admission.charged() <= 10_007, "{workers} workers");
            assert!(admission.stopped());
            assert!(!admission.charge(0));
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
                                    if !admission.charge(*amount) {
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
        assert!(admission.charge(60));
        assert!(!admission.charge(41));
        assert_eq!(admission.charged(), 60);
        // Once stopped, even a charge that would fit is refused.
        assert!(!admission.charge(1));
        assert!(!admission.reserve_row(Agent::Codex));
    }

    #[test]
    fn checkpoints_replace_phase_charges_and_keep_holds_and_charges_until_exit() {
        let admission = ledger(1000);
        admission.hold(Hold::Discovery(Agent::Codex), 100).unwrap();
        admission.hold(Hold::Discovery(Agent::Claude), 50).unwrap();
        admission.checkpoint(Phase::Decode, Some(Agent::Codex), 200).unwrap();
        assert_eq!(admission.charged(), 350);
        assert!(admission.charge(300));
        assert!(admission.charge_until_exit(20));
        assert_eq!(admission.charged(), 670);
        // The checkpoint's exact estimate replaces the forward charges, not the intern.
        admission.checkpoint(Phase::Construction, Some(Agent::Codex), 400).unwrap();
        assert_eq!(admission.charged(), 570);
        admission.checkpoint(Phase::Grouping, Some(Agent::Codex), 600).unwrap();
        assert_eq!(admission.charged(), 770);
        // Commit releases the agent's phase charges and discovery, and holds its ledger.
        admission.commit(Agent::Codex, 250).unwrap();
        assert_eq!(admission.charged(), 50 + 20 + 250);
        // Releasing discovery tables shrinks the held ledger; a hold keeps phase charges.
        admission.checkpoint(Phase::SessionIndex, Some(Agent::Codex), 30).unwrap();
        admission.hold(Hold::Ledger(Agent::Codex), 200).unwrap();
        assert_eq!(admission.charged(), 50 + 20 + 200 + 30);
        admission.hold(Hold::Discovery(Agent::Claude), 0).unwrap();
        assert_eq!(admission.charged(), 20 + 200 + 30);
        assert_eq!(admission.largest(), 770);
    }

    #[test]
    fn a_checkpoint_refuses_with_its_exact_whole_process_estimate() {
        let model = ProcessModel { baseline: 16 * MIB, headroom: Headroom::ratio(3, 2).unwrap() };
        let budget = MemoryBudget::exact(64 * MIB, "--max-ram 64M");
        let admission = MemoryAdmission::new(budget, model);
        // (64 − 16) MiB / 1.5 is a heap limit of exactly 32 MiB.
        admission.checkpoint(Phase::Grouping, Some(Agent::Codex), 32 * MIB).unwrap();
        let error =
            admission.checkpoint(Phase::Grouping, Some(Agent::Codex), 32 * MIB + 1).unwrap_err();
        let CapacityError::Memory { estimate: Some(estimate), .. } = &error else {
            unreachable!("expected a memory refusal, got {error:?}");
        };
        assert_eq!(*estimate, 16 * MIB + (3 * (32 * MIB + 1)).div_ceil(2));
        assert!(admission.stopped());
        // Later checkpoints report the first refusal.
        assert_eq!(admission.checkpoint(Phase::Query, None, 0).unwrap_err(), error);
        assert_eq!(admission.commit(Agent::Codex, 0).unwrap_err(), error);
    }

    #[test]
    fn refusal_text_is_fixed_per_phase() {
        let budget = MemoryBudget::exact(8 << 30, "25% of 32 GiB physical RAM (8 GiB)");
        let refusal = |phase, agent, estimate| {
            CapacityError::Memory {
                phase,
                agent,
                estimate,
                budget: budget.bytes(),
                label: budget.label().to_owned(),
            }
            .to_string()
        };
        let advice = "; raise --max-ram (for example --max-ram 50%), select fewer sessions with --session, or pass narrower --source roots with --no-default-sources";
        let estimate = Some(10_093_173_555);
        for (phase, agent, activity) in [
            (Phase::Discovery, None, "cataloging discovered sources"),
            (Phase::Decode, Some(Agent::Claude), "reading Claude Code transcripts"),
            (Phase::Construction, Some(Agent::Codex), "building Codex observations"),
            (Phase::Grouping, Some(Agent::Codex), "reconciling Codex requests"),
            (Phase::Finalize, Some(Agent::Claude), "finalizing the Claude Code ledger"),
            (Phase::SessionIndex, Some(Agent::Codex), "indexing Codex sessions"),
            (Phase::Query, None, "querying and rendering the output"),
        ] {
            assert_eq!(
                refusal(phase, agent, estimate),
                format!(
                    "estimated memory for {activity} (9.4 GiB) exceeds the budget of 25% of 32 GiB physical RAM (8 GiB){advice}"
                )
            );
        }
        assert_eq!(
            refusal(Phase::Decode, Some(Agent::Codex), None),
            format!(
                "estimated memory exceeds the budget of 25% of 32 GiB physical RAM (8 GiB) while reading Codex rollouts{advice}"
            )
        );
        let rows = CapacityError::Rows {
            agent: Agent::Codex,
            observations: 3,
            maximum: 2,
            limit: "2 rows".into(),
        };
        assert_eq!(
            rows.to_string(),
            "3 request observations exceed the reconciliation capacity of 2 compact rows (2 rows)"
        );
    }

    #[test]
    fn a_decode_refusal_names_the_phase_of_the_last_checkpoint() {
        let admission = ledger(100);
        admission.checkpoint(Phase::Decode, Some(Agent::Claude), 10).unwrap();
        assert!(!admission.charge(91));
        let error = admission.refusal().unwrap();
        assert_eq!(
            error,
            CapacityError::Memory {
                phase: Phase::Decode,
                agent: Some(Agent::Claude),
                estimate: None,
                budget: 100,
                label: "--max-ram 100".into(),
            }
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
        admission.checkpoint(Phase::Decode, Some(Agent::Claude), 0).unwrap();
        assert!(admission.charge_large_record(100));
        assert_eq!(admission.charged(), 100);
        assert!(!admission.charge_large_record(1001));
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
        assert!(admission.charge(u64::MAX / 2));
        assert!(admission.charge(u64::MAX / 2));
        assert!(admission.ensure_floor(u64::MAX).is_ok());
        admission.checkpoint(Phase::Query, None, u64::MAX).unwrap();
        assert_eq!(admission.worker_slots(u64::MAX, None).get(), 8);
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
            "the memory budget of --max-ram tiny is below the 34 MiB that one decoding worker needs; raise --max-ram"
        );
        assert!(below.stopped());
        let under_baseline = MemoryAdmission::new(MemoryBudget::exact(MIB, "--max-ram 1M"), model);
        assert!(under_baseline.ensure_floor(1).is_err());
    }

    #[test]
    fn headroom_rounds_toward_the_conservative_side() {
        let half = Headroom::ratio(3, 2).unwrap();
        assert_eq!(half.apply(3), 5);
        assert_eq!(half.heap_within(5), 3);
        assert_eq!(half.apply(u64::MAX), u64::MAX);
        assert_eq!(Headroom::ratio(1, 2), None);
        assert_eq!(Headroom::ratio(1, 0), None);
        assert_eq!(ProcessModel::DEFAULT.whole_process(2 * MIB), 19 * MIB);
    }
}
