//! The admission model-bound harness (scalable-ingestion plan, "Admission Tests"): a
//! counting global allocator measures live Rust heap above its starting level and
//! compares it with the cost model of `ledger::admission::model`.
//!
//! Admission is not wired into ingestion yet, so this binary checks what can be
//! estimated from outside: the retained ledger against its deep size, reconciliation
//! against its grouping and finalize estimates, and the query against its per-request and
//! per-row costs. Those checks fail the test when the measured heap exceeds the estimate.
//! It also reports, without failing, the whole-ingest peak beside the forward estimate it
//! can compute after the fact, and measures the decoder constants of a worker slot.
//!
//! It runs without the libtest harness, single-threaded apart from the decoding workers it
//! starts, so no other test's allocations reach the counters. Its `unsafe` is the
//! allocator below and stays in this binary.

use std::alloc::{GlobalAlloc, Layout, System};
use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::fs;
use std::io::{BufRead as _, Write as _};
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::json;
use urollup_core::adapters::{Ingested, claude_project, codex_rollout};
use urollup_core::ledger::admission::deep_size::DeepSize;
use urollup_core::ledger::admission::model::{
    self, ReconcileCounts, allocation, finalize_estimate, grouping_estimate,
};
use urollup_core::ledger::entities::{Basis, ModelBasis, ModelName, ModelUsage, Requests, Thread};
use urollup_core::ledger::identity::{
    AnalyticalId, IdPrefix, IdentityKey, KeyComponent, StoredIdentity,
};
use urollup_core::ledger::names::{self, Name};
use urollup_core::ledger::reconcile::{
    LatestRevision, ObservationRole, OwnerEvidence, ReconcileInput, RequestObservation, reconcile,
};
use urollup_core::ledger::scope::{
    ComponentRole, ComponentSlot, DerivedKey, IdScope, IdentityBasis, KeySpec,
};
use urollup_core::ledger::tokens::{self, TokenMeasures};
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

/// The costing rule's charge for one allocation of `size` bytes.
fn cost(size: usize) -> u64 {
    allocation(size as u64)
}

fn grow(size: usize) {
    REQUESTED.fetch_add(size as u64, Ordering::Relaxed);
    let live = LIVE.fetch_add(cost(size), Ordering::Relaxed) + cost(size);
    PEAK.fetch_max(live, Ordering::Relaxed);
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
        // A moving reallocation holds both buffers, so the new one counts before the old
        // one is released.
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
    let start = LIVE.load(Ordering::Relaxed);
    let start_requested = REQUESTED.load(Ordering::Relaxed);
    PEAK.store(start, Ordering::Relaxed);
    let value = operation();
    let end = LIVE.load(Ordering::Relaxed);
    let measured = Measured {
        peak: PEAK.load(Ordering::Relaxed).saturating_sub(start),
        retained: end.saturating_sub(start),
        retained_requested: REQUESTED.load(Ordering::Relaxed).saturating_sub(start_requested),
    };
    (value, measured)
}

/// Comparisons of measured heap with the model, and the failures among them.
#[derive(Default)]
struct Report {
    lines: Vec<String>,
    violations: Vec<String>,
}

impl Report {
    /// Records a bound: `measured` must not exceed `estimate`.
    fn bound(&mut self, check: &str, case: &str, measured: u64, estimate: u64) {
        let ratio = ratio(measured, estimate);
        let verdict = if measured <= estimate { "ok" } else { "VIOLATION" };
        self.lines.push(format!(
            "{verdict:9} {check:22} {case:42} measured {measured:>11} estimate {estimate:>11} ({ratio})"
        ));
        if measured > estimate {
            self.violations.push(format!("{check} {case}: {measured} > {estimate}"));
        }
    }

    /// Records a measurement the model does not bound yet.
    fn note(&mut self, check: &str, case: &str, text: impl AsRef<str>) {
        self.lines.push(format!("{:9} {check:22} {case:42} {}", "report", text.as_ref()));
    }
}

fn ratio(measured: u64, estimate: u64) -> String {
    if estimate == 0 {
        return "no estimate".to_owned();
    }
    let percent = u128::from(measured) * 100 / u128::from(estimate);
    format!("{percent}% of estimate")
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

fn ingest(agent: Agent, root: &Path, count: usize) -> Ingested {
    match agent {
        Agent::Claude => claude_project::ingest_discovery_with_workers(
            discover(&[root.to_owned()]),
            true,
            workers(count),
        ),
        Agent::Codex => {
            let roots = codex_rollout::rollout_roots(&[root.to_owned()]);
            codex_rollout::ingest_discovery_with_workers(discover(&roots), true, workers(count))
        }
        Agent::Pi => unreachable!("Pi has no adapter"),
    }
    .expect("the corpus ingests")
}

/// The model's charge for every process intern so far: names and overflow patterns.
fn interns() -> u64 {
    names::interned_bytes() + tokens::overflow_interned_bytes()
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

/// Ingests `root` and checks the retained ledger against its deep size, then reports the
/// peak beside the forward estimate that can be computed after the fact.
fn check_ingest(report: &mut Report, agent: Agent, case: &str, root: &Path) {
    // The first run warms lazily initialized state: interned names, timezone data and
    // thread-local caches, which belong to the baseline or to process interns.
    drop(ingest(agent, root, 1));
    for count in [1, 8] {
        let interns_before = interns();
        let (ingested, measured) = measure(|| ingest(agent, root, count));
        let new_interns = interns() - interns_before;
        let retained = ingested.deep_size() + new_interns;
        let label = format!("{}/{case} ({count}w)", agent.token());
        report.bound("retained ledger", &label, measured.retained, retained);
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
        let slots = count as u64 * model::worker_slot(false, false);
        report.note(
            "ingest peak",
            &label,
            format!(
                "peak {} vs forward estimate {} without payloads or {} of worker slots ({} requested retained)",
                measured.peak, forward, slots, measured.retained_requested
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

/// A reconciliation input shaped like `agent`'s observations: Claude block records share
/// a response and request key, carry the model invariant, sometimes conflict on it and so
/// split, and carry per-model usage; Codex records carry one key each, some shared by
/// revisions, and a tenth carry none.
fn synthetic_input(agent: Agent, observations: usize) -> ReconcileInput {
    let sources: Vec<AnalyticalId> = (0..8)
        .map(|index| {
            IdentityKey::new(IdPrefix::Source, "test-source", vec![KeyComponent::Integer(index)])
                .derive_id()
                .expect("a source ID derives")
        })
        .collect();
    let threads: Vec<Thread> = (0_u32..4)
        .map(|index| {
            let identity = stored_thread(&format!("thread-{index}"));
            Thread {
                identity,
                basis: IdentityBasis::Native,
                aliases: Vec::new(),
                native_key: [("session_id".to_owned(), format!("thread-{index}"))]
                    .into_iter()
                    .collect(),
                source: Basis::Observed("cli".to_owned()),
                initiator: Basis::Unknown,
                purpose: Basis::Unknown,
                execution_environment: Basis::Observed("local".to_owned()),
                project: Basis::Inferred("project".to_owned()),
                account: Basis::Unknown,
                evidence: vec![EvidenceRef::new(index, 0, 10)],
            }
        })
        .collect();
    let models = [Name::new("model-a"), Name::new("model-b")];
    let mut requests = Vec::with_capacity(observations);
    for index in 0..observations {
        let source = u32::try_from(index % 8).expect("eight sources");
        let evidence = EvidenceRef::new(source, index as u64 * 100, 50);
        let mut observation = RequestObservation::new(evidence);
        observation.owner = OwnerEvidence::Proven(threads[index % 4].identity.id.clone());
        let usage = TokenMeasures {
            uncached_input: Some(index as u64),
            output: Some(7),
            ..TokenMeasures::default()
        };
        observation.usage = Some(usage.into());
        observation.timestamp = Some(
            jiff::Timestamp::from_second(
                1_780_000_000 + i64::try_from(index).expect("a small index"),
            )
            .expect("time")
            .into(),
        );
        match agent {
            Agent::Claude => {
                let block = index / 3;
                observation.keys.push(key(&SYNTHETIC_RESPONSE, &format!("msg-{block}")));
                observation.keys.push(key(&SYNTHETIC_REQUEST, &format!("req-{block}")));
                // Every seventh block's last record reports another model, so it splits.
                let model = models[usize::from(block % 7 == 0 && index % 3 == 2)];
                observation.model = Some(ModelName { name: model, basis: ModelBasis::Served });
                observation.invariants.push(("model", model));
                if index % 13 == 0 {
                    observation.model_usage = vec![
                        ModelUsage {
                            model: observation.model,
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
            Agent::Codex | Agent::Pi => {
                if index % 10 != 9 {
                    observation.keys.push(key(&SYNTHETIC_RESPONSE, &format!("resp-{}", index / 2)));
                }
                observation.model =
                    Some(ModelName { name: models[0], basis: ModelBasis::Requested });
            }
        }
        requests.push(observation);
    }
    ReconcileInput {
        threads,
        requests,
        source_table: SourceTable::from_ordered(sources),
        ..ReconcileInput::default()
    }
}

/// The observations in the largest linked set, read from the ledger: a request's records,
/// or a candidate set's members together.
fn largest_group(requests: &Requests, candidate_sets: &[BTreeSet<AnalyticalId>]) -> u64 {
    let records =
        requests.values().map(|request| request.evidence().len() + request.copies().len());
    let split = candidate_sets.iter().map(|set| {
        set.iter()
            .filter_map(|id| requests.get(id))
            .map(|request| request.evidence().len() + request.copies().len())
            .sum()
    });
    records.chain(split).max().unwrap_or(0) as u64
}

fn check_reconcile(report: &mut Report, agent: Agent, observations: usize) {
    let interns_before = interns();
    let ((ledger, counts), measured) = measure(|| {
        let input = synthetic_input(agent, observations);
        let metadata = input.threads.heap()
            + input.source_table.heap()
            + model::sized_vec(
                input.threads.capacity() as u64,
                std::mem::size_of::<Thread>() as u64,
            );
        let counts = ReconcileCounts {
            observations: input.requests.len() as u64,
            observation_capacity: input.requests.capacity() as u64,
            observation_payload: input.requests.iter().map(DeepSize::heap).sum(),
            largest_group: 0,
            limit_rows: 0,
            metadata,
        };
        (reconcile(input, &LatestRevision).expect("the synthetic input reconciles"), counts)
    });
    let counts = ReconcileCounts {
        largest_group: largest_group(&ledger.requests, &ledger.candidate_sets),
        ..counts
    };
    // Diagnostics pushed while grouping are charged where they are pushed.
    let pushed: u64 = ledger
        .diagnostics
        .iter()
        .map(|diagnostic| {
            model::diagnostic(diagnostic.detail.len() as u64, diagnostic.evidence.len() as u64)
        })
        .sum();
    // Requests split past the presized vector are charged where they are pushed too.
    let parts: u64 = ledger.candidate_sets.iter().map(|set| set.len() as u64 - 1).sum();
    let requests = ledger.requests.len() as u64;
    let groups = requests - parts;
    let split = if requests > groups + groups / 32 { model::split_growth(requests) } else { 0 };
    let estimate =
        grouping_estimate(agent, &counts).max(finalize_estimate(&counts)) + pushed + split;
    let label = format!("{} × {observations}", agent.token());
    report.bound("reconcile peak", &label, measured.peak, estimate);
    report.bound(
        "reconcile retained",
        &label,
        measured.retained,
        ledger.deep_size() + interns() - interns_before,
    );
    drop(ledger);
}

/// Measures the Rust heap each source decoder holds once it starts, and a scan's peak
/// while it reads a 3.5 MiB line, against the worker slot's components. zstd's own
/// buffers are C allocations, which `ZSTD_sizeof_DCtx` reports instead.
fn check_decoders(report: &mut Report) {
    let line = "y".repeat(7 << 19);
    let plain = format!("{{\"a\":1}}\n{line}\n");
    let mut gzip = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
    gzip.write_all(plain.as_bytes()).expect("gzip encodes");
    let gzip = gzip.finish().expect("gzip finishes");
    let mut zstd = zstd::stream::write::Encoder::new(Vec::new(), 1).expect("zstd encoder");
    zstd.window_log(23).expect("an 8 MiB window");
    zstd.write_all(plain.as_bytes()).expect("zstd encodes");
    let zstd = zstd.finish().expect("zstd finishes");
    let read_buffer = allocation(model::READ_BUFFER) + allocation(model::READER_STATE);
    let zstd_input = allocation(zstd::zstd_safe::DCtx::in_size() as u64);
    for (name, representation, bytes, estimate) in [
        ("plain", Representation::Plain, plain.as_bytes(), read_buffer),
        ("gzip", Representation::Gzip, gzip.as_slice(), read_buffer + model::GZIP_DECODER),
        ("zstd", Representation::Zstd, zstd.as_slice(), read_buffer + zstd_input),
    ] {
        let ((), measured) = measure(|| {
            let mut reader = decode(bytes, representation).expect("the decoder starts");
            let first = reader.fill_buf().expect("the stream decodes").len();
            reader.consume(first);
        });
        report.bound("decoder heap", name, measured.peak, estimate);
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
    drop(entry);
    // The line grows inside the slot's line allowance, without its parse-owned copy.
    let estimate = read_buffer + model::SLOT_LINE_CAPACITY * 3 / 2 + (64 << 10);
    report.bound("scan heap", "3.5 MiB line", measured.peak, estimate);

    let context = zstd::zstd_safe::DCtx::create();
    report.bound("zstd context", "ZSTD_sizeof_DCtx", context.sizeof() as u64, model::ZSTD_CONTEXT);
    let mut stream = zstd::zstd_safe::DCtx::create();
    let mut output = vec![0_u8; 1 << 16];
    let mut input = zstd::zstd_safe::InBuffer::around(&zstd);
    let mut out = zstd::zstd_safe::OutBuffer::around(&mut output[..]);
    stream.decompress_stream(&mut out, &mut input).expect("a frame decodes");
    report.bound(
        "zstd stream",
        "8 MiB window",
        stream.sizeof() as u64 + zstd_input,
        model::ZSTD_DECODER,
    );
}

/// Measures the query documents over two ingests against the per-request, per-row and
/// rendering costs, and reports the measured costs.
fn check_query(report: &mut Report, case: &str, sources: &[(Agent, &Ingested)]) {
    let mut index = SessionIndex::default();
    for (agent, ingested) in sources {
        index.add(*agent, ingested).expect("the index adds");
    }
    let query_sources: Vec<QuerySource<'_>> =
        sources.iter().map(|(agent, ingested)| QuerySource { agent: *agent, ingested }).collect();
    let timezone = ResolvedTimeZone::resolve(Some("UTC")).expect("UTC resolves");
    let requests: u64 =
        sources.iter().map(|(_, ingested)| ingested.ledger.requests.len() as u64).sum();
    let selected = BTreeSet::new();
    let metadata = || QueryMetadata::new("report", "all", Scope::SelfOnly, &timezone);
    let ((rows, rendered), measured) = measure(|| {
        let document =
            query::report(&query_sources, &index, &selected, true, metadata(), &BTreeSet::new())
                .expect("the report queries");
        let rows: usize = document.breakdowns.values().map(Vec::len).sum();
        (rows, serde_json::to_string(&document).expect("the report renders").len())
    });
    let estimate = model::query(requests, rows as u64, rendered as u64);
    report.bound("query report", case, measured.peak, estimate);
    let ((rows, rendered), measured) = measure(|| {
        let document =
            query::daily(&query_sources, &selected, true, metadata(), &timezone).expect("daily");
        (document.rows.len(), serde_json::to_string(&document).expect("daily renders").len())
    });
    report.bound(
        "query daily",
        case,
        measured.peak,
        model::query(requests, rows as u64, rendered as u64),
    );
    let ((rows, rendered), measured) = measure(|| {
        let document =
            query::sessions(&query_sources, &index, &selected, true, metadata(), &timezone)
                .expect("sessions");
        (document.rows.len(), serde_json::to_string(&document).expect("sessions render").len())
    });
    report.bound(
        "query sessions",
        case,
        measured.peak,
        model::query(requests, rows as u64, rendered as u64),
    );
    report.note(
        "query sessions",
        case,
        format!(
            "{requests} requests, {rows} rows, {rendered} rendered bytes: {} bytes per row above requests and rendering",
            measured.peak.saturating_sub(requests * model::QUERY_PER_REQUEST + 3 * rendered as u64) / (rows.max(1) as u64)
        ),
    );
}

fn main() {
    let mut report = Report::default();
    let started = std::time::Instant::now();
    for agent in [Agent::Codex, Agent::Claude] {
        let structure = model::request_structure(agent);
        report.note(
            "model",
            agent.token(),
            format!(
                "κ {} (construction {}, grouping {}, finalize {}); decoded record {}; key bound {}",
                structure.kappa(),
                structure.construction,
                structure.grouping,
                structure.finalize,
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
    for (agent, dialect) in [(Agent::Claude, "claude-project"), (Agent::Codex, "codex-rollout")] {
        for case in fixtures(dialect) {
            let name = case
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default();
            check_ingest(&mut report, agent, &name, &case);
        }
    }
    report.note("time", "fixtures", format!("{} ms", started.elapsed().as_millis()));
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
    report.note("time", "generated ingests", format!("{} ms", started.elapsed().as_millis()));
    for agent in [Agent::Codex, Agent::Claude] {
        // Warm the synthetic names and identities first.
        drop(reconcile(synthetic_input(agent, 10), &LatestRevision));
        for observations in [1_000, 8_000] {
            check_reconcile(&mut report, agent, observations);
        }
    }
    report.note("time", "reconcile", format!("{} ms", started.elapsed().as_millis()));
    for (sessions, records) in [(4, 200), (100, 4)] {
        let claude = claude_corpus(sessions, records, 0);
        let codex = codex_corpus(sessions, records, 50, 0);
        let claude = ingest(Agent::Claude, claude.path(), 1);
        let codex = ingest(Agent::Codex, codex.path(), 1);
        let case = format!("generated {sessions}×{records} both agents");
        check_query(&mut report, &case, &[(Agent::Claude, &claude), (Agent::Codex, &codex)]);
    }
    report.note("time", "query", format!("{} ms", started.elapsed().as_millis()));
    check_decoders(&mut report);
    let mut out = String::new();
    writeln!(out, "memory model harness ({} ms)", started.elapsed().as_millis()).expect("writes");
    for line in &report.lines {
        writeln!(out, "{line}").expect("writes");
    }
    print!("{out}");
    assert!(
        report.violations.is_empty(),
        "live heap exceeded the model:\n{}",
        report.violations.join("\n")
    );
}
