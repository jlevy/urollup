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
//! - **The large-record permit** ([`MemoryAdmission::large_record`]) lets one thread at a
//!   time hold a line buffer, zstd window or `quotaLimits` document larger than a worker
//!   slot allows. It is re-entrant for its holder, whose need is the sum of the shares it
//!   holds, and is free again once the holder drops its last share. Other threads wait,
//!   and stop waiting once a refusal sets the stop flag. Its charge is the largest need
//!   any holder has held at once in the current phase.
//!
//! - **Process interns** charge the ledger the interning thread is scoped to
//!   ([`MemoryAdmission::scope`]), under their table's lock and only when new, so a text
//!   is charged once however many threads intern it; a refused intern stores nothing.
//!
//! [`model`] prices each unit of retained state and each phase, and [`deep_size`] measures
//! committed state. In ingestion so far, the source reader takes the permit; each agent's
//! decode checkpoint charges its worker slots and the parallel read's work order and
//! releases the coordinating slot ([`Hold::Coordinating`]), which the CLI holds while it
//! peeks at sources; each worker charges its source's rows; threads scoped to the ledger
//! charge their new process interns; and the Codex adapter charges its decode,
//! construction, reconciliation and commit. Claude's record, payload and thread charges
//! arrive with slice 6, and the CLI's ledger refuses no bytes until slice 7 gives it a
//! budget; until then only the row ceiling refuses.

pub mod deep_size;
pub mod model;

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fmt;
use std::marker::PhantomData;
use std::num::NonZeroUsize;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, OnceLock, PoisonError};
use std::thread::ThreadId;
use std::time::Duration;

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
    /// Worker slots and the large-record permit, whose charge
    /// [`MemoryAdmission::large_record`] raises.
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
    /// The coordinating thread's worker slot, for the peeks that read sources on that
    /// thread before decode: catalog links for `--session` and `--source`
    /// classification. The first decode checkpoint releases it.
    Coordinating,
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
///
/// The ledger's state is shared with the threads [`Self::scope`] scopes to it, so that a
/// new process intern made on any of them is charged here.
#[derive(Debug)]
pub struct MemoryAdmission {
    shared: Arc<Shared>,
    rows: [AtomicUsize; 3],
    row_ceiling: Option<ObservationCapacity>,
}

/// Where a ledger also stores `E` and whether workers are decoding, for a counting
/// allocator that compares live heap with `E` at every allocation, as the model-bound
/// harness does ([`MemoryAdmission::with_heap_mirror`]).
#[derive(Debug, Default)]
pub struct HeapMirror {
    heap: AtomicU64,
    decoding: AtomicBool,
}

impl HeapMirror {
    /// A mirror of an empty ledger.
    pub const fn new() -> Self {
        Self { heap: AtomicU64::new(0), decoding: AtomicBool::new(false) }
    }

    /// The latest `E`.
    pub fn heap(&self) -> u64 {
        self.heap.load(Ordering::Relaxed)
    }

    /// Whether the latest phase is a decode, whose worker slots are in use.
    pub fn decoding(&self) -> bool {
        self.decoding.load(Ordering::Relaxed)
    }
}

/// The state every charge reads and moves, which scoped threads share with the ledger.
#[derive(Debug)]
struct Shared {
    limit: Limit,
    model: ProcessModel,
    /// `E`: every hold, every charge until exit, and the current phase's charges.
    charged: AtomicU64,
    /// The part of `charged` made by [`MemoryAdmission::charge_until_exit`].
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
    /// The large-record permit: its holder, what the holder holds, and its charge.
    permit: Mutex<Permit>,
    /// Signaled when the permit is released or the invocation stops.
    permit_released: Condvar,
    stopped: AtomicBool,
    refusal: OnceLock<CapacityError>,
    /// Where every new `E` is also stored, for the model-bound harness
    /// ([`MemoryAdmission::with_heap_mirror`]).
    mirror: OnceLock<&'static HeapMirror>,
}

thread_local! {
    /// The ledger this thread's new process interns are charged to, while
    /// [`MemoryAdmission::scope`] runs.
    static SCOPED: RefCell<Option<Arc<Shared>>> = const { RefCell::new(None) };
}

/// Restores the ledger a thread was scoped to before [`MemoryAdmission::scope`], even
/// when the scoped work unwinds.
struct Rescope(Option<Arc<Shared>>);

impl Drop for Rescope {
    fn drop(&mut self) {
        let previous = self.0.take();
        // A thread being torn down has no scope left to restore.
        SCOPED.try_with(|scoped| scoped.replace(previous)).ok();
    }
}

/// Charges `bytes` of a new process intern, until exit, to the ledger the current thread
/// is scoped to ([`MemoryAdmission::scope`]); `false` when that ledger refuses, and then the
/// intern must insert nothing. Outside a scope nothing is charged and every intern is
/// admitted.
///
/// The process tables call it under their lock, before they allocate the intern, so a
/// text two threads intern at once is charged once.
pub(crate) fn charge_intern(bytes: u64) -> bool {
    SCOPED
        .try_with(|scoped| {
            scoped.borrow().as_ref().is_none_or(|shared| shared.charge_until_exit(bytes))
        })
        .unwrap_or(true)
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
            shared: Arc::new(Shared {
                limit,
                model,
                charged: AtomicU64::new(0),
                until_exit: AtomicU64::new(0),
                largest: AtomicU64::new(0),
                components: std::array::from_fn(|_| AtomicU64::new(0)),
                holds: Mutex::new(BTreeMap::new()),
                phase: Mutex::new(Phase::Discovery),
                permit: Mutex::new(Permit::default()),
                permit_released: Condvar::new(),
                stopped: AtomicBool::new(false),
                refusal: OnceLock::new(),
                mirror: OnceLock::new(),
            }),
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

    /// Also stores every new `E`, and whether workers are decoding, in `mirror`, so that a
    /// counting allocator can compare live heap with `E` at every allocation, as the
    /// model-bound harness does. A ledger keeps the first mirror it is given.
    #[must_use]
    pub fn with_heap_mirror(self, mirror: &'static HeapMirror) -> Self {
        mirror.heap.store(self.charged(), Ordering::Relaxed);
        mirror
            .decoding
            .store(matches!(*lock(&self.shared.phase), Phase::Decode(_)), Ordering::Relaxed);
        self.shared.mirror.get_or_init(|| mirror);
        self
    }

    /// Runs `work` with this ledger as the current thread's ledger for new process
    /// interns: a new `Name`, overflow `Measures` pattern or `PricingContext` that `work`
    /// interns on this thread is charged here until exit, under the process table's lock
    /// and before it is allocated. A refused intern stops the invocation, inserts nothing,
    /// and yields a placeholder that no successful result holds, since every phase checks
    /// [`Self::stopped`] before it returns.
    ///
    /// Scopes nest: the thread's previous ledger, if any, is restored when `work` returns
    /// or unwinds.
    pub fn scope<R>(&self, work: impl FnOnce() -> R) -> R {
        let previous = SCOPED.with(|scoped| scoped.replace(Some(Arc::clone(&self.shared))));
        let _restore = Rescope(previous);
        work()
    }

    /// The per-agent ceiling on request-bearing observations, when there is one.
    pub const fn row_ceiling(&self) -> Option<&ObservationCapacity> {
        self.row_ceiling.as_ref()
    }

    /// The byte budget, or `None` for an unlimited ledger.
    pub fn budget(&self) -> Option<&MemoryBudget> {
        match &self.shared.limit {
            Limit::Unlimited => None,
            Limit::Budget { budget, .. } => Some(budget),
        }
    }

    /// The baseline and headroom the budget check uses.
    pub fn model(&self) -> ProcessModel {
        self.shared.model
    }

    /// Refuses a budget below `F + H × slot_bytes`, the baseline and one decoding worker,
    /// before any work. A budget below `F` alone refuses whatever the slot size.
    pub fn ensure_floor(&self, slot_bytes: u64) -> Result<(), CapacityError> {
        let Limit::Budget { budget, heap } = &self.shared.limit else { return Ok(()) };
        if heap.is_some_and(|heap| heap >= slot_bytes) {
            return Ok(());
        }
        Err(self.shared.refuse(CapacityError::BelowFloor {
            floor: self.shared.model.whole_process(slot_bytes),
            budget: budget.bytes(),
            label: budget.label().to_owned(),
        }))
    }

    /// How many decoding threads to run: `clamp(⌊B / (8 × H × slot_bytes)⌋, 1,
    /// max(8, jobs))`, where `jobs` is an explicit `UROLLUP_JOBS`.
    ///
    /// The count depends on the budget and on an explicit `jobs` above 8, never on the
    /// default worker count, so admission charges the same for one to eight workers. The
    /// default worker count never passes [`SLOT_CEILING`], so passing it as `jobs` is the
    /// same as passing none.
    pub fn worker_slots(&self, slot_bytes: u64, jobs: Option<NonZeroUsize>) -> NonZeroUsize {
        let ceiling = jobs.map_or(SLOT_CEILING, NonZeroUsize::get).max(SLOT_CEILING);
        let slots = match self.budget() {
            None => ceiling,
            Some(budget) => {
                let headroom = self.shared.model.headroom;
                let per_slot = u128::from(slot_bytes)
                    .saturating_mul(8)
                    .saturating_mul(u128::from(headroom.numerator))
                    .max(1);
                let slots = u128::from(budget.bytes())
                    .saturating_mul(u128::from(headroom.denominator))
                    / per_slot;
                usize::try_from(slots).unwrap_or(usize::MAX).clamp(1, ceiling)
            }
        };
        NonZeroUsize::new(slots).unwrap_or(NonZeroUsize::MIN)
    }

    /// Whether any charge or reservation has been refused.
    pub fn stopped(&self) -> bool {
        self.shared.stopped()
    }

    /// The first refusal, once there is one.
    pub fn refusal(&self) -> Option<CapacityError> {
        self.shared.refusal.get().cloned()
    }

    /// Charges `bytes` of `component` to the current phase before they are allocated;
    /// `false` refuses, and every later charge and reservation is refused too.
    #[must_use]
    pub fn charge(&self, component: Component, bytes: u64) -> bool {
        self.shared.charge(component, bytes)
    }

    /// [`Self::charge`], returning the invocation's first refusal when the charge is
    /// refused, for a caller that reports it.
    pub fn try_charge(&self, component: Component, bytes: u64) -> Result<(), CapacityError> {
        if self.charge(component, bytes) {
            return Ok(());
        }
        // A refused charge records its refusal before it stops the invocation, and a
        // charge refused because the invocation stopped sees the one that stopped it.
        Err(self.refusal().expect("a refused charge follows a recorded refusal"))
    }

    /// Charges `bytes` of a new process intern, which stays allocated until the process
    /// exits; checkpoints and commits keep it.
    #[must_use]
    pub fn charge_until_exit(&self, bytes: u64) -> bool {
        self.shared.charge_until_exit(bytes)
    }

    /// Takes a share of the large-record permit for `need` bytes, waiting while another
    /// thread holds the permit; `None` once the invocation is stopped, while waiting or
    /// because the charge is refused.
    ///
    /// The permit is re-entrant: a thread that already holds it adds `need` to what it
    /// holds, as a long line inside a wide zstd frame does. The permit's charge is raised
    /// to the holder's total when that passes every need held at once since the last
    /// checkpoint, charging only the increase to [`Component::Slots`], so the final charge
    /// is the input's largest need in any order. Dropping the share releases its need, and
    /// the permit once its holder holds nothing.
    ///
    /// A waiting thread holds nothing of the permit, so it cannot block the holder; it
    /// returns within 20 ms of a refusal anywhere in the invocation. A share belongs to the
    /// thread that took it, which the type enforces: it cannot move to another thread.
    pub fn large_record(&self, need: u64) -> Option<LargeRecord<'_>> {
        let shared = &*self.shared;
        let me = std::thread::current().id();
        let mut permit = lock(&shared.permit);
        loop {
            if shared.stopped() {
                return None;
            }
            match permit.holder {
                None => {
                    permit.holder = Some(me);
                    break;
                }
                Some(holder) if holder == me => break,
                Some(_) => {
                    permit.waiting = permit.waiting.saturating_add(1);
                    permit = shared
                        .permit_released
                        .wait_timeout(permit, PERMIT_POLL)
                        .unwrap_or_else(PoisonError::into_inner)
                        .0;
                    permit.waiting = permit.waiting.saturating_sub(1);
                }
            }
        }
        if shared.raise_permit(&mut permit, need) {
            permit.shares = permit.shares.saturating_add(1);
            return Some(LargeRecord { admission: self, need, _thread: PhantomData });
        }
        if permit.shares == 0 {
            permit.holder = None;
            drop(permit);
            shared.permit_released.notify_all();
        }
        None
    }

    /// The large-record permit's holder, for tests.
    #[cfg(test)]
    pub(crate) fn permit_holder(&self) -> Option<ThreadId> {
        lock(&self.shared.permit).holder
    }

    /// Whether a thread starts waiting for the large-record permit within `within`, for
    /// tests. It never panics, so a test can still stop the thread it waited for.
    #[cfg(test)]
    pub(crate) fn permit_waiter_within(&self, within: Duration) -> bool {
        let deadline = std::time::Instant::now() + within;
        while lock(&self.shared.permit).waiting == 0 {
            if std::time::Instant::now() >= deadline {
                return false;
            }
            std::thread::yield_now();
        }
        true
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
            self.shared.refuse(CapacityError::Rows {
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
        self.transition(Some(phase), Some(estimate), |next| set_holds(next, holds))
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
        self.transition(None, None, |next| set_holds(next, changes))
    }

    /// Raises `hold` to `bytes` when it holds less, keeping the current phase's charges; a
    /// hold that already covers `bytes` is kept. The coordinating slot grows this way as
    /// discovery finds compressed sources.
    ///
    /// The comparison and the change are one transition, so concurrent raises never lower
    /// a hold. Like [`Self::hold`], call it on the coordinating thread while no worker
    /// charges, and a refusal names the current phase, changes nothing, and is final.
    pub fn raise_hold(&self, hold: Hold, bytes: u64) -> Result<(), CapacityError> {
        // The comparison and the change are one transition under the holds' lock.
        self.transition(None, None, |next| {
            if next.get(&hold).copied().unwrap_or(0) < bytes {
                next.insert(hold, bytes);
            }
        })
    }

    /// The bytes `hold` holds now, or 0 when it is not held.
    pub fn held(&self, hold: Hold) -> u64 {
        lock(&self.shared.holds).get(&hold).copied().unwrap_or(0)
    }

    /// Ends `agent`'s ingest in one transition: its phase charges and discovery metadata
    /// are released, and its retained ledger is held at `retained`, its deep size after
    /// finalize. Charges are then attributed to finalizing that agent's ledger until the
    /// next checkpoint. Call it as [`Self::checkpoint`] is called.
    pub fn commit(&self, agent: Agent, retained: u64) -> Result<(), CapacityError> {
        self.transition(Some(Phase::Finalize(agent)), Some(0), |next| {
            set_holds(next, &[(Hold::Discovery(agent), 0), (Hold::Ledger(agent), retained)]);
        })
    }

    /// The current modeled heap `E`.
    pub fn charged(&self) -> u64 {
        self.shared.charged.load(Ordering::Relaxed)
    }

    /// The bytes charged to `component` so far, over every phase; statistics take the
    /// difference across a phase. Checkpoint estimates and holds are not included: their
    /// callers compute them from the cost model, component by component.
    pub fn charged_by(&self, component: Component) -> u64 {
        self.shared.components[component.index()].load(Ordering::Relaxed)
    }

    /// The largest modeled heap reached so far.
    ///
    /// A successful run's largest gives the threshold budget `F + H × largest` only for
    /// budgets that leave [`Self::worker_slots`] unchanged: the slot count depends on the
    /// budget, and so do the slot charges.
    pub fn largest(&self) -> u64 {
        self.shared.largest.load(Ordering::Relaxed).max(self.charged())
    }

    /// Sets the holds in `changes` and, with an `estimate`, replaces the phase's charges
    /// with it and starts `phase`, all in one move of `E`: `E − released + held`.
    fn transition(
        &self,
        phase: Option<Phase>,
        estimate: Option<u64>,
        edit: impl FnOnce(&mut BTreeMap<Hold, u64>),
    ) -> Result<(), CapacityError> {
        let shared = &*self.shared;
        let mut holds = lock(&shared.holds);
        if let Some(refusal) = self.refusal() {
            return Err(refusal);
        }
        let mut next_holds = holds.clone();
        edit(&mut next_holds);
        let (held_before, held_after) = (sum(&holds), sum(&next_holds));
        let unlimited = shared.limit.is_unlimited();
        let moved = shared.update(|current| {
            let until_exit = || u128::from(shared.until_exit.load(Ordering::Relaxed));
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
                    *lock(&shared.phase) = phase;
                    if let Some(mirror) = shared.mirror.get() {
                        mirror.decoding.store(matches!(phase, Phase::Decode(_)), Ordering::Relaxed);
                    }
                }
                if estimate.is_some() {
                    lock(&shared.permit).level = 0;
                }
                shared.largest.fetch_max(previous.max(next), Ordering::Relaxed);
                Ok(())
            }
            Err(Refused { budget, total }) => {
                let phase = phase.unwrap_or_else(|| *lock(&shared.phase));
                let heap = u64::try_from(total).unwrap_or(u64::MAX);
                let estimate = shared.model.whole_process(heap);
                Err(shared.refuse(memory_error(budget, phase, Some(estimate))))
            }
        }
    }
}

impl Shared {
    fn stopped(&self) -> bool {
        self.stopped.load(Ordering::Acquire)
    }

    fn charge(&self, component: Component, bytes: u64) -> bool {
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

    fn charge_until_exit(&self, bytes: u64) -> bool {
        if !self.charge(Component::Interns, bytes) {
            return false;
        }
        saturating_add(&self.until_exit, bytes);
        true
    }

    /// Adds `more` to what the permit's holder holds and raises the permit's charge to
    /// that total when it passes every earlier one; `false`, changing nothing, when the
    /// charge is refused or the invocation is already stopped.
    fn raise_permit(&self, permit: &mut Permit, more: u64) -> bool {
        let held = permit.held.saturating_add(more);
        if held > permit.level {
            if !self.charge(Component::Slots, held - permit.level) {
                return false;
            }
            permit.level = held;
        } else if self.stopped() {
            return false;
        }
        permit.held = held;
        true
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
                Ok(previous) => {
                    if let Some(mirror) = self.mirror.get() {
                        // The latest `E`, should another update have landed meanwhile.
                        mirror.heap.store(self.charged.load(Ordering::Relaxed), Ordering::Relaxed);
                    }
                    return Ok((previous, admitted));
                }
                Err(actual) => current = actual,
            }
        }
    }

    /// Records `error` unless an earlier refusal is already recorded, stops every
    /// worker, and returns the refusal the invocation reports.
    fn refuse(&self, error: CapacityError) -> CapacityError {
        let first = self.refusal.get_or_init(|| error).clone();
        self.stopped.store(true, Ordering::Release);
        // Without the permit's lock, which the refusing thread may hold: a waiter that
        // misses this signal sees the flag at its next poll.
        self.permit_released.notify_all();
        first
    }
}

/// The most worker slots [`MemoryAdmission::worker_slots`] gives unless an explicit worker
/// count passes it: the eight workers the design charges for, whatever the default count.
pub const SLOT_CEILING: usize = 8;

// Admission treats the default worker count as no explicit count only while the default
// never passes the slot ceiling; raising it would make slot counts depend on the machine.
const _: () = assert!(crate::sources::parallel::MAX_DEFAULT_WORKERS.get() <= SLOT_CEILING);

/// How long a thread waiting for the large-record permit sleeps before it checks the stop
/// flag again, should it miss the signal a refusal sends.
pub(crate) const PERMIT_POLL: Duration = Duration::from_millis(20);

/// The large-record permit's state.
#[derive(Debug, Default)]
struct Permit {
    /// The thread holding the permit.
    holder: Option<ThreadId>,
    /// The shares the holder holds.
    shares: usize,
    /// The sum of their needs.
    held: u64,
    /// The permit's charge: the largest `held` since the last checkpoint.
    level: u64,
    /// The threads waiting for the permit.
    waiting: usize,
}

/// A share of the large-record permit, from [`MemoryAdmission::large_record`]; dropping it
/// releases its need, and the permit once its holder holds no other share.
///
/// Re-entrance is keyed by the thread that took the permit, so a share never moves to
/// another thread: the type is neither `Send` nor `Sync`.
///
/// ```
/// use urollup_core::ledger::admission::MemoryAdmission;
///
/// let admission = MemoryAdmission::unlimited();
/// let share = admission.large_record(1 << 20).expect("an unlimited ledger admits it");
/// assert_eq!(share.need(), 1 << 20);
/// ```
///
/// A share cannot be sent to another thread:
///
/// ```compile_fail,E0277
/// use urollup_core::ledger::admission::LargeRecord;
///
/// fn release_elsewhere(share: LargeRecord<'static>) {
///     std::thread::spawn(move || drop(share));
/// }
/// ```
///
/// nor shared with one:
///
/// ```compile_fail,E0277
/// use urollup_core::ledger::admission::LargeRecord;
///
/// fn shared<T: Sync>() {}
/// shared::<LargeRecord<'static>>();
/// ```
#[derive(Debug)]
#[must_use = "dropping the share releases the large-record permit"]
pub struct LargeRecord<'a> {
    admission: &'a MemoryAdmission,
    need: u64,
    _thread: PhantomData<*const ()>,
}

impl LargeRecord<'_> {
    /// Raises this share's need to `need`, as a line buffer that doubles again does;
    /// `false`, leaving the share unchanged, when the charge is refused or the invocation
    /// is stopped. A smaller need changes nothing.
    #[must_use]
    pub fn raise(&mut self, need: u64) -> bool {
        if need <= self.need {
            return !self.admission.stopped();
        }
        let mut permit = lock(&self.admission.shared.permit);
        debug_assert_eq!(permit.holder, Some(std::thread::current().id()), "a share's holder");
        let raised = self.admission.shared.raise_permit(&mut permit, need - self.need);
        if raised {
            self.need = need;
        }
        raised
    }

    /// The need this share holds.
    pub const fn need(&self) -> u64 {
        self.need
    }
}

impl Drop for LargeRecord<'_> {
    fn drop(&mut self) {
        let mut permit = lock(&self.admission.shared.permit);
        debug_assert_eq!(permit.holder, Some(std::thread::current().id()), "a share's holder");
        permit.held = permit.held.saturating_sub(self.need);
        permit.shares = permit.shares.saturating_sub(1);
        if permit.shares == 0 {
            permit.holder = None;
            permit.held = 0;
            drop(permit);
            self.admission.shared.permit_released.notify_all();
        }
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

/// Sets each hold in `changes`; zero releases one.
fn set_holds(holds: &mut BTreeMap<Hold, u64>, changes: &[(Hold, u64)]) {
    for (hold, bytes) in changes {
        if *bytes == 0 {
            holds.remove(hold);
        } else {
            holds.insert(*hold, *bytes);
        }
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
            Self::Coordinating => f.write_str("coordinating slot"),
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
                            if admission.large_record(*need).is_none()
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
    fn raising_a_hold_never_lowers_it_even_when_raises_race() {
        for _ in 0..20 {
            let admission = ledger(1_000_000);
            std::thread::scope(|scope| {
                for worker in 0..8_u64 {
                    let admission = &admission;
                    scope.spawn(move || {
                        for step in 0..200 {
                            let bytes = (step * 8 + worker) % 1_000 + 1;
                            admission.raise_hold(Hold::Coordinating, bytes).unwrap();
                        }
                    });
                }
            });
            // The largest raise is 1,000 bytes, and no smaller raise replaced it.
            assert_eq!(admission.charged(), 1_000);
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
        for need in [300, 200, 500] {
            drop(admission.large_record(need).unwrap());
        }
        assert_eq!(admission.charged(), 500);
        // A checkpoint ends the decode: the permit's charge is replaced, and the next
        // decode charges its own largest need again.
        admission.checkpoint(Phase::Decode(Agent::Claude), 0, &[]).unwrap();
        drop(admission.large_record(100).unwrap());
        assert_eq!(admission.charged(), 100);
        assert_eq!(admission.charged_by(Component::Slots), 600);
        // A refused share leaves the permit free and the charge unchanged.
        assert!(admission.large_record(1001).is_none());
        assert_eq!(admission.permit_holder(), None);
        assert_eq!(admission.charged(), 100);
    }

    #[test]
    fn the_permit_is_re_entrant_and_its_need_is_the_sum_its_holder_holds() {
        let admission = ledger(10_000);
        let me = std::thread::current().id();
        // A wide frame, then a long line inside it whose buffer doubles twice.
        let frame = admission.large_record(1000).unwrap();
        let mut line = admission.large_record(400).unwrap();
        assert!(line.raise(800));
        assert_eq!(line.need(), 800);
        assert!(line.raise(100), "a smaller need changes nothing");
        assert_eq!(admission.charged(), 1800);
        assert_eq!(admission.permit_holder(), Some(me));
        // The line's buffer shrinks; the frame still holds the permit.
        drop(line);
        assert_eq!(admission.permit_holder(), Some(me));
        let again = admission.large_record(300).unwrap();
        assert_eq!(admission.charged(), 1800, "1300 held at once never passes 1800");
        drop(again);
        drop(frame);
        assert_eq!(admission.permit_holder(), None);
        // A refused raise leaves the share's need, and the charge, unchanged.
        let mut share = admission.large_record(100).unwrap();
        assert!(!share.raise(20_000));
        assert_eq!(share.need(), 100);
        assert!(admission.stopped());
        assert!(!share.raise(50), "a stopped invocation refuses even a smaller need");
    }

    #[test]
    fn one_thread_at_a_time_holds_the_permit() {
        let admission = MemoryAdmission::unlimited();
        let holding = AtomicUsize::new(0);
        let most = AtomicUsize::new(0);
        std::thread::scope(|scope| {
            for worker in 0..8_u64 {
                let (admission, holding, most) = (&admission, &holding, &most);
                scope.spawn(move || {
                    for round in 0..50 {
                        let outer = admission.large_record(worker * 10 + round).unwrap();
                        let now = holding.fetch_add(1, Ordering::SeqCst) + 1;
                        most.fetch_max(now, Ordering::SeqCst);
                        let inner = admission.large_record(1).unwrap();
                        std::thread::yield_now();
                        holding.fetch_sub(1, Ordering::SeqCst);
                        drop((inner, outer));
                    }
                });
            }
        });
        assert_eq!(most.load(Ordering::SeqCst), 1);
        assert_eq!(admission.permit_holder(), None);
        // The charge is the largest single holder's total: 70 + 49 + 1.
        assert_eq!(admission.charged_by(Component::Slots), 120);
    }

    #[test]
    fn a_thread_waiting_for_the_permit_returns_when_a_refusal_stops_the_invocation() {
        let admission = ledger(1000);
        let held = admission.large_record(10).unwrap();
        let (sender, receiver) = std::sync::mpsc::channel();
        std::thread::scope(|scope| {
            scope.spawn(|| sender.send(admission.large_record(5).is_none()).unwrap());
            // Once the other thread waits on the held permit, refuse elsewhere. The refusal
            // comes whatever happens, so the thread always stops and the scope ends.
            let waited = admission.permit_waiter_within(Duration::from_secs(60));
            assert!(!admission.charge(Component::Records, 2000));
            let stopped = receiver.recv_timeout(Duration::from_secs(60));
            assert!(waited, "the other thread waits for the held permit");
            assert_eq!(stopped, Ok(true), "the waiter returns, with no share once stopped");
        });
        drop(held);
        assert_eq!(admission.permit_holder(), None);
        assert!(admission.large_record(1).is_none());
    }

    #[test]
    fn charges_are_totaled_by_component_across_phases() {
        let admission = ledger(1000);
        assert!(admission.charge(Component::Records, 30));
        assert!(admission.charge(Component::Kappa, 20));
        admission.checkpoint(Phase::Construction(Agent::Codex), 10, &[]).unwrap();
        assert!(admission.charge(Component::Records, 5));
        assert!(admission.charge_until_exit(7));
        drop(admission.large_record(11).unwrap());
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
        drop(admission.large_record(u64::MAX).unwrap());
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

    /// A text no other test interns: tests in one process share the intern tables.
    fn unique(label: &str) -> String {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        format!("admission-test-{label}-{}", NEXT.fetch_add(1, Ordering::Relaxed))
    }

    /// The charge for interning `text` as a new name: its entry, and the table's first node
    /// when no earlier test interned a name.
    fn name_charge(admission: &MemoryAdmission, text: &str) -> u64 {
        let charged = admission.charged_by(Component::Interns);
        let entry = model::name_intern(text.len() as u64);
        assert!(
            charged == entry || charged == entry + model::NAME_TABLE_BASE,
            "{charged} charged for a name entry of {entry}"
        );
        charged
    }

    #[test]
    fn a_new_intern_on_a_scoped_thread_is_charged_until_exit() {
        use crate::ledger::names::Name;
        let admission = ledger(1 << 20);
        let text = unique("scoped");
        let name = admission.scope(|| Name::new(&text));
        assert_eq!(name.as_str(), text);
        let charge = name_charge(&admission, &text);
        assert_eq!(admission.charged(), charge);
        // An existing text is not charged again, and charges until exit survive a
        // checkpoint.
        admission.scope(|| Name::new(&text));
        admission.checkpoint(Phase::Construction(Agent::Codex), 10, &[]).unwrap();
        assert_eq!(admission.charged(), charge + 10);
        // Outside a scope nothing is charged.
        Name::new(&unique("unscoped"));
        assert_eq!(admission.charged(), charge + 10);
    }

    #[test]
    fn a_refused_intern_stores_nothing_and_stops_the_invocation() {
        use crate::ledger::entities::{Pricing, PricingContext};
        use crate::ledger::names::Name;
        use crate::ledger::tokens::{Measures, TokenMeasures};
        let text = unique("refused");
        let admission = ledger(1);
        let placeholder = admission.scope(|| Name::new(&text));
        assert!(admission.stopped());
        assert_eq!(placeholder.as_str(), "");
        assert_eq!(admission.charged(), 0);
        assert!(matches!(admission.refusal(), Some(CapacityError::Memory { estimate: None, .. })));
        // Nothing was stored or cached: the text interns normally afterwards.
        let name = Name::new(&text);
        assert_eq!(name.as_str(), text);
        assert_ne!(name, placeholder);
        // Overflow patterns and pricing contexts refuse the same way.
        let large =
            TokenMeasures { output: Some(u64::from(u32::MAX) + 7), ..TokenMeasures::default() };
        let context = PricingContext { provider: Some(name), ..PricingContext::default() };
        let (measures, pricing) =
            admission.scope(|| (Measures::from(large), Pricing::new(context.clone())));
        assert_eq!(measures, Measures::default());
        assert_eq!(*pricing.context(), PricingContext::default());
        assert_eq!(TokenMeasures::from(Measures::from(large)), large);
        assert_eq!(*Pricing::new(context.clone()).context(), context);
        assert_eq!(admission.charged(), 0);
    }

    #[test]
    fn a_text_two_scoped_threads_intern_at_once_is_charged_once() {
        use crate::ledger::names::Name;
        for _ in 0..20 {
            let admission = ledger(1 << 20);
            let text = unique("shared");
            std::thread::scope(|scope| {
                for _ in 0..8 {
                    scope.spawn(|| admission.scope(|| Name::new(&text)));
                }
            });
            name_charge(&admission, &text);
        }
    }

    #[test]
    fn scopes_nest_and_restore_the_previous_ledger() {
        use crate::ledger::names::Name;
        let (outer, inner) = (ledger(1 << 20), ledger(1 << 20));
        outer.scope(|| {
            let text = unique("inner");
            inner.scope(|| Name::new(&text));
            name_charge(&inner, &text);
            assert_eq!(outer.charged(), 0);
            let text = unique("outer");
            Name::new(&text);
            name_charge(&outer, &text);
        });
        let charged = outer.charged();
        Name::new(&unique("after"));
        assert_eq!(outer.charged(), charged);
        // An unwinding scope restores too.
        let unwound = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            inner.scope(|| std::panic::resume_unwind(Box::new("the scoped work unwinds")));
        }));
        assert!(unwound.is_err());
        let charged = inner.charged();
        Name::new(&unique("after unwind"));
        assert_eq!(inner.charged(), charged);
    }

    #[test]
    fn a_heap_mirror_follows_every_change_of_e() {
        static MIRROR: HeapMirror = HeapMirror::new();
        let admission = ledger(1000).with_heap_mirror(&MIRROR);
        assert_eq!((MIRROR.heap(), MIRROR.decoding()), (0, false));
        assert!(admission.charge(Component::Records, 30));
        assert_eq!(MIRROR.heap(), 30);
        admission.hold(&[(Hold::SessionIndex, 5)]).unwrap();
        assert_eq!(MIRROR.heap(), 35);
        admission.checkpoint(Phase::Decode(Agent::Codex), 4, &[]).unwrap();
        assert_eq!((MIRROR.heap(), MIRROR.decoding()), (9, true));
        admission.checkpoint(Phase::Query, 2, &[]).unwrap();
        assert_eq!((MIRROR.heap(), MIRROR.decoding()), (7, false));
        assert_eq!(admission.held(Hold::SessionIndex), 5);
        assert_eq!(admission.held(Hold::Coordinating), 0);
    }
}
