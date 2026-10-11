//! The admission model-bound harness (scalable-ingestion plan, "Admission Tests"): a
//! counting global allocator measures live Rust heap and compares it with the cost model
//! of `ledger::admission::model`.
//!
//! Admission is not wired into ingestion yet, so this binary checks every term it can
//! measure from outside ingestion, each against the structure it models:
//!
//! - **Primitives:** vectors, hash maps (inserted, presized and churned), B-trees
//!   (inserted and collected), stable sorts and `Arc` text, at every count up to a few
//!   thousand, growing one structure and checking the running peak, so each check is O(N).
//! - **Structures:** diagnostic compaction, the structures a conflicting key leaves, an
//!   ambiguous request's owners, the request sort, a key derivation, the name table's
//!   first node, a thread's name cache, and cold name and overflow interning.
//! - **Reconciliation:** tight synthetic inputs, one per term, measured between the
//!   [`ReconcileStage`]s where slice 5's checkpoints run, and the whole run against the
//!   grouping and finalize estimates, with live heap tracked against that `E` at every
//!   allocation.
//! - **Retained state:** each ingest's ledger, discovery and the session index against
//!   their deep sizes, on every fixture and on generated corpora at one and eight workers.
//! - **Query and slots:** query documents and their rendering, each decoder's heap, a long
//!   line's scan, and zstd's context and stream.
//!
//! Each check names its terms. A check fails when measured heap exceeds their sum, and the
//! harness fails unless every term is necessary somewhere: dropping it would make some
//! check fail. It also reports, without failing, each ingest's peak beside the forward
//! estimate it can compute after the fact; that bound fails once slices 5 and 6 charge
//! decode.
//!
//! It runs without the libtest harness, single-threaded apart from the workers it starts,
//! so no other test's allocations reach the counters; it honors `--list`, name filters,
//! `--exact`, `--skip` and `--ignored`. Its `unsafe` is the allocator below and stays in
//! this binary.

use std::alloc::{GlobalAlloc, Layout, System};
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::fmt::Write as _;
use std::fs;
use std::io::{BufRead as _, Write as _};
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::json;
use urollup_core::adapters::{Ingested, claude_project, codex_rollout};
use urollup_core::ledger::admission::deep_size::DeepSize;
use urollup_core::ledger::admission::model::{
    self, ReconcileCounts, allocation, btree_collected, btree_entry, btree_map, btree_node,
    grown_vec, hash_base, hash_churned, hash_entry, hash_table, hash_with_capacity, sizes,
    sort_scratch,
};
use urollup_core::ledger::capacity::ObservationCapacity;
use urollup_core::ledger::diagnostics::{Diagnostic, DiagnosticCode, compact};
use urollup_core::ledger::entities::{
    Basis, Counting as RequestCounting, ModelBasis, ModelName, ModelUsage, Ownership,
    ProviderLimitObservation, RecordRefs, Request, Requests, Thread,
};
use urollup_core::ledger::identity::{
    AnalyticalId, IdPrefix, IdentityKey, KeyComponent, StoredIdentity,
};
use urollup_core::ledger::linking::LinkGraph;
use urollup_core::ledger::names::{self, Name};
use urollup_core::ledger::reconcile::{
    LatestRevision, Ledger, ObservationRole, OwnerEvidence, ReconcileInput, ReconcileStage,
    RequestObservation, reconcile_with_stages,
};
use urollup_core::ledger::scope::{
    ComponentRole, ComponentSlot, DerivedKey, IdScope, IdentityBasis, KeySpec, artifact_local_key,
};
use urollup_core::ledger::tokens::{self, Measures, TokenMeasures};
use urollup_core::query::{self, QueryMetadata, QuerySource, ResolvedTimeZone};
use urollup_core::selection::{Agent, Scope, SessionIndex};
use urollup_core::sources::evidence::{EvidenceRef, SourceTable};
use urollup_core::sources::manifest::Representation;
use urollup_core::sources::reader::{
    LogicalSource, ReadOptions, RecordDisposition, SourceSpec, decode, read_source,
};
use urollup_core::sources::roots::discover;

/// Live heap by the costing rule, and its peak since the last reset.
static LIVE: AtomicU64 = AtomicU64::new(0);
static PEAK: AtomicU64 = AtomicU64::new(0);
/// Live heap as requested, without the costing rule, for the report.
static REQUESTED: AtomicU64 = AtomicU64::new(0);
/// The current `E` as a live-heap level, and the most live heap has passed it at any
/// allocation since the last reset.
static LIMIT: AtomicU64 = AtomicU64::new(u64::MAX);
static EXCESS: AtomicU64 = AtomicU64::new(0);

/// The costing rule's charge for one allocation of `size` bytes.
fn cost(size: usize) -> u64 {
    allocation(size as u64)
}

fn grow(size: usize) {
    REQUESTED.fetch_add(size as u64, Ordering::Relaxed);
    let live = LIVE.fetch_add(cost(size), Ordering::Relaxed) + cost(size);
    PEAK.fetch_max(live, Ordering::Relaxed);
    let limit = LIMIT.load(Ordering::Relaxed);
    if live > limit {
        EXCESS.fetch_max(live - limit, Ordering::Relaxed);
    }
}

fn shrink(size: usize) {
    REQUESTED.fetch_sub(size as u64, Ordering::Relaxed);
    LIVE.fetch_sub(cost(size), Ordering::Relaxed);
}

/// The system allocator, counting every live allocation.
struct Counting;

// SAFETY: every method forwards its arguments unchanged to `System`, which upholds the
// `GlobalAlloc` contract; the counters are atomics that never touch the allocation.
#[expect(unsafe_code, reason = "a counting allocator for the model-bound harness")]
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: the caller's layout contract passes through unchanged.
        let pointer = unsafe { System.alloc(layout) };
        if !pointer.is_null() {
            grow(layout.size());
        }
        pointer
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        // SAFETY: the caller's layout contract passes through unchanged.
        let pointer = unsafe { System.alloc_zeroed(layout) };
        if !pointer.is_null() {
            grow(layout.size());
        }
        pointer
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        // SAFETY: `pointer` was allocated by this allocator, so by `System`, with `layout`.
        unsafe { System.dealloc(pointer, layout) };
        shrink(layout.size());
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // Every reallocation, moved or not, counts the new block before the old one is
        // released, as the doubling rule charges it.
        grow(new_size);
        // SAFETY: the caller's contract for `pointer`, `layout` and `new_size` passes
        // through unchanged.
        let moved = unsafe { System.realloc(pointer, layout, new_size) };
        if moved.is_null() {
            shrink(new_size);
        } else {
            shrink(layout.size());
        }
        moved
    }
}

#[global_allocator]
static ALLOCATOR: Counting = Counting;

fn live() -> u64 {
    LIVE.load(Ordering::Relaxed)
}

/// Heap measured around one operation, above the live heap before it started.
#[derive(Clone, Copy, Debug)]
struct Measured {
    /// The peak while it ran, by the costing rule.
    peak: u64,
    /// What it left allocated, by the costing rule.
    retained: u64,
    /// What it left allocated, as requested.
    retained_requested: u64,
}

fn measure<T>(operation: impl FnOnce() -> T) -> (T, Measured) {
    let start = live();
    let start_requested = REQUESTED.load(Ordering::Relaxed);
    PEAK.store(start, Ordering::Relaxed);
    let value = operation();
    let measured = Measured {
        peak: PEAK.load(Ordering::Relaxed).saturating_sub(start),
        retained: live().saturating_sub(start),
        retained_requested: REQUESTED.load(Ordering::Relaxed).saturating_sub(start_requested),
    };
    (value, measured)
}

/// A named model term and its bytes in one check.
type Term = (&'static str, u64);

/// Comparisons of measured heap with the model, the failures among them, and whether
/// each term is necessary somewhere.
#[derive(Default)]
struct Report {
    lines: Vec<String>,
    violations: Vec<String>,
    necessary: BTreeMap<&'static str, bool>,
}

impl Report {
    /// Records a bound: `measured` must not exceed the sum of `terms`. A term is
    /// necessary in this check when `measured` exceeds the sum without it.
    fn bound(&mut self, check: &str, case: &str, measured: u64, terms: &[Term]) {
        self.bound_beside(check, case, measured, terms, 0);
    }

    /// Records a bound on an aggregate, such as a deep size or a phase total, whose parts
    /// are model terms checked by other checks, so it names no term of its own.
    fn bound_total(&mut self, check: &str, case: &str, measured: u64, estimate: u64) {
        self.bound_beside(check, case, measured, &[], estimate);
    }

    /// [`Report::bound`], with `beside` bytes that are not a model term, such as an exact
    /// deep size, held beside the terms.
    fn bound_beside(
        &mut self,
        check: &str,
        case: &str,
        measured: u64,
        terms: &[Term],
        beside: u64,
    ) {
        let total = terms.iter().fold(beside, |total, (_, bytes)| total.saturating_add(*bytes));
        let verdict = if measured <= total { "ok" } else { "VIOLATION" };
        let necessary: Vec<&str> = terms
            .iter()
            .filter(|(_, bytes)| measured > total.saturating_sub(*bytes))
            .map(|(name, _)| *name)
            .collect();
        for (name, _) in terms {
            let entry = self.necessary.entry(name).or_insert(false);
            *entry |= necessary.contains(name);
        }
        self.lines.push(format!(
            "{verdict:9} {check:20} {case:40} measured {measured:>11} estimate {total:>11} ({}) needs {}",
            ratio(measured, total),
            if necessary.is_empty() { "-".to_owned() } else { necessary.join(" ") },
        ));
        if measured > total {
            self.violations.push(format!("{check} {case}: {measured} > {total}"));
        }
    }

    /// Records a measurement the model does not bound yet.
    fn note(&mut self, check: &str, case: &str, text: impl AsRef<str>) {
        self.lines.push(format!("{:9} {check:20} {case:40} {}", "report", text.as_ref()));
    }

    /// Fails each term no check needs: dropping it would go undetected.
    fn require_necessary_terms(&mut self) {
        let unneeded: Vec<&str> =
            self.necessary.iter().filter(|(_, needed)| !**needed).map(|(name, _)| *name).collect();
        for name in unneeded {
            self.violations.push(format!("dropping the {name} term fails no check"));
        }
    }
}

fn ratio(measured: u64, estimate: u64) -> String {
    if estimate == 0 {
        return "no estimate".to_owned();
    }
    let percent = u128::from(measured) * 100 / u128::from(estimate);
    format!("{percent}%")
}

/// Whether the arguments cargo passes select this binary's one test, `memory_model`, as
/// libtest would; `--list` prints it and runs nothing.
fn selected(arguments: &[String]) -> bool {
    const NAME: &str = "memory_model";
    let mut filters = Vec::new();
    let mut skips = Vec::new();
    let (mut exact, mut list, mut ignored) = (false, false, false);
    let mut arguments = arguments.iter();
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--list" => list = true,
            "--exact" => exact = true,
            "--ignored" => ignored = true,
            "--skip" => skips.extend(arguments.next()),
            "--test-threads" | "--format" | "--color" | "--logfile" | "-Z" => {
                arguments.next();
            }
            flag if flag.starts_with('-') => {}
            filter => filters.push(filter),
        }
    }
    let matches = |filter: &str| if exact { filter == NAME } else { NAME.contains(filter) };
    let chosen = !ignored
        && (filters.is_empty() || filters.iter().any(|filter| matches(filter)))
        && !skips.iter().any(|skip| matches(skip));
    if list {
        if chosen {
            println!("{NAME}: test");
        }
        return false;
    }
    chosen
}

fn fixtures(dialect: &str) -> Vec<PathBuf> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(dialect);
    let mut cases: Vec<PathBuf> = fs::read_dir(root)
        .expect("the fixture directory is readable")
        .map(|entry| entry.expect("the fixture entry is readable").path())
        .filter(|path| path.is_dir())
        .collect();
    cases.sort();
    cases
}

fn workers(count: usize) -> NonZeroUsize {
    NonZeroUsize::new(count).expect("worker counts are positive")
}

fn roots(agent: Agent, root: &Path) -> Vec<PathBuf> {
    match agent {
        Agent::Codex => codex_rollout::rollout_roots(&[root.to_owned()]),
        Agent::Claude | Agent::Pi => vec![root.to_owned()],
    }
}

fn ingest(agent: Agent, root: &Path, count: usize) -> Ingested {
    let discovery = discover(&roots(agent, root));
    match agent {
        Agent::Claude => {
            claude_project::ingest_discovery_with_workers(discovery, true, workers(count))
        }
        Agent::Codex => {
            codex_rollout::ingest_discovery_with_workers(discovery, true, workers(count))
        }
        Agent::Pi => unreachable!("Pi has no adapter"),
    }
    .expect("the corpus ingests")
}

/// The model's charge for every process intern so far: names and overflow patterns.
fn interns() -> u64 {
    names::interned_bytes() + tokens::overflow_interned_bytes()
}

/// Interns names and overflow patterns that no earlier code interned, so the table bases
/// and this thread's name cache are charged where they are allocated. It must run first.
fn check_cold_interns(report: &mut Report) {
    let texts: Vec<String> = (0..1_000).map(|index| format!("cold-name-{index:05}")).collect();
    let charged_before = names::interned_bytes();
    let ((), first) = measure(|| {
        Name::new(&texts[0]);
    });
    let first_charge = names::interned_bytes() - charged_before;
    report.bound(
        "intern cold",
        "first name",
        first.retained,
        &[("name_intern", first_charge), ("NAME_CACHE", model::NAME_CACHE)],
    );
    let charged_before = names::interned_bytes();
    let ((), rest) = measure(|| {
        for text in &texts[1..] {
            Name::new(text);
        }
    });
    report.bound(
        "intern cold",
        "999 more names",
        rest.retained,
        &[("name_intern", names::interned_bytes() - charged_before)],
    );
    let overflow = |index: u64| TokenMeasures {
        uncached_input: Some(u64::from(u32::MAX) + 1 + index),
        ..TokenMeasures::default()
    };
    let charged_before = tokens::overflow_interned_bytes();
    let ((), first) = measure(|| {
        // Converting a counter past `u32::MAX` interns its pattern.
        let _interned = Measures::from(overflow(0));
    });
    report.bound(
        "intern cold",
        "first overflow pattern",
        first.retained,
        &[
            ("overflow_intern", model::overflow_intern()),
            (
                "OVERFLOW_TABLE_BASE",
                tokens::overflow_interned_bytes() - charged_before - model::overflow_intern(),
            ),
        ],
    );
    let charged_before = tokens::overflow_interned_bytes();
    let ((), rest) = measure(|| {
        for index in 1..1_000 {
            let _interned = Measures::from(overflow(index));
        }
    });
    report.bound(
        "intern cold",
        "999 more patterns",
        rest.retained,
        &[("overflow_intern", tokens::overflow_interned_bytes() - charged_before)],
    );
}

/// The name table's first node, and a fresh thread's name cache.
fn check_name_structures(report: &mut Report) {
    let name = Name::new("cold-name-00000");
    let (map, measured) = measure(|| {
        let mut map: BTreeMap<&'static str, Name> = BTreeMap::new();
        map.insert("text", name);
        map
    });
    drop(map);
    report.bound(
        "name table",
        "first node",
        measured.peak,
        &[("NAME_TABLE_BASE", model::NAME_TABLE_BASE)],
    );
    // Inside a fresh thread, interning an existing name allocates only the thread's cache
    // and its destructor's registration.
    let cache = std::thread::spawn(move || {
        let before = live();
        Name::new("cold-name-00001");
        live().saturating_sub(before)
    })
    .join()
    .expect("the thread runs");
    report.bound("name cache", "fresh thread", cache, &[("NAME_CACHE", model::NAME_CACHE)]);
}

/// The running peak above `start`, after each step of a growing structure.
fn running_peak(start: u64) -> u64 {
    PEAK.load(Ordering::Relaxed).saturating_sub(start)
}

/// A distinct `S`-byte value for `index`.
fn bytes<const S: usize>(index: usize) -> [u8; S] {
    let mut value = [0_u8; S];
    for (slot, byte) in value.iter_mut().zip(index.to_le_bytes()) {
        *slot = byte;
    }
    value
}

/// The largest of each check's measured over estimate, across every count, as one line.
struct Worst {
    measured: u64,
    estimate: u64,
}

impl Worst {
    fn new() -> Self {
        Self { measured: 0, estimate: 1 }
    }

    /// Keeps the count whose measured bytes come closest to, or pass, the estimate.
    fn keep(&mut self, measured: u64, estimate: u64) {
        if u128::from(measured) * u128::from(self.estimate)
            > u128::from(self.measured) * u128::from(estimate)
        {
            *self = Self { measured, estimate };
        }
    }
}

fn check_vec<const S: usize>(report: &mut Report, count: usize) {
    let mut grown = Worst::new();
    let start = live();
    PEAK.store(start, Ordering::Relaxed);
    let mut items: Vec<[u8; S]> = Vec::new();
    for length in 1..=count {
        items.push(bytes::<S>(length));
        grown.keep(running_peak(start), grown_vec(length as u64, S as u64));
    }
    drop(items);
    let mut shrunk = Worst::new();
    for length in (1..=count).filter(|length| length.is_power_of_two() || length % 97 == 0) {
        let start = live();
        PEAK.store(start, Ordering::Relaxed);
        let mut items: Vec<[u8; S]> = Vec::new();
        for index in 0..=length {
            items.push(bytes::<S>(index));
        }
        items.pop();
        items.shrink_to_fit();
        shrunk.keep(running_peak(start), grown_vec(length as u64, S as u64));
    }
    let case = format!("{S} B × 1..{count}");
    report.bound("vec push", &case, grown.measured, &[("grown_vec", grown.estimate)]);
    report.bound("vec shrink", &case, shrunk.measured, &[("grown_vec", shrunk.estimate)]);
}

fn check_hash<const S: usize>(report: &mut Report, count: usize) {
    let entry = S as u64;
    let mut table = Worst::new();
    let mut rule = Worst::new();
    let start = live();
    PEAK.store(start, Ordering::Relaxed);
    let mut set: HashSet<[u8; S]> = HashSet::new();
    for index in 0..count {
        if !set.insert(bytes::<S>(index)) {
            break;
        }
        let entries = set.len() as u64;
        table.keep(running_peak(start), hash_table(entries, entry));
        rule.keep(running_peak(start), entries * hash_entry(entry) + hash_base(entry));
    }
    drop(set);
    let case = format!("{S} B × 1..{count}");
    report.bound("hash insert", &case, table.measured, &[("hash_table", table.estimate)]);
    report.bound("hash rule", &case, rule.measured, &[("hash_entry+hash_base", rule.estimate)]);
    let capacity = count.min(1 << (8 * S.min(3)));
    let (set, presized) = measure(|| HashSet::<[u8; S]>::with_capacity(capacity));
    drop(set);
    report.bound(
        "hash presized",
        &format!("{S} B, capacity {capacity}"),
        presized.peak,
        &[("hash_with_capacity", hash_with_capacity(capacity as u64, 0, entry))],
    );
}

fn check_hash_churn<const S: usize>(report: &mut Report, live_entries: usize, churn: usize) {
    let ((), measured) = measure(|| {
        let mut set: HashSet<[u8; S]> = (0..live_entries).map(bytes::<S>).collect();
        for index in 0..churn {
            set.remove(&bytes::<S>(index));
            set.insert(bytes::<S>(live_entries + index));
        }
    });
    report.bound(
        "hash churn",
        &format!("{S} B, {live_entries} live, {churn} churned"),
        measured.peak,
        &[("hash_churned", hash_churned(live_entries as u64, S as u64))],
    );
}

/// A deterministic shuffle of `0..count`.
fn shuffled(count: usize) -> Vec<usize> {
    let mut order: Vec<usize> = (0..count).collect();
    let mut state = 0x9e37_79b9_7f4a_7c15_u64;
    for index in (1..count).rev() {
        state =
            state.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        let other = usize::try_from(state >> 33).expect("fits") % (index + 1);
        order.swap(index, other);
    }
    order
}

fn check_btree<const K: usize>(report: &mut Report, count: usize) {
    let (key, value) = (K as u64, size_of::<u64>() as u64);
    let order = shuffled(count);
    let mut inserted = Worst::new();
    let mut rule = Worst::new();
    let start = live();
    PEAK.store(start, Ordering::Relaxed);
    let mut map: BTreeMap<[u8; K], u64> = BTreeMap::new();
    for (entries, index) in order.iter().enumerate() {
        map.insert(bytes::<K>(*index), 0);
        let entries = entries as u64 + 1;
        inserted.keep(running_peak(start), btree_map(entries, key, value));
        rule.keep(running_peak(start), entries * btree_entry(key, value) + btree_node(key, value));
    }
    drop(map);
    let case = format!("{K} B keys × 1..{count}");
    report.bound("btree insert", &case, inserted.measured, &[("btree_map", inserted.estimate)]);
    report.bound("btree rule", &case, rule.measured, &[("btree_entry", rule.estimate)]);
    let mut collected = Worst::new();
    for entries in (1..=count).filter(|entries| *entries < 64 || entries.is_power_of_two()) {
        let keys: Vec<[u8; K]> = shuffled(entries).into_iter().map(bytes::<K>).collect();
        let (set, measured) =
            measure(|| keys.iter().copied().filter(|_| true).collect::<BTreeSet<[u8; K]>>());
        drop(set);
        collected.keep(measured.peak, btree_collected(entries as u64, key, 0));
    }
    report.bound(
        "btree collect",
        &case,
        collected.measured,
        &[("btree_collected", collected.estimate)],
    );
}

#[expect(clippy::stable_sort_primitive, reason = "the check measures the stable sort's scratch")]
fn check_sort<const S: usize>(report: &mut Report, count: usize) {
    let mut sorted = Worst::new();
    for length in
        (2..=count).filter(|length| *length < 100 || length.is_power_of_two() || length % 1000 == 0)
    {
        let mut items: Vec<[u8; S]> = shuffled(length).into_iter().map(bytes::<S>).collect();
        let ((), measured) = measure(|| items.sort());
        sorted.keep(measured.peak, sort_scratch(length as u64, S as u64));
    }
    report.bound(
        "stable sort",
        &format!("{S} B × 2..{count}"),
        sorted.measured,
        &[("sort_scratch", sorted.estimate)],
    );
}

fn check_primitives(report: &mut Report) {
    check_vec::<1>(report, 3_000);
    check_vec::<8>(report, 3_000);
    check_vec::<16>(report, 3_000);
    check_vec::<24>(report, 3_000);
    check_vec::<216>(report, 3_000);
    check_vec::<1_024>(report, 1_000);
    check_vec::<1_025>(report, 1_000);
    check_hash::<1>(report, 256);
    check_hash::<2>(report, 3_000);
    check_hash::<4>(report, 3_000);
    check_hash::<16>(report, 3_000);
    check_hash::<28>(report, 3_000);
    check_hash::<80>(report, 3_000);
    check_hash_churn::<16>(report, 1_000, 20_000);
    check_hash_churn::<80>(report, 1_000, 20_000);
    check_btree::<16>(report, 2_000);
    check_btree::<17>(report, 2_000);
    check_btree::<24>(report, 2_000);
    check_sort::<16>(report, 8_000);
    check_sort::<80>(report, 8_000);
    check_sort::<400>(report, 4_000);
    let text = "x".repeat(1_000);
    let (arc, measured) = measure(|| Arc::<str>::from(text.as_str()));
    drop(arc);
    report.bound("arc text", "1,000 bytes", measured.peak, &[("arc", model::arc(1_000))]);
}

fn id(prefix: IdPrefix, kind: &str, index: usize) -> AnalyticalId {
    IdentityKey::new(prefix, kind, vec![KeyComponent::Integer(i64::try_from(index).expect("fits"))])
        .derive_id()
        .expect("an ID derives")
}

/// Composite terms, each against the real structure it models.
fn check_structures(report: &mut Report) {
    // Diagnostic compaction: distinct subjects, so nothing merges, from a vector of twice
    // the rows, its worst case.
    let count = 1_025;
    let subjects: Vec<AnalyticalId> =
        (0..count).map(|index| id(IdPrefix::Request, "diag", index)).collect();
    let (charge, measured) = measure(|| {
        let mut diagnostics = Vec::with_capacity(2 * count);
        let mut charge = 0;
        for (index, subject) in subjects.iter().enumerate() {
            let detail = format!("observations sharing a key disagree on {index}");
            let evidence =
                (0..4).map(|offset| EvidenceRef::new(0, offset * 100 + index as u64, 10));
            let diagnostic = Diagnostic::new(
                DiagnosticCode::ConflictingSharedKey,
                Some(subject.clone()),
                evidence,
                detail,
            );
            charge += model::diagnostic(
                diagnostic.detail.capacity() as u64,
                diagnostic.evidence.len() as u64,
            );
            diagnostics.push(diagnostic);
        }
        drop(compact(diagnostics));
        charge
    });
    report.bound("diagnostics", "1,025 compacted", measured.peak, &[("diagnostic", charge)]);

    // A conflicting key's candidate graph, candidate sets and components, for 500 keys of
    // 2 to 4 parts.
    let (parts, measured) = measure(|| {
        let mut graph = LinkGraph::new();
        let mut parts = 0;
        let mut charge = 0;
        for key in 0..500 {
            let width = 2 + key % 3;
            let ids: Vec<AnalyticalId> =
                (0..width).map(|part| id(IdPrefix::Request, "part", key * 8 + part)).collect();
            for pair in ids.windows(2) {
                graph.link(&pair[0], &pair[1]);
            }
            parts += width;
            charge += model::conflicting_key(width as u64);
        }
        let sets: Vec<BTreeSet<AnalyticalId>> = graph.components().into_values().collect();
        drop(sets);
        drop(graph);
        (parts, charge)
    });
    report.bound(
        "conflicting keys",
        &format!("500 keys, {} parts", parts.0),
        measured.peak,
        &[("conflicting_key", parts.1)],
    );

    // An ambiguous request's candidate owners.
    let owners: Vec<AnalyticalId> =
        (0..64).map(|index| id(IdPrefix::Thread, "owner", index)).collect();
    let ((), inserted) = measure(|| {
        let mut set = BTreeSet::new();
        for owner in &owners {
            set.insert(owner.clone());
        }
    });
    report.bound(
        "ambiguous owners",
        "64 candidates",
        inserted.peak,
        &[("ambiguous_owners", model::ambiguous_owners(64))],
    );

    // The request sort: requests as reconcile presizes them, in reverse ID order.
    let count = 8_000;
    let mut rows = Vec::with_capacity(count + count / 32);
    for index in (0..count).rev() {
        rows.push(Request {
            id: id(IdPrefix::Request, "sort", index),
            basis: IdentityBasis::Native,
            aliases: Box::new([]),
            ownership: Ownership::Unknown,
            first_seen: None,
            last_seen: None,
            model: None,
            effort: None,
            usage: None,
            records: RecordRefs::Empty,
            originals: 0,
            counting: RequestCounting::Counted,
        });
    }
    let mut spare = Vec::with_capacity(2 * count);
    spare.extend(rows.iter().cloned());
    let (requests, measured) = measure(|| Requests::from_unsorted(rows));
    drop(requests);
    report.bound(
        "request sort",
        "8,000 requests, presized",
        measured.peak,
        &[("request_sort", model::request_sort(count as u64))],
    );
    // With more than 1/16 spare capacity, the sort also shrinks the vector.
    let (requests, measured) = measure(|| Requests::from_unsorted(spare));
    drop(requests);
    report.bound(
        "request sort",
        "8,000 requests, shrunk",
        measured.peak,
        &[("request_sort", model::request_sort(count as u64))],
    );

    // One artifact-local key's derivation.
    let source = id(IdPrefix::Source, "source", 7);
    let (_key, measured) = measure(|| {
        artifact_local_key(IdPrefix::Request, &source, 123_456)
            .expect("a small offset")
            .derive()
            .expect("the key derives")
    });
    report.bound(
        "key derivation",
        "artifact-local",
        measured.peak,
        &[("KEY_DERIVATION", model::KEY_DERIVATION)],
    );
}

const SYNTHETIC_RESPONSE: KeySpec = KeySpec {
    prefix: IdPrefix::Request,
    kind: "provider-response",
    precedence: 0,
    basis: IdentityBasis::Native,
    scope: IdScope::Provider,
    slots: &[
        ComponentSlot::required("provider", ComponentRole::Namespace),
        ComponentSlot::required("native_id", ComponentRole::NativeId),
    ],
};

const SYNTHETIC_REQUEST: KeySpec =
    KeySpec { kind: "provider-request", precedence: 1, ..SYNTHETIC_RESPONSE };

fn key(spec: &KeySpec, id: &str) -> DerivedKey {
    spec.key(vec![KeyComponent::text("synthetic"), KeyComponent::text(id)])
        .expect("a synthetic key is valid")
        .derive()
        .expect("a synthetic key derives")
}

fn stored_thread(name: &str) -> StoredIdentity {
    StoredIdentity::derive(IdentityKey::new(
        IdPrefix::Thread,
        "test-thread",
        vec![KeyComponent::text(name)],
    ))
    .expect("a thread identity derives")
}

/// A synthetic reconciliation input, each shaped so that one term dominates.
#[derive(Clone, Copy, Debug)]
enum Shape {
    /// One distinct key each: the request vector dominates grouping.
    CodexSingles,
    /// Pairs share a key: requests hold two records each.
    CodexPairs,
    /// No keys: each observation gets its artifact-local key.
    CodexKeyless,
    /// Every observation shares one key: one group, whose scratch dominates.
    OneGroup,
    /// Two distinct keys and the model invariant each: two key-graph nodes and an alias.
    ClaudeSingles,
    /// Pairs share both keys and disagree on the model: every pair splits.
    ClaudeSplits,
    /// Block records of three, a seventh of them splitting, with per-model usage and copies.
    ClaudeBlocks,
    /// Eight per-model usage components each: payloads dominate.
    PayloadHeavy,
    /// One thread per observation: thread reconciliation dominates.
    ManyThreads,
    /// Two limit observations per observation.
    Limits,
}

impl Shape {
    const ALL: [Self; 10] = [
        Self::CodexSingles,
        Self::CodexPairs,
        Self::CodexKeyless,
        Self::OneGroup,
        Self::ClaudeSingles,
        Self::ClaudeSplits,
        Self::ClaudeBlocks,
        Self::PayloadHeavy,
        Self::ManyThreads,
        Self::Limits,
    ];

    fn agent(self) -> Agent {
        match self {
            Self::ClaudeSingles | Self::ClaudeSplits | Self::ClaudeBlocks | Self::PayloadHeavy => {
                Agent::Claude
            }
            Self::CodexSingles
            | Self::CodexPairs
            | Self::CodexKeyless
            | Self::OneGroup
            | Self::ManyThreads
            | Self::Limits => Agent::Codex,
        }
    }
}

fn synthetic_thread(index: usize) -> Thread {
    Thread {
        identity: stored_thread(&format!("thread-{index}")),
        basis: IdentityBasis::Native,
        aliases: Vec::new(),
        native_key: [("session_id".to_owned(), format!("thread-{index}"))].into_iter().collect(),
        source: Basis::Observed("cli".to_owned()),
        initiator: Basis::Unknown,
        purpose: Basis::Unknown,
        execution_environment: Basis::Observed("local".to_owned()),
        project: Basis::Inferred("project".to_owned()),
        account: Basis::Unknown,
        evidence: vec![EvidenceRef::new(u32::try_from(index % 8).expect("fits"), 0, 10)],
    }
}

fn synthetic_input(shape: Shape, observations: usize) -> ReconcileInput {
    let sources: Vec<AnalyticalId> =
        (0..8).map(|index| id(IdPrefix::Source, "test-source", index)).collect();
    let thread_count = if matches!(shape, Shape::ManyThreads) { observations } else { 4 };
    let threads: Vec<Thread> = (0..thread_count).map(synthetic_thread).collect();
    let models = [Name::new("model-a"), Name::new("model-b")];
    let mut requests = Vec::with_capacity(observations);
    let mut limit_observations = Vec::new();
    for index in 0..observations {
        let source = u32::try_from(index % 8).expect("eight sources");
        let evidence = EvidenceRef::new(source, index as u64 * 100, 50);
        let mut observation = RequestObservation::new(evidence);
        let thread = &threads[index % thread_count].identity.id;
        observation.owner = OwnerEvidence::Proven(thread.clone());
        let usage = TokenMeasures {
            uncached_input: Some(index as u64),
            output: Some(7),
            ..TokenMeasures::default()
        };
        observation.usage = Some(usage.into());
        observation.timestamp = Some(
            jiff::Timestamp::from_second(1_780_000_000 + i64::try_from(index).expect("small"))
                .expect("time")
                .into(),
        );
        let served = |model: Name| Some(ModelName { name: model, basis: ModelBasis::Served });
        match shape {
            Shape::CodexSingles | Shape::ManyThreads | Shape::Limits => {
                observation.keys.push(key(&SYNTHETIC_RESPONSE, &format!("resp-{index}")));
            }
            Shape::CodexPairs => {
                observation.keys.push(key(&SYNTHETIC_RESPONSE, &format!("resp-{}", index / 2)));
            }
            Shape::CodexKeyless => {}
            Shape::OneGroup => {
                // Without owners, the group pushes no conflicting-owner diagnostic, so its
                // scratch stands alone.
                observation.owner = OwnerEvidence::None;
                observation.keys.push(key(&SYNTHETIC_RESPONSE, "resp-all"));
            }
            Shape::ClaudeSingles | Shape::PayloadHeavy => {
                observation.keys.push(key(&SYNTHETIC_RESPONSE, &format!("msg-{index}")));
                observation.keys.push(key(&SYNTHETIC_REQUEST, &format!("req-{index}")));
                observation.model = served(models[0]);
                observation.invariants.push(("model", models[0]));
                if matches!(shape, Shape::PayloadHeavy) {
                    observation.model_usage = (0..8)
                        .map(|component| ModelUsage {
                            model: served(models[component % 2]),
                            usage: usage.into(),
                            source: "advisor_message",
                        })
                        .collect::<Vec<_>>()
                        .into();
                }
            }
            Shape::ClaudeSplits => {
                observation.keys.push(key(&SYNTHETIC_RESPONSE, &format!("msg-{}", index / 2)));
                observation.keys.push(key(&SYNTHETIC_REQUEST, &format!("req-{}", index / 2)));
                observation.model = served(models[index % 2]);
                observation.invariants.push(("model", models[index % 2]));
            }
            Shape::ClaudeBlocks => {
                let block = index / 3;
                observation.keys.push(key(&SYNTHETIC_RESPONSE, &format!("msg-{block}")));
                observation.keys.push(key(&SYNTHETIC_REQUEST, &format!("req-{block}")));
                let model = models[usize::from(block % 7 == 0 && index % 3 == 2)];
                observation.model = served(model);
                observation.invariants.push(("model", model));
                if index % 13 == 0 {
                    observation.model_usage = vec![
                        ModelUsage {
                            model: served(model),
                            usage: usage.into(),
                            source: "message.usage",
                        },
                        ModelUsage { model: None, usage: usage.into(), source: "advisor_message" },
                    ]
                    .into();
                }
                if index % 17 == 0 {
                    observation.role = ObservationRole::Copy;
                }
            }
        }
        if matches!(shape, Shape::Limits) {
            for window in ["primary", "secondary"] {
                limit_observations.push(ProviderLimitObservation {
                    limit_name: Some(Name::new("codex")),
                    window: Some(Name::new(window)),
                    observed_at: Basis::Observed(
                        jiff::Timestamp::from_second(1_780_000_000).expect("time"),
                    ),
                    owner_thread: Some(thread.clone()),
                    owner_request: None,
                    native: Name::new(if index % 2 == 0 { "{\"used\":1}" } else { "{\"used\":2}" }),
                    evidence,
                });
            }
        }
        requests.push(observation);
    }
    ReconcileInput {
        threads,
        requests,
        limit_observations,
        source_table: SourceTable::from_ordered(sources),
        ..ReconcileInput::default()
    }
}

/// The counts a grouping checkpoint would see in `input`.
fn reconcile_counts(input: &ReconcileInput) -> ReconcileCounts {
    let key_nodes: u64 = input
        .requests
        .iter()
        .map(|observation| {
            observation.keys.len().max(1) as u64 + u64::from(!observation.invariants.is_empty())
        })
        .sum::<u64>()
        + 2 * input.links.len() as u64;
    ReconcileCounts {
        observations: input.requests.len() as u64,
        observation_capacity: input.requests.capacity() as u64,
        key_nodes,
        observation_payload: input.requests.iter().map(DeepSize::heap).sum(),
        largest_group: input.requests.len() as u64,
        limit_rows: input.limit_observations.len() as u64,
        threads: input.threads.len() as u64,
        thread_heap: input.threads.iter().map(DeepSize::heap).sum(),
        links: input.links.len() as u64,
        metadata: [
            model::sized_vec(input.threads.capacity() as u64, sizes::THREAD),
            input.relationships.heap(),
            input.tool_actions.heap(),
            input.links.heap(),
            input.gaps.heap(),
            input.diagnostics.heap(),
            input.source_table.heap(),
        ]
        .into_iter()
        .sum(),
    }
}

/// The live heap and the peak since the previous stage, at each stage.
struct Stages {
    entries: Vec<(ReconcileStage, u64, u64)>,
}

/// The observations in the largest linked set, read from the ledger: a request's records,
/// or a candidate set's members together.
fn largest_group(ledger: &Ledger) -> u64 {
    let size = |request: &Request| request.evidence().len() + request.copies().len();
    let records = ledger.requests.values().map(size);
    let split = ledger
        .candidate_sets
        .iter()
        .map(|set| set.iter().filter_map(|id| ledger.requests.get(id)).map(size).sum());
    records.chain(split).max().unwrap_or(0) as u64
}

fn check_reconcile(report: &mut Report, shape: Shape, observations: usize) {
    let agent = shape.agent();
    let case = format!("{shape:?} × {observations}");
    let base = live();
    let input = synthetic_input(shape, observations);
    let counts = reconcile_counts(&input);
    let grouping = model::grouping_estimate(agent, &counts);
    let finalize = model::finalize_estimate(agent, &counts);
    let mut stages = Stages { entries: Vec::with_capacity(16) };
    EXCESS.store(0, Ordering::Relaxed);
    PEAK.store(live(), Ordering::Relaxed);
    let capacity = ObservationCapacity::from_rows(usize::MAX);
    // `E` is each phase's checkpoint estimate. Growth that grouping pushes is only known
    // afterwards, so live heap above `E` at any allocation is checked against it below.
    let ledger = reconcile_with_stages(input, &LatestRevision, &capacity, &mut |stage| {
        let now = live();
        let peak = PEAK.swap(now, Ordering::Relaxed);
        stages.entries.push((stage, now, peak));
        match stage {
            ReconcileStage::Start => LIMIT.store(base + grouping.total(), Ordering::Relaxed),
            ReconcileStage::Released => LIMIT.store(base + finalize.total(), Ordering::Relaxed),
            ReconcileStage::Canonical
            | ReconcileStage::KeyGraph
            | ReconcileStage::Order
            | ReconcileStage::Requests
            | ReconcileStage::Sorted
            | ReconcileStage::Finalized => {}
        }
    })
    .expect("the synthetic input reconciles");
    LIMIT.store(u64::MAX, Ordering::Relaxed);
    let excess = EXCESS.swap(0, Ordering::Relaxed);

    // What grouping pushed, read from the result. A diagnostic's occurrences count the
    // evidence it held before compaction kept a sample.
    let requests = ledger.requests.len() as u64;
    let parts: u64 = ledger.candidate_sets.iter().map(|set| set.len() as u64).sum();
    let sets = requests - parts + ledger.candidate_sets.len() as u64;
    let split = if requests > sets + sets / 32 { model::split_growth(requests) } else { 0 };
    let conflicting: u64 =
        ledger.candidate_sets.iter().map(|set| model::conflicting_key(set.len() as u64)).sum();
    let ambiguous: u64 = ledger
        .requests
        .values()
        .map(|request| match &request.ownership {
            Ownership::Ambiguous { candidates } => model::ambiguous_owners(candidates.len() as u64),
            Ownership::Owned { .. } | Ownership::Unknown => 0,
        })
        .sum();
    let diagnostics: u64 = ledger
        .diagnostics
        .iter()
        .map(|diagnostic| {
            model::diagnostic(diagnostic.detail.capacity() as u64, diagnostic.occurrences)
        })
        .sum();
    let clone = ledger
        .requests
        .values()
        .map(|request| request.usage.as_ref().map_or(0, DeepSize::heap))
        .max()
        .unwrap_or(0);
    let aliases: u64 = ledger.requests.values().map(|request| request.aliases.len() as u64).sum();
    let most_aliases =
        ledger.requests.values().map(|request| request.aliases.len() as u64).max().unwrap_or(0);
    let limits = ledger.limit_observations.len() as u64;
    let largest = largest_group(&ledger);
    let pushed = split + conflicting + ambiguous + diagnostics;
    report.bound_total("live above E", &case, excess, pushed);

    let stage_delta = |stage: ReconcileStage| -> u64 {
        let index =
            stages.entries.iter().position(|(seen, _, _)| *seen == stage).expect("stage seen");
        let (_, _, peak) = stages.entries[index];
        let (_, previous, _) = stages.entries[index - 1];
        peak.saturating_sub(previous)
    };
    report.bound(
        "stage canonical",
        &case,
        stage_delta(ReconcileStage::Canonical),
        &[("thread_reconcile", model::thread_reconcile(counts.threads, counts.thread_heap))],
    );
    report.bound(
        "stage key graph",
        &case,
        stage_delta(ReconcileStage::KeyGraph),
        &[("key_graph", model::key_graph(counts.key_nodes, counts.observations))],
    );
    report.bound(
        "stage order",
        &case,
        stage_delta(ReconcileStage::Order),
        &[("grouping_order", model::grouping_order(counts.observations))],
    );
    // The selected revision's per-model usage is cloned into its request just before the
    // group's observations release theirs, so one clone is live at a time.
    report.bound_beside(
        "stage requests",
        &case,
        stage_delta(ReconcileStage::Requests),
        &[
            ("request_vector", model::request_vector(sets, counts.observations, most_aliases)),
            ("group_scratch", model::group_scratch(agent, largest)),
            ("split_growth", split),
            ("conflicting_key", conflicting),
            ("ambiguous_owners", ambiguous),
            ("diagnostic", diagnostics),
        ],
        clone,
    );
    report.bound(
        "stage sorted",
        &case,
        stage_delta(ReconcileStage::Sorted),
        &[("request_sort", model::request_sort(requests))],
    );
    report.bound(
        "stage finalized",
        &case,
        stage_delta(ReconcileStage::Finalized),
        &[
            ("alias_map", model::alias_map(aliases)),
            ("limit_row", limits.max(counts.limit_rows) * model::limit_row()),
            ("diagnostic", diagnostics),
            ("conflicting_key", conflicting),
        ],
    );
    // The whole run, from before the input existed, against the larger phase estimate,
    // plus what grouping pushed.
    let (_, end, _) = *stages.entries.last().expect("stages seen");
    let peak = stages.entries.iter().map(|(_, _, peak)| *peak).max().unwrap_or(0).max(end);
    report.bound_total(
        "reconcile phases",
        &case,
        peak.saturating_sub(base),
        grouping.total().max(finalize.total()) + pushed,
    );
    // The heap the ledger frees when dropped is what it retains.
    let heap = ledger.heap();
    let before = live();
    drop(ledger);
    report.bound_total("reconcile retained", &case, before.saturating_sub(live()), heap);
}

/// The request vector's growth once split parts push it past its presized capacity.
fn check_split_growth(report: &mut Report) {
    let (sets, requests) = (2_000_usize, 4_000_usize);
    let template = Request {
        id: id(IdPrefix::Request, "split", 0),
        basis: IdentityBasis::Native,
        aliases: Box::new([]),
        ownership: Ownership::Unknown,
        first_seen: None,
        last_seen: None,
        model: None,
        effort: None,
        usage: None,
        records: RecordRefs::Empty,
        originals: 0,
        counting: RequestCounting::Counted,
    };
    let ((), measured) = measure(|| {
        let mut rows = Vec::with_capacity(sets + sets / 32);
        let presized = live();
        PEAK.store(presized, Ordering::Relaxed);
        for _ in 0..requests {
            rows.push(template.clone());
        }
    });
    report.bound(
        "split growth",
        "2,000 sets, 4,000 requests",
        measured.peak,
        &[("split_growth", model::split_growth(requests as u64))],
    );
}

/// A report over a synthetic ledger with many requests and few rows, so the per-request
/// lists dominate the document.
fn check_query_reserve(report: &mut Report) {
    let mut ledger = reconcile_with_stages(
        synthetic_input(Shape::CodexSingles, 8_000),
        &LatestRevision,
        &ObservationCapacity::from_rows(usize::MAX),
        &mut |_| {},
    )
    .expect("the synthetic input reconciles");
    let threads = std::mem::take(&mut ledger.threads);
    let ingested = Ingested { ledger, threads, ..Ingested::default() };
    let mut index = SessionIndex::default();
    index.add(Agent::Codex, &ingested).expect("the index adds");
    let sources = [QuerySource { agent: Agent::Codex, ingested: &ingested }];
    let timezone = ResolvedTimeZone::resolve(Some("UTC")).expect("UTC resolves");
    let metadata = QueryMetadata::new("report", "all", Scope::SelfOnly, &timezone);
    let (document, measured) = measure(|| {
        query::report(&sources, &index, &BTreeSet::new(), true, metadata, &BTreeSet::new())
            .expect("the report queries")
    });
    let rows: usize = document.breakdowns.values().map(Vec::len).sum();
    let requests = ingested.ledger.requests.len() as u64;
    report.bound(
        "query report",
        "8,000 synthetic requests",
        measured.peak,
        &[
            ("query reserve", requests * model::QUERY_RESERVE),
            ("QUERY_PER_ROW", rows as u64 * model::QUERY_PER_ROW),
            ("QUERY_FIXED", model::QUERY_FIXED),
        ],
    );
}

/// Request-bearing records and decoded records of an ingest, read after the fact.
struct IngestCounts {
    decoded: u64,
    observations: u64,
    limit_rows: u64,
}

fn counts(ingested: &Ingested) -> IngestCounts {
    IngestCounts {
        decoded: ingested.manifest.entries.iter().map(|entry| entry.counters.decoded).sum(),
        observations: ingested.ledger.coverage.observations,
        limit_rows: ingested.limit_observations.len() as u64,
    }
}

/// Discovers and ingests `root`, checking discovery and the retained ledger against their
/// deep heap, then reports the peak beside the forward estimate that can be computed
/// after the fact.
fn check_ingest(report: &mut Report, agent: Agent, case: &str, root: &Path) {
    // The first run warms lazily initialized state: interned names, timezone data and
    // thread-local caches, which belong to the baseline or to process interns.
    drop(ingest(agent, root, 1));
    let roots = roots(agent, root);
    let (discovery, measured) = measure(|| discover(&roots));
    report.bound_total(
        "discovery",
        &format!("{}/{case}", agent.token()),
        measured.retained,
        discovery.heap(),
    );
    drop(discovery);
    for count in [1, 8] {
        let interns_before = interns();
        let (ingested, measured) = measure(|| ingest(agent, root, count));
        let new_interns = interns() - interns_before;
        let label = format!("{}/{case} ({count}w)", agent.token());
        report.bound_total(
            "retained ledger",
            &label,
            measured.retained,
            ingested.heap() + new_interns,
        );
        let counts = counts(&ingested);
        // Metadata stays beside the request structure that κ covers.
        let metadata = [
            ingested.manifest.heap(),
            ingested.sources.heap(),
            ingested.threads.heap(),
            ingested.relationships.heap(),
            ingested.ledger.diagnostics.heap(),
            ingested.ledger.gaps.heap(),
            ingested.ledger.source_table.heap(),
        ]
        .into_iter()
        .sum::<u64>();
        let forward = counts.decoded * model::decoded_record(agent)
            + counts.observations * model::kappa(agent)
            + counts.limit_rows * model::limit_row()
            + metadata
            + new_interns;
        report.note(
            "ingest peak",
            &label,
            format!(
                "peak {} vs forward estimate {forward} without payloads or worker slots ({} requested retained)",
                measured.peak, measured.retained_requested
            ),
        );
        drop(ingested);
    }
}

fn write_lines(path: &Path, lines: &[serde_json::Value]) {
    fs::create_dir_all(path.parent().expect("a corpus file has a parent")).expect("corpus dir");
    let mut text = String::new();
    for line in lines {
        text.push_str(&line.to_string());
        text.push('\n');
    }
    fs::write(path, text).expect("the corpus file is written");
}

/// A Claude project of `sessions` sessions with `records` assistant records each: block
/// records share message IDs, some lines replay another session's record as a nested
/// progress copy, every fifth record reports quota limits, and models alternate.
fn claude_corpus(sessions: usize, records: usize, padding: usize) -> tempfile::TempDir {
    let root = tempfile::tempdir().expect("a temporary corpus root");
    for session in 0..sessions {
        let id = format!("00000000-0000-4000-8000-{session:012}");
        let mut lines = Vec::new();
        for record in 0..records {
            let message = format!("msg_{session}_{}", record / 3);
            let assistant = json!({
                "type": "assistant", "sessionId": id, "cwd": "/home/example/project",
                "version": "2.1.0",
                "uuid": format!("uuid-{session}-{record}"),
                "requestId": format!("req_{session}_{}", record / 3),
                "timestamp": format!("2026-09-{:02}T10:{:02}:{:02}.000Z", session % 28 + 1, record / 60 % 60, record % 60),
                "message": {"id": message, "model": if record % 2 == 0 { "claude-a" } else { "claude-b" },
                    "content": [{"type": "text", "text": "x".repeat(padding)}],
                    "usage": {"input_tokens": record + 1, "output_tokens": record % 7 + 1,
                        "cache_read_input_tokens": 10}},
                "quotaLimits": if record % 5 == 0 {
                    json!({"status": "allowed", "rateLimitType": "five_hour", "resetsAt": record})
                } else { serde_json::Value::Null },
            });
            if record % 11 == 10 && session > 0 {
                lines.push(json!({"type": "progress", "sessionId": id,
                    "uuid": format!("progress-{session}-{record}"),
                    "data": {"message": {
                        "type": "assistant", "sessionId": format!("00000000-0000-4000-8000-{:012}", session - 1),
                        "uuid": format!("uuid-{}-{record}", session - 1),
                        "message": {"id": format!("msg_{}_{}", session - 1, record / 3), "model": "claude-a",
                            "usage": {"input_tokens": 1, "output_tokens": 1}}}}}));
            }
            lines.push(assistant);
        }
        write_lines(
            &root.path().join(format!("projects/-home-example-project/{id}.jsonl")),
            &lines,
        );
    }
    root
}

/// A Codex home of `rollouts` rollouts with `records` token counts each, whose rate limits
/// change every `limit_period` records, plus one direct-usage rollout.
fn codex_corpus(
    rollouts: usize,
    records: usize,
    limit_period: usize,
    padding: usize,
) -> tempfile::TempDir {
    let root = tempfile::tempdir().expect("a temporary corpus root");
    for rollout in 0..=rollouts {
        let thread = format!("019f0000-0000-7000-8000-{rollout:012}");
        let mut lines = vec![
            json!({"timestamp": "2026-09-02T17:00:00.000Z", "type": "session_meta",
                "payload": {"id": thread, "cwd": "/home/example/project", "cli_version": "0.150.0",
                    "source": "cli", "thread_source": "user"}}),
            json!({"timestamp": "2026-09-02T17:00:00.100Z", "type": "turn_context",
                "payload": {"turn_id": format!("turn-{rollout}"), "model": "gpt-test", "effort": "medium"}}),
        ];
        for record in 0..records {
            let total = (record + 1) * 100;
            lines.push(json!({"timestamp": "2026-09-02T17:00:01.000Z", "type": "response_item",
                "payload": {"type": "message", "content": [{"type": "output_text", "text": "x".repeat(padding)}]}}));
            if rollout == rollouts {
                lines.push(
                    json!({"timestamp": "2026-09-02T17:00:02.000Z", "type": "token_usage_record",
                    "payload": {"thread_id": thread, "turn_id": format!("turn-{rollout}"),
                        "response_id": format!("resp-{record}"),
                        "usage": {"input_tokens": 100, "output_tokens": 10, "total_tokens": 110}}}),
                );
            } else {
                lines.push(json!({"timestamp": "2026-09-02T17:00:02.000Z", "type": "event_msg",
                    "payload": {"type": "token_count",
                        "info": {"total_token_usage": {"input_tokens": total, "output_tokens": total / 10, "total_tokens": total + total / 10},
                            "last_token_usage": {"input_tokens": 100, "output_tokens": 10, "total_tokens": 110}},
                        "rate_limits": {"limit_id": "codex", "primary": {"used_percent": record / limit_period, "window_minutes": 300},
                            "secondary": {"used_percent": 1, "window_minutes": 10080}}}}));
            }
        }
        let name = format!("sessions/2026/09/02/rollout-2026-09-02T17-00-00-{thread}.jsonl");
        write_lines(&root.path().join(name), &lines);
    }
    root
}

/// Checks the session index and the query documents over ingested sources, the document
/// apart from its rendering.
fn check_query(report: &mut Report, case: &str, sources: &[(Agent, &Ingested)]) {
    let mut index = SessionIndex::default();
    for (agent, ingested) in sources {
        let before = index.heap();
        let ((), measured) = measure(|| index.add(*agent, ingested).expect("the index adds"));
        report.bound_total(
            "session index",
            &format!("{case} {}", agent.token()),
            measured.retained,
            index.heap() - before,
        );
    }
    let query_sources: Vec<QuerySource<'_>> =
        sources.iter().map(|(agent, ingested)| QuerySource { agent: *agent, ingested }).collect();
    let timezone = ResolvedTimeZone::resolve(Some("UTC")).expect("UTC resolves");
    let requests: u64 =
        sources.iter().map(|(_, ingested)| ingested.ledger.requests.len() as u64).sum();
    let selected = BTreeSet::new();
    let metadata = || QueryMetadata::new("report", "all", Scope::SelfOnly, &timezone);
    let document_terms = |rows: usize| -> [Term; 3] {
        [
            ("query reserve", requests * model::QUERY_RESERVE),
            ("QUERY_PER_ROW", rows as u64 * model::QUERY_PER_ROW),
            ("QUERY_FIXED", model::QUERY_FIXED),
        ]
    };
    let (document, measured) = measure(|| {
        query::report(&query_sources, &index, &selected, true, metadata(), &BTreeSet::new())
            .expect("the report queries")
    });
    let rows: usize = document.breakdowns.values().map(Vec::len).sum();
    report.bound("query report", case, measured.peak, &document_terms(rows));
    let (text, rendered) =
        measure(|| serde_json::to_string(&document).expect("the report renders"));
    report.bound(
        "render report",
        case,
        rendered.peak,
        &[("render", model::render(text.len() as u64))],
    );
    drop((document, text));
    let (document, measured) = measure(|| {
        query::daily(&query_sources, &selected, true, metadata(), &timezone).expect("daily")
    });
    report.bound("query daily", case, measured.peak, &document_terms(document.rows.len()));
    drop(document);
    let (document, measured) = measure(|| {
        query::sessions(&query_sources, &index, &selected, true, metadata(), &timezone)
            .expect("sessions")
    });
    report.bound("query sessions", case, measured.peak, &document_terms(document.rows.len()));
    let (text, rendered) = measure(|| serde_json::to_string(&document).expect("sessions render"));
    report.bound(
        "render sessions",
        case,
        rendered.peak,
        &[("render", model::render(text.len() as u64))],
    );
    let per_row = measured.peak.saturating_sub(requests * model::QUERY_RESERVE)
        / (document.rows.len().max(1) as u64);
    report.note(
        "query sessions",
        case,
        format!(
            "{requests} requests, {} rows: {per_row} bytes per row above the per-request lists",
            document.rows.len()
        ),
    );
}

/// Each decoder's Rust heap once it starts, a long line's scan, and zstd's C state.
fn check_decoders(report: &mut Report) {
    let line = "y".repeat(7 << 19);
    let plain = format!("{{\"a\":1}}\n{line}\n");
    // A gzip member whose header carries the longest name, comment and extra field.
    let mut gzip = flate2::GzBuilder::new()
        .filename(vec![b'n'; 65_535])
        .comment(vec![b'c'; 65_535])
        .extra(vec![b'e'; 65_535])
        .write(Vec::new(), flate2::Compression::fast());
    gzip.write_all(plain.as_bytes()).expect("gzip encodes");
    let gzip = gzip.finish().expect("gzip finishes");
    let mut zstd = zstd::stream::write::Encoder::new(Vec::new(), 1).expect("zstd encoder");
    zstd.window_log(23).expect("an 8 MiB window");
    zstd.write_all(plain.as_bytes()).expect("zstd encodes");
    let zstd = zstd.finish().expect("zstd finishes");
    let read_buffer: Term =
        ("read buffer", allocation(model::READ_BUFFER) + allocation(model::READER_STATE));
    for (name, representation, bytes, decoder) in [
        ("plain", Representation::Plain, plain.as_bytes(), None),
        (
            "gzip, longest header",
            Representation::Gzip,
            gzip.as_slice(),
            Some(("GZIP_DECODER", model::GZIP_DECODER)),
        ),
        (
            "zstd",
            Representation::Zstd,
            zstd.as_slice(),
            Some(("zstd input", allocation(model::ZSTD_INPUT_BUFFER))),
        ),
    ] {
        let ((), measured) = measure(|| {
            let mut reader = decode(bytes, representation).expect("the decoder starts");
            let first = reader.fill_buf().expect("the stream decodes").len();
            reader.consume(first);
        });
        let terms: Vec<Term> = std::iter::once(read_buffer).chain(decoder).collect();
        report.bound("decoder heap", name, measured.peak, &terms);
    }

    let directory = tempfile::tempdir().expect("a temporary directory");
    let path = directory.path().join("long-line.jsonl");
    fs::write(&path, &plain).expect("the long line is written");
    let files = LogicalSource { plain: Some(path), ..LogicalSource::default() };
    let spec = SourceSpec {
        environment: "local",
        dialect: "test",
        locator: "long-line",
        stable_locator: true,
    };
    let (entry, measured) = measure(|| {
        read_source(&spec, &files, &ReadOptions::default(), |_| RecordDisposition::Skipped)
            .expect("the source reads")
    });
    // The scan's line buffer stays within the slot's line need at its 4 MiB capacity; the
    // read buffer, the boxed reader and the manifest entry the scan returns are beside it.
    report.bound_beside(
        "scan heap",
        "3.5 MiB line",
        measured.peak,
        &[read_buffer, ("line_need", model::line_need(model::SLOT_LINE_CAPACITY))],
        entry.heap(),
    );
    drop(entry);

    let context = zstd::zstd_safe::DCtx::create();
    report.bound(
        "zstd context",
        "ZSTD_sizeof_DCtx",
        context.sizeof() as u64,
        &[("ZSTD_CONTEXT", model::ZSTD_CONTEXT)],
    );
    let mut stream = zstd::zstd_safe::DCtx::create();
    let mut output = vec![0_u8; 1 << 16];
    let mut input = zstd::zstd_safe::InBuffer::around(&zstd);
    let mut out = zstd::zstd_safe::OutBuffer::around(&mut output[..]);
    stream.decompress_stream(&mut out, &mut input).expect("a frame decodes");
    report.bound(
        "zstd stream",
        "8 MiB window",
        stream.sizeof() as u64,
        &[("zstd_stream", model::zstd_stream(model::SLOT_ZSTD_WINDOW))],
    );
}

fn main() {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    if !selected(&arguments) {
        return;
    }
    let mut report = Report::default();
    let started = std::time::Instant::now();
    check_cold_interns(&mut report);
    check_name_structures(&mut report);
    for agent in [Agent::Codex, Agent::Claude] {
        let structure = model::request_structure(agent);
        report.note(
            "model",
            agent.token(),
            format!(
                "κ {} (construction {}, grouping {}, finalize {}, retained {}); decoded record {}; key bound {}",
                structure.kappa(),
                structure.construction,
                structure.grouping,
                structure.finalize,
                structure.retained,
                model::decoded_record(agent),
                model::key_bound(agent),
            ),
        );
    }
    report.note(
        "model",
        "worker slot",
        format!(
            "{} plain, {} with zstd, {} with gzip; limit row {}; F {}",
            model::worker_slot(false, false),
            model::worker_slot(true, false),
            model::worker_slot(false, true),
            model::limit_row(),
            model::BASELINE_BYTES,
        ),
    );
    report.note(
        "model",
        "slot parts",
        format!(
            "join pair {}, done vector {}, name cache {}, gzip {}, zstd {}, largest permit need {}, Codex turn state {}",
            model::JOIN_PAIR,
            model::JOIN_DONE,
            model::NAME_CACHE,
            model::GZIP_DECODER,
            model::ZSTD_DECODER,
            model::LARGEST_LARGE_RECORD,
            model::CODEX_TURN_STATE,
        ),
    );
    check_primitives(&mut report);
    report.note("time", "primitives", format!("{} ms", started.elapsed().as_millis()));
    check_structures(&mut report);
    check_split_growth(&mut report);
    for shape in Shape::ALL {
        // Warm the shape's names and identities first.
        drop(reconcile_with_stages(
            synthetic_input(shape, 10),
            &LatestRevision,
            &ObservationCapacity::from_rows(usize::MAX),
            &mut |_| {},
        ));
        check_reconcile(&mut report, shape, 4_000);
    }
    report.note("time", "reconcile", format!("{} ms", started.elapsed().as_millis()));
    for (agent, dialect) in [(Agent::Claude, "claude-project"), (Agent::Codex, "codex-rollout")] {
        for case in fixtures(dialect) {
            let name = case
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default();
            check_ingest(&mut report, agent, &name, &case);
        }
    }
    for (sessions, records, padding) in [(2, 200, 0), (4, 200, 0), (4, 200, 2048)] {
        let corpus = claude_corpus(sessions, records, padding);
        let case = format!("generated {sessions}×{records} pad {padding}");
        check_ingest(&mut report, Agent::Claude, &case, corpus.path());
    }
    for (rollouts, records, period, padding) in
        [(2, 200, 1, 0), (4, 200, 50, 0), (4, 200, 50, 2048)]
    {
        let corpus = codex_corpus(rollouts, records, period, padding);
        let case = format!("generated {rollouts}×{records} limits/{period} pad {padding}");
        check_ingest(&mut report, Agent::Codex, &case, corpus.path());
    }
    report.note("time", "ingest", format!("{} ms", started.elapsed().as_millis()));
    {
        let empty = Ingested::default();
        check_query(&mut report, "no requests", &[(Agent::Codex, &empty)]);
    }
    for (sessions, records) in [(4, 200), (100, 4)] {
        let claude = claude_corpus(sessions, records, 0);
        let codex = codex_corpus(sessions, records, 50, 0);
        let claude = ingest(Agent::Claude, claude.path(), 1);
        let codex = ingest(Agent::Codex, codex.path(), 1);
        let case = format!("generated {sessions}×{records}");
        check_query(&mut report, &case, &[(Agent::Claude, &claude), (Agent::Codex, &codex)]);
    }
    check_query_reserve(&mut report);
    check_decoders(&mut report);
    report.require_necessary_terms();
    let mut out = String::new();
    writeln!(out, "memory model harness ({} ms)", started.elapsed().as_millis()).expect("writes");
    for line in &report.lines {
        writeln!(out, "{line}").expect("writes");
    }
    print!("{out}");
    assert!(
        report.violations.is_empty(),
        "live heap exceeded the model, or a term is never necessary:\n{}",
        report.violations.join("\n")
    );
}
