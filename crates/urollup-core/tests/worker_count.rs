//! Worker-count invariance: decoding sources on one thread or on many produces the same
//! ingestion, because per-source results merge in discovery order before normalization.

use std::fs;
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use urollup_core::adapters::{AdapterError, Ingested, claude_project, codex_rollout};
use urollup_core::sources::roots::discover;

/// Worker counts compared against one worker, including more workers than sources.
const WORKER_COUNTS: [usize; 4] = [2, 3, 8, 64];

fn fixtures(dialect: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(dialect)
}

fn fixture_cases(dialect: &str) -> Vec<PathBuf> {
    let mut cases: Vec<PathBuf> = fs::read_dir(fixtures(dialect))
        .expect("the fixture directory is readable")
        .map(|entry| entry.expect("the fixture entry is readable").path())
        .filter(|path| path.is_dir())
        .collect();
    cases.sort();
    assert!(!cases.is_empty(), "{dialect} has fixture cases");
    cases
}

fn workers(count: usize) -> NonZeroUsize {
    NonZeroUsize::new(count).expect("worker counts are positive")
}

/// Clears each source's capture clock, in the manifest and in its source entity: the one
/// field that differs between any two runs, even on one worker.
fn without_capture_times(mut ingested: Ingested) -> Ingested {
    let snapshots = ingested.sources.iter_mut().map(|source| &mut source.snapshot);
    for entry in ingested.manifest.entries.iter_mut().chain(snapshots) {
        entry.cutoff.captured_at = SystemTime::UNIX_EPOCH;
    }
    ingested
}

fn claude(roots: &[PathBuf], count: usize) -> Result<Ingested, AdapterError> {
    claude_project::ingest_discovery_with_workers(discover(roots), true, workers(count))
        .map(without_capture_times)
}

fn codex(roots: &[PathBuf], count: usize) -> Result<Ingested, AdapterError> {
    let roots = codex_rollout::rollout_roots(roots);
    codex_rollout::ingest_discovery_with_workers(discover(&roots), true, workers(count))
        .map(without_capture_times)
}

type Ingest = fn(&[PathBuf], usize) -> Result<Ingested, AdapterError>;

/// Ingests `roots` on one worker and on every other worker count and requires identical
/// results, or identical errors; returns the one-worker result.
fn assert_worker_count_invariant(
    ingest: Ingest,
    roots: &[PathBuf],
    name: &str,
) -> Result<Ingested, String> {
    let sequential = ingest(roots, 1).map_err(|error| error.to_string());
    for count in WORKER_COUNTS {
        let parallel = ingest(roots, count).map_err(|error| error.to_string());
        assert_eq!(parallel, sequential, "{name}: {count} workers differ from one worker");
    }
    sequential
}

fn case_name(case: &Path) -> String {
    case.file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_default()
}

#[test]
fn every_claude_fixture_ingests_identically_on_any_worker_count() {
    let mut multi_source = Vec::new();
    for case in fixture_cases("claude-project") {
        let name = case_name(&case);
        let ingested = assert_worker_count_invariant(claude, std::slice::from_ref(&case), &name)
            .unwrap_or_else(|error| panic!("{name}: {error}"));
        if ingested.manifest.entries.len() > 1 {
            multi_source.push(name);
        }
    }
    for expected in ["gateway-message-id-reuse", "workflow-subagents"] {
        assert!(multi_source.iter().any(|name| name == expected), "{expected} has several sources");
    }
}

#[test]
fn every_codex_fixture_ingests_identically_on_any_worker_count() {
    let mut multi_source = Vec::new();
    for case in fixture_cases("codex-rollout") {
        let name = case_name(&case);
        let ingested = assert_worker_count_invariant(codex, std::slice::from_ref(&case), &name)
            .unwrap_or_else(|error| panic!("{name}: {error}"));
        if ingested.manifest.entries.len() > 1 {
            multi_source.push(name);
        }
    }
    for expected in [
        "archived-rename",
        "legacy-subagent-prefix",
        "legacy-user-fork-counters",
        "paginated-subagent",
    ] {
        assert!(multi_source.iter().any(|name| name == expected), "{expected} has several sources");
    }
}

#[test]
fn all_fixtures_of_a_dialect_ingest_identically_as_one_corpus() {
    // Every case at once gives each worker count many more sources than workers, so
    // sources really finish out of discovery order.
    for (dialect, ingest) in [("claude-project", claude as Ingest), ("codex-rollout", codex)] {
        let cases = fixture_cases(dialect);
        let ingested = assert_worker_count_invariant(ingest, &cases, dialect)
            .unwrap_or_else(|error| panic!("{dialect}: {error}"));
        assert!(ingested.manifest.entries.len() > 16, "{dialect}: the corpus has many sources");
    }
}

#[test]
fn the_first_failing_source_in_discovery_order_is_reported() {
    let root = tempfile::tempdir().expect("a temporary directory");
    let transcript = fixtures("claude-project").join(
        "workflow-subagents/projects/-Users-example-project/00000000-0000-4000-8000-001100000001/subagents/agent-a0000000000110001.jsonl",
    );
    let first_broken = 3;
    for session in 0..16 {
        let subagents =
            root.path().join(format!("projects/example/session-{session:02}/subagents"));
        fs::create_dir_all(&subagents).expect("the subagent directory is created");
        let copy = subagents.join("agent-example.jsonl");
        fs::copy(&transcript, &copy).expect("the transcript is copied");
        if session == first_broken || session == 11 {
            fs::write(subagents.join("agent-example.meta.json"), b"{").expect("a broken sidecar");
        }
        if session == 11 {
            // The later failure is the heaviest source, so parallel workers start it first.
            let mut padded = fs::read(&copy).expect("the copy is readable");
            padded.extend(std::iter::repeat_n(b'\n', 256 * 1024));
            fs::write(&copy, padded).expect("the copy is padded");
        }
    }
    let expected = root.path().join(format!(
        "projects/example/session-{first_broken:02}/subagents/agent-example.meta.json"
    ));

    for count in [1, 2, 8] {
        let error =
            claude(&[root.path().to_owned()], count).expect_err("a broken sidecar fails ingestion");
        let AdapterError::MetadataParse { path, .. } = error else {
            panic!("{count} workers: unexpected error {error}");
        };
        assert_eq!(path, expected, "{count} workers");
    }
}
