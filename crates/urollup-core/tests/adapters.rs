//! End-to-end adapter tests over the frozen public dialect fixtures.

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde_json::Value;
use urollup_core::accounting::totals::{LedgerTotals, ledger_totals, selection_totals};
use urollup_core::adapters::AdapterError;
use urollup_core::adapters::claude_project::ingest_root;
use urollup_core::adapters::codex_rollout::ingest_root as ingest_codex;
use urollup_core::ledger::entities::{Confidence, ProviderLimitObservation, RelationshipKind};
use urollup_core::ledger::scope::IdentityBasis;
use urollup_core::ledger::tokens::TokenMeasures;
use urollup_core::selection::{Agent, Scope, SelectionQuery, SessionIndex};

fn fixture(case: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/claude-project").join(case)
}

fn codex_fixture(case: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/codex-rollout").join(case)
}

fn assert_request_counts(totals: &LedgerTotals, expected: &Value, name: &str) {
    let requests = &expected["totals"]["requests"];
    let count = |field: &str| {
        requests[field].as_u64().expect("fixture request count must be an unsigned integer")
    };
    assert_eq!(totals.total.requests, count("unique"), "{name}: requests");
    assert_eq!(totals.owned.requests, count("owned"), "{name}: owned");
    assert_eq!(totals.ambiguous.requests, count("ambiguous"), "{name}: ambiguous");
    assert_eq!(totals.unknown.requests, count("unknown"), "{name}: unknown");
    assert_eq!(
        totals.unresolved.requests,
        expected["totals"]["unresolved"]["requests"]
            .as_u64()
            .expect("fixture unresolved request count must be an unsigned integer"),
        "{name}: unresolved"
    );
}

fn assert_limit_observations(actual: &[ProviderLimitObservation], expected: &Value, name: &str) {
    let expected = expected["limit_observations"]
        .as_array()
        .expect("fixture limit observations must be an array");
    assert_eq!(actual.len(), expected.len(), "{name}: limit observations");
    for (index, (actual, expected)) in actual.iter().zip(expected).enumerate() {
        assert_eq!(
            actual.limit_name.as_ref().map(|name| name.as_str()),
            expected["limit"].as_str(),
            "{name}: limit {index}"
        );
        assert_eq!(
            actual.window.as_ref().map(|name| name.as_str()),
            expected["window"].as_str(),
            "{name}: window {index}"
        );
        let native: Value =
            serde_json::from_str(actual.native.as_str()).expect("native fields are JSON");
        assert_eq!(native, expected["native"], "{name}: native limit fields {index}");
    }
}

fn assert_diagnostic_counts(
    actual: &[urollup_core::ledger::diagnostics::Diagnostic],
    expected: &Value,
    name: &str,
) {
    let mut actual_counts = BTreeMap::new();
    for diagnostic in actual {
        *actual_counts.entry(diagnostic.code.token()).or_insert(0_u64) += diagnostic.occurrences;
    }
    let mut expected_counts = BTreeMap::new();
    for diagnostic in
        expected["diagnostics"].as_array().expect("fixture diagnostics must be an array")
    {
        let code = diagnostic["code"].as_str().expect("diagnostic code must be a string");
        let references = diagnostic["refs"].as_array().expect("diagnostic refs must be an array");
        let count = u64::try_from(references.len()).expect("diagnostic count fits in u64");
        *expected_counts.entry(code).or_insert(0_u64) += count.max(1);
    }
    assert_eq!(actual_counts, expected_counts, "{name}: diagnostic occurrence counts");
}

#[test]
fn adapters_reject_unreadable_discovery_roots() {
    let root = tempfile::NamedTempFile::new().unwrap();

    for result in [ingest_root(root.path()), ingest_codex(root.path())] {
        assert!(matches!(result, Err(AdapterError::UnreadablePath { .. })));
    }
}

#[test]
fn claude_rejects_malformed_subagent_metadata() {
    let root = tempfile::tempdir().unwrap();
    let subagents = root.path().join("projects/example/session/subagents");
    std::fs::create_dir_all(&subagents).unwrap();
    let fixture = fixture("workflow-subagents").join(
        "projects/-Users-example-project/00000000-0000-4000-8000-001100000001/subagents/agent-a0000000000110001.jsonl",
    );
    std::fs::copy(fixture, subagents.join("agent-example.jsonl")).unwrap();
    std::fs::write(subagents.join("agent-example.meta.json"), b"{").unwrap();

    assert!(matches!(ingest_root(root.path()), Err(AdapterError::MetadataParse { .. })));
}

#[test]
fn release_discovery_keeps_the_ledger_after_session_index_copies_threads() {
    let ingested = ingest_root(&fixture("brief-double-counting")).unwrap();
    let requests = ingested.ledger.requests.len();
    let threads = ingested.threads.len();
    assert!(requests > 0 && threads > 0);

    let mut index = SessionIndex::default();
    index.add(Agent::Claude, &ingested).unwrap();
    assert_eq!(index.sessions().count(), threads);

    let mut ingested = ingested;
    ingested.release_discovery();
    assert!(ingested.manifest.entries.is_empty());
    assert!(ingested.sources.is_empty());
    assert!(ingested.threads.is_empty());
    assert!(ingested.relationships.is_empty());
    assert_eq!(ingested.ledger.requests.len(), requests);
    assert_eq!(index.sessions().count(), threads);
}

#[test]
fn claude_keeps_sessions_without_usage_records() {
    let root = tempfile::tempdir().unwrap();
    let projects = root.path().join("projects/example");
    std::fs::create_dir_all(&projects).unwrap();
    std::fs::write(
        projects.join("session-without-usage.jsonl"),
        concat!(
            r#"{"type":"user","sessionId":"session-without-usage","cwd":"/workspace/example","version":"2.1.270"}"#,
            "\n"
        ),
    )
    .unwrap();

    let ingested = ingest_root(root.path()).unwrap();

    assert!(ingested.ledger.requests.is_empty());
    assert_eq!(ingested.threads.len(), 1);
    let thread = ingested.threads.values().next().unwrap();
    assert_eq!(thread.project.value().map(String::as_str), Some("example"));
    assert_eq!(ingested.sources[0].dialect_version.value().map(String::as_str), Some("2.1.270"));
}

#[test]
fn claude_block_records_are_selected_once() {
    let ingested = ingest_root(&fixture("brief-double-counting")).unwrap();
    let totals = ledger_totals(&ingested.ledger).unwrap();

    assert_eq!(totals.total.requests, 1);
    assert_eq!(totals.total.tokens.uncached_input, Some(3));
    assert_eq!(totals.total.tokens.cache_read, Some(40_000));
    assert_eq!(totals.total.tokens.output, Some(600));
    assert_eq!(ingested.ledger.coverage.copies, 1);
    assert_eq!(ingested.manifest.entries.len(), 2);
}

#[test]
fn claude_inline_sidechains_are_fallback_children_in_descendant_scope() {
    let ingested = ingest_root(&fixture("inline-sidechains")).unwrap();
    let main_session = "00000000-0000-4000-8000-001500000001";
    let fallback_threads: Vec<_> = ingested
        .threads
        .values()
        .filter(|thread| thread.basis == IdentityBasis::Fallback)
        .collect();

    assert_eq!(fallback_threads.len(), 2);
    assert!(fallback_threads.iter().all(|thread| thread.native_key.is_empty()));
    assert!(
        ingested.relationships.iter().all(|edge| edge.kind == RelationshipKind::InlineSidechain
            && edge.confidence == Confidence::Inferred
            && edge.evidence.len() == 2)
    );

    let mut index = SessionIndex::default();
    index.add(Agent::Claude, &ingested).unwrap();
    let main = index.resolve(main_session.as_ref()).unwrap();
    let descendants = index
        .select(&SelectionQuery {
            sessions: vec![main_session.into()],
            ..SelectionQuery::default()
        })
        .unwrap();
    assert_eq!(descendants.len(), 3);
    assert_eq!(selection_totals(&ingested.ledger, &descendants).unwrap().counted.requests, 6);

    let own = index
        .select(&SelectionQuery {
            sessions: vec![main.to_string().into()],
            scope: Some(Scope::SelfOnly),
            ..SelectionQuery::default()
        })
        .unwrap();
    assert_eq!(own.len(), 1);
    assert_eq!(selection_totals(&ingested.ledger, &own).unwrap().counted.requests, 3);
}

#[test]
fn claude_advisor_usage_keeps_its_model_breakdown() {
    let ingested = ingest_root(&fixture("advisor-iterations")).unwrap();
    let request = ingested
        .ledger
        .requests
        .values()
        .find(|request| {
            request.usage.as_ref().is_some_and(|usage| usage.revision.model_usage.len() == 2)
        })
        .unwrap();
    let model_usage: Vec<_> = request.usage.as_ref().unwrap().revision.model_usage.iter().collect();

    assert_eq!(model_usage[0].model.as_ref().unwrap().name, "claude-sonnet-4-5");
    assert_eq!(TokenMeasures::from(model_usage[0].usage).output, Some(530));
    assert_eq!(model_usage[1].model.as_ref().unwrap().name, "claude-opus-4-5");
    assert_eq!(TokenMeasures::from(model_usage[1].usage).output, Some(7_200));
}

#[test]
fn every_claude_fixture_matches_its_reconciled_totals() {
    let dialect = fixture("");
    let mut cases: Vec<_> = std::fs::read_dir(&dialect)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.join("expected.json").is_file())
        .collect();
    cases.sort();

    for case in cases {
        let expected: Value =
            serde_json::from_slice(&std::fs::read(case.join("expected.json")).unwrap()).unwrap();
        let ingested = ingest_root(&case).unwrap();
        let totals = ledger_totals(&ingested.ledger).unwrap();
        let name = case.file_name().unwrap().to_string_lossy();
        let token = |field: &str| expected["totals"]["tokens"][field].as_u64();

        assert_request_counts(&totals, &expected, &name);
        assert_eq!(totals.total.tokens.uncached_input, token("uncached_input"), "{name}: input");
        assert_eq!(totals.total.tokens.cache_read, token("cache_read"), "{name}: cache read");
        assert_eq!(
            totals.total.tokens.cache_write().unwrap(),
            token("cache_write"),
            "{name}: cache write"
        );
        assert_eq!(totals.total.tokens.output, token("output"), "{name}: output");
        assert_eq!(totals.total.tokens.reasoning, token("reasoning"), "{name}: reasoning");
        assert_eq!(
            ingested.ledger.coverage.copies,
            expected["copies"].as_array().unwrap().len() as u64,
            "{name}: copies"
        );
    }
}

#[test]
fn every_claude_fixture_matches_metadata_counts() {
    let mut cases: Vec<_> = std::fs::read_dir(fixture(""))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.join("expected.json").is_file())
        .collect();
    cases.sort();

    for case in cases {
        let expected: Value =
            serde_json::from_slice(&std::fs::read(case.join("expected.json")).unwrap()).unwrap();
        let ingested = ingest_root(&case).unwrap();
        let name = case.file_name().unwrap().to_string_lossy();

        assert!(
            ingested.sources.iter().all(|source| source.dialect_version.value().is_some()),
            "{name}: source versions"
        );

        assert_eq!(
            ingested.threads.len(),
            expected["threads"].as_array().unwrap().len(),
            "{name}: threads"
        );
        assert_eq!(
            ingested.relationships.len(),
            expected["threads"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|thread| !thread["parent"].is_null())
                .count(),
            "{name}: relationships"
        );
        assert_eq!(
            ingested.ledger.diagnostics.len(),
            expected["diagnostics"].as_array().unwrap().len(),
            "{name}: diagnostics"
        );
        let mut actual_codes: Vec<_> =
            ingested.ledger.diagnostics.iter().map(|diagnostic| diagnostic.code.token()).collect();
        let mut expected_codes: Vec<_> = expected["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .map(|diagnostic| diagnostic["code"].as_str().unwrap())
            .collect();
        actual_codes.sort_unstable();
        expected_codes.sort_unstable();
        assert_eq!(actual_codes, expected_codes, "{name}: diagnostic codes");
        assert_diagnostic_counts(&ingested.ledger.diagnostics, &expected, &name);
        assert_limit_observations(&ingested.limit_observations, &expected, &name);
    }
}

/// Copies a Claude fixture with its main session names permuted into reverse order.
///
/// Every occurrence of a session ID, in paths and in record contents, is renamed
/// consistently, so the copy describes the same history under different file names.
/// The permutation reuses the fixture's own names, which keeps record lengths and offsets.
fn copy_with_reversed_session_names(case: &std::path::Path, destination: &std::path::Path) {
    let projects = case.join("projects");
    let mut sessions = Vec::new();
    for project in std::fs::read_dir(&projects).expect("fixture projects are readable") {
        for entry in
            std::fs::read_dir(project.expect("project entry").path()).expect("project is readable")
        {
            let path = entry.expect("session entry").path();
            if path.extension().is_some_and(|extension| extension == "jsonl") {
                sessions.push(
                    path.file_stem()
                        .expect("session file has a stem")
                        .to_string_lossy()
                        .into_owned(),
                );
            }
        }
    }
    sessions.sort();
    sessions.dedup();
    let renamed: BTreeMap<_, _> =
        sessions.iter().cloned().zip(sessions.iter().rev().cloned()).collect();
    let rename = |text: &str| {
        let mut text = text.to_owned();
        for (index, session) in sessions.iter().enumerate() {
            text = text.replace(session, &format!("\u{0}{index}\u{0}"));
        }
        for (index, session) in sessions.iter().enumerate() {
            text = text.replace(&format!("\u{0}{index}\u{0}"), &renamed[session]);
        }
        text
    };

    let mut pending = vec![projects.clone()];
    while let Some(directory) = pending.pop() {
        for entry in std::fs::read_dir(&directory).expect("fixture directory is readable") {
            let path = entry.expect("fixture entry").path();
            if path.is_dir() {
                pending.push(path);
                continue;
            }
            let relative = path
                .strip_prefix(case)
                .expect("fixture file is under its case")
                .to_string_lossy()
                .into_owned();
            let target = destination.join(rename(&relative));
            std::fs::create_dir_all(target.parent().expect("target has a parent"))
                .expect("target directory is writable");
            let contents = std::fs::read_to_string(&path).expect("fixture file is UTF-8 text");
            std::fs::write(target, rename(&contents)).expect("renamed fixture is writable");
        }
    }
}

#[test]
fn claude_results_do_not_depend_on_session_file_names() {
    let mut cases: Vec<_> = std::fs::read_dir(fixture(""))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.join("expected.json").is_file())
        .collect();
    cases.sort();

    for case in cases {
        let name = case.file_name().unwrap().to_string_lossy().into_owned();
        let reversed = tempfile::tempdir().unwrap();
        copy_with_reversed_session_names(&case, reversed.path());

        let original = ingest_root(&case).unwrap();
        let permuted = ingest_root(reversed.path()).unwrap();
        assert_eq!(
            ledger_totals(&permuted.ledger).unwrap(),
            ledger_totals(&original.ledger).unwrap(),
            "{name}: totals under reversed session names"
        );
        assert_eq!(
            permuted.ledger.coverage.copies, original.ledger.coverage.copies,
            "{name}: copies under reversed session names"
        );
        let occurrences = |ingested: &urollup_core::adapters::Ingested| {
            let mut counts = BTreeMap::new();
            for diagnostic in &ingested.ledger.diagnostics {
                *counts.entry(diagnostic.code.token()).or_insert(0_u64) += diagnostic.occurrences;
            }
            counts
        };
        assert_eq!(
            occurrences(&permuted),
            occurrences(&original),
            "{name}: diagnostics under reversed session names"
        );
    }
}

#[test]
fn codex_response_usage_is_normalized_and_copied_history_is_excluded() {
    let ingested = ingest_codex(&codex_fixture("token-usage-records")).unwrap();
    let totals = ledger_totals(&ingested.ledger).unwrap();

    assert_eq!(totals.total.requests, 4);
    assert_eq!(totals.total.tokens.uncached_input, Some(11_000));
    assert_eq!(totals.total.tokens.cache_read, Some(59_000));
    assert_eq!(totals.total.tokens.output, Some(3_200));
    assert_eq!(totals.total.tokens.reasoning, Some(950));
    assert_eq!(ingested.ledger.coverage.copies, 5);
}

#[test]
fn every_codex_fixture_matches_its_reconciled_totals() {
    let dialect = codex_fixture("");
    let mut cases: Vec<_> = std::fs::read_dir(&dialect)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.join("expected.json").is_file())
        .collect();
    cases.sort();

    for case in cases {
        let expected: Value =
            serde_json::from_slice(&std::fs::read(case.join("expected.json")).unwrap()).unwrap();
        let ingested = ingest_codex(&case).unwrap();
        let totals = ledger_totals(&ingested.ledger).unwrap();
        let name = case.file_name().unwrap().to_string_lossy();
        let token = |field: &str| expected["totals"]["tokens"][field].as_u64();

        assert_request_counts(&totals, &expected, &name);
        assert_eq!(totals.total.tokens.uncached_input, token("uncached_input"), "{name}: input");
        assert_eq!(totals.total.tokens.cache_read, token("cache_read"), "{name}: cache read");
        assert_eq!(
            totals.total.tokens.cache_write().unwrap(),
            token("cache_write"),
            "{name}: cache write"
        );
        assert_eq!(totals.total.tokens.output, token("output"), "{name}: output");
        assert_eq!(totals.total.tokens.reasoning, token("reasoning"), "{name}: reasoning");
        assert_eq!(
            ingested.ledger.coverage.copies,
            expected["copies"].as_array().unwrap().len() as u64,
            "{name}: copies"
        );
    }
}

#[test]
fn every_codex_fixture_matches_metadata_counts() {
    let mut cases: Vec<_> = std::fs::read_dir(codex_fixture(""))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.join("expected.json").is_file())
        .collect();
    cases.sort();

    for case in cases {
        let expected: Value =
            serde_json::from_slice(&std::fs::read(case.join("expected.json")).unwrap()).unwrap();
        let ingested = ingest_codex(&case).unwrap();
        let name = case.file_name().unwrap().to_string_lossy();

        assert!(
            ingested.sources.iter().all(|source| source.dialect_version.value().is_some()),
            "{name}: source versions"
        );

        assert_eq!(
            ingested.threads.len(),
            expected["threads"].as_array().unwrap().len(),
            "{name}: threads"
        );
        assert_eq!(
            ingested.relationships.len(),
            expected["threads"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|thread| !thread["parent"].is_null())
                .count(),
            "{name}: relationships"
        );
        assert_eq!(
            ingested.ledger.diagnostics.len(),
            expected["diagnostics"].as_array().unwrap().len(),
            "{name}: diagnostics"
        );
        let mut actual_codes: Vec<_> =
            ingested.ledger.diagnostics.iter().map(|diagnostic| diagnostic.code.token()).collect();
        let mut expected_codes: Vec<_> = expected["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .map(|diagnostic| diagnostic["code"].as_str().unwrap())
            .collect();
        actual_codes.sort_unstable();
        expected_codes.sort_unstable();
        assert_eq!(actual_codes, expected_codes, "{name}: diagnostic codes");
        assert_diagnostic_counts(&ingested.ledger.diagnostics, &expected, &name);
        assert_limit_observations(&ingested.limit_observations, &expected, &name);
    }
}

#[test]
fn claude_largest_u64_block_index_wins_equal_output_ties() {
    let root = tempfile::tempdir().unwrap();
    let projects = root.path().join("projects/example");
    std::fs::create_dir_all(&projects).unwrap();
    let mut lines = String::new();
    for (block, input) in [(u64::MAX, 100), (1, 1)] {
        let record = serde_json::json!({
            "type": "assistant",
            "sessionId": "boundary-session",
            "requestId": "boundary-request",
            "apiBlockIndex": block,
            "message": {
                "id": "boundary-message",
                "model": "claude-test",
                "usage": {"input_tokens": input, "output_tokens": 10}
            }
        });
        lines.push_str(&record.to_string());
        lines.push('\n');
    }
    std::fs::write(projects.join("boundary-session.jsonl"), lines).unwrap();
    let ingested = ingest_root(root.path()).unwrap();
    assert_eq!(ingested.ledger.requests.len(), 1);
    let request = ingested.ledger.requests.values().next().unwrap();
    let selected = request.usage.as_ref().unwrap();
    let usage = TokenMeasures::from(selected.revision.usage);
    assert_eq!(usage.uncached_input, Some(100));
    assert_eq!(usage.output, Some(10));
}

/// A way to store a fixture's `.jsonl` files compressed.
#[derive(Clone, Copy, Debug)]
enum Compression {
    Gzip,
    Zstd,
}

impl Compression {
    fn suffix(self) -> &'static str {
        match self {
            Self::Gzip => ".gz",
            Self::Zstd => ".zst",
        }
    }

    fn encode(self, contents: &[u8]) -> Vec<u8> {
        use std::io::Write as _;
        match self {
            Self::Gzip => {
                let mut encoder =
                    flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
                encoder.write_all(contents).expect("gzip encodes in memory");
                encoder.finish().expect("gzip finishes in memory")
            }
            Self::Zstd => zstd::stream::encode_all(contents, 3).expect("zstd encodes in memory"),
        }
    }
}

/// Copies a fixture case, storing every `.jsonl` file that has no compressed twin as
/// `.jsonl.gz` or `.jsonl.zst` and every other file unchanged. Returns how many files were
/// compressed.
fn copy_compressed(
    case: &std::path::Path,
    destination: &std::path::Path,
    compression: Compression,
) -> usize {
    let mut compressed = 0;
    let mut pending = vec![case.to_owned()];
    while let Some(directory) = pending.pop() {
        for entry in std::fs::read_dir(&directory).expect("fixture directory is readable") {
            let path = entry.expect("fixture entry").path();
            if path.is_dir() {
                pending.push(path);
                continue;
            }
            let target = destination.join(path.strip_prefix(case).expect("file is under its case"));
            std::fs::create_dir_all(target.parent().expect("target has a parent"))
                .expect("target directory is writable");
            let name = path.file_name().expect("fixture file has a name").to_string_lossy();
            let has_twin = [".zst", ".gz"]
                .iter()
                .any(|suffix| path.with_file_name(format!("{name}{suffix}")).exists());
            let contents = std::fs::read(&path).expect("fixture file is readable");
            if name.ends_with(".jsonl") && !has_twin {
                let target = target.with_file_name(format!("{name}{}", compression.suffix()));
                std::fs::write(target, compression.encode(&contents)).expect("copy is writable");
                compressed += 1;
            } else {
                std::fs::write(target, contents).expect("copy is writable");
            }
        }
    }
    compressed
}

#[test]
fn every_fixture_ingests_identically_from_gzip_and_zstd_files() {
    type Ingest =
        fn(
            &std::path::Path,
        )
            -> Result<urollup_core::adapters::Ingested, urollup_core::adapters::AdapterError>;
    for (dialect, ingest) in
        [("claude-project", ingest_root as Ingest), ("codex-rollout", ingest_codex as Ingest)]
    {
        let mut cases: Vec<_> = std::fs::read_dir(fixture("").join("..").join(dialect))
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| path.join("expected.json").is_file())
            .collect();
        cases.sort();
        assert!(!cases.is_empty(), "{dialect} has fixtures");
        let mut compressed_files = 0;

        for case in cases {
            let name = case.file_name().unwrap().to_string_lossy().into_owned();
            let original = ingest(&case).unwrap();
            for compression in [Compression::Gzip, Compression::Zstd] {
                let copy = tempfile::tempdir().unwrap();
                // A case that already holds compressed twins, such as zst-twin, keeps them.
                compressed_files += copy_compressed(&case, copy.path(), compression);

                let decoded = ingest(copy.path()).unwrap();
                let label = format!("{dialect}/{name} as {compression:?}");
                assert_eq!(decoded.ledger, original.ledger, "{label}: ledger");
                assert_eq!(decoded.threads, original.threads, "{label}: threads");
                assert_eq!(decoded.relationships, original.relationships, "{label}: relationships");
                assert_eq!(
                    decoded.limit_observations, original.limit_observations,
                    "{label}: limit observations"
                );
                assert_eq!(
                    decoded.manifest.entries.len(),
                    original.manifest.entries.len(),
                    "{label}: one source per logical file"
                );
            }
        }
        assert!(compressed_files > 0, "{dialect}: sources were compressed");
    }
}

#[test]
fn a_source_deleted_after_discovery_leaves_partial_totals_with_a_diagnostic() {
    use urollup_core::accounting::totals::{Completeness, PartialReason};
    use urollup_core::adapters::claude_project::ingest_discovery;
    use urollup_core::sources::roots::discover;

    let root = tempfile::tempdir().unwrap();
    copy_compressed(&fixture("brief-double-counting"), root.path(), Compression::Gzip);
    let projects = root.path().join("projects");
    let discovery = discover(std::slice::from_ref(&projects));
    let complete = ingest_discovery(discovery.clone(), true).unwrap();
    assert_eq!(ledger_totals(&complete.ledger).unwrap().completeness, Completeness::Complete);

    // Claude Code expires old transcripts; one disappears between discovery and reading.
    let (removed, _) = discovery.sources[0].files.primary().unwrap();
    std::fs::remove_file(removed).unwrap();
    let partial = ingest_discovery(discovery, true).expect("a vanished source is not fatal");

    let totals = ledger_totals(&partial.ledger).unwrap();
    assert!(
        matches!(&totals.completeness, Completeness::Partial(reasons)
            if reasons.contains(&PartialReason::UnobservedGap)),
        "{:?}",
        totals.completeness
    );
    assert!(totals.total.requests < ledger_totals(&complete.ledger).unwrap().total.requests);
    let incomplete: Vec<_> = partial
        .ledger
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code.token() == "source-incomplete")
        .collect();
    assert_eq!(incomplete.len(), 1, "{:?}", partial.ledger.diagnostics);
    assert_eq!(
        incomplete[0].detail,
        "a Claude Code transcript could not be read completely: vanished"
    );
}

#[test]
fn codex_usage_takes_its_model_from_its_own_turn_not_the_root_turn() {
    // A multi-agent subagent's records name the parent's turn as their root, so only
    // `turn_id` matches the subagent's own `turn_context`.
    let home = tempfile::tempdir().unwrap();
    let thread = "019f0000-0000-7000-8000-00aa00000001";
    let day = home.path().join("sessions/2026/10/01");
    std::fs::create_dir_all(&day).unwrap();
    let lines = [
        format!(
            r#"{{"timestamp":"2026-10-01T07:00:00.000Z","type":"session_meta","payload":{{"id":"{thread}","cwd":"/work/project","cli_version":"0.160.0"}}}}"#
        ),
        r#"{"timestamp":"2026-10-01T07:00:01.000Z","type":"turn_context","payload":{"turn_id":"turn-own","root_turn_id":"turn-parent","model":"gpt-test","effort":"high"}}"#.to_owned(),
        format!(
            r#"{{"timestamp":"2026-10-01T07:00:02.000Z","type":"token_usage_record","payload":{{"thread_id":"{thread}","response_id":"resp-1","turn_id":"turn-own","root_turn_id":"turn-parent","usage":{{"input_tokens":100,"cached_input_tokens":40,"cache_write_input_tokens":0,"output_tokens":10,"reasoning_output_tokens":2,"total_tokens":110}}}}}}"#
        ),
        // A record without its own turn still falls back to the root turn.
        r#"{"timestamp":"2026-10-01T07:00:03.000Z","type":"turn_context","payload":{"turn_id":"turn-legacy","model":"gpt-legacy","effort":"low"}}"#.to_owned(),
        format!(
            r#"{{"timestamp":"2026-10-01T07:00:04.000Z","type":"token_usage_record","payload":{{"thread_id":"{thread}","response_id":"resp-2","root_turn_id":"turn-legacy","usage":{{"input_tokens":50,"cached_input_tokens":0,"cache_write_input_tokens":0,"output_tokens":5,"reasoning_output_tokens":0,"total_tokens":55}}}}}}"#
        ),
    ];
    std::fs::write(
        day.join(format!("rollout-2026-10-01T00-00-00-{thread}.jsonl")),
        lines.join("\n") + "\n",
    )
    .unwrap();

    let ingested = ingest_codex(home.path()).unwrap();
    let mut models: Vec<(String, String)> = ingested
        .ledger
        .requests
        .values()
        .map(|request| {
            let name = |value: Option<&str>| value.unwrap_or("unknown").to_owned();
            (
                name(request.model.as_ref().map(|model| model.name.as_str())),
                name(request.effort.as_ref().map(|effort| effort.as_str())),
            )
        })
        .collect();
    models.sort();
    assert_eq!(
        models,
        vec![
            ("gpt-legacy".to_owned(), "low".to_owned()),
            ("gpt-test".to_owned(), "high".to_owned()),
        ]
    );
}

#[test]
fn a_codex_total_lowered_at_compaction_counts_only_its_own_request() {
    // Codex lowers its running total at compaction instead of restarting it. The
    // decreasing token_count's last_token_usage is that request's usage; its new total
    // is the session so far.
    let home = tempfile::tempdir().unwrap();
    let thread = "019f0000-0000-7000-8000-00bb00000001";
    let day = home.path().join("sessions/2026/10/01");
    std::fs::create_dir_all(&day).unwrap();
    let count = |second: u32, total: [u64; 4], last: [u64; 4]| {
        let usage = |[input, cached, output, reasoning]: [u64; 4]| {
            format!(
                r#"{{"input_tokens":{input},"cached_input_tokens":{cached},"cache_write_input_tokens":0,"output_tokens":{output},"reasoning_output_tokens":{reasoning},"total_tokens":{}}}"#,
                input + output
            )
        };
        format!(
            r#"{{"timestamp":"2026-10-01T07:00:{second:02}.000Z","type":"event_msg","payload":{{"type":"token_count","info":{{"total_token_usage":{},"last_token_usage":{},"model_context_window":272000}},"rate_limits":null}}}}"#,
            usage(total),
            usage(last)
        )
    };
    let lines = [
        format!(
            r#"{{"timestamp":"2026-10-01T07:00:00.000Z","type":"session_meta","payload":{{"id":"{thread}","cwd":"/work/project","cli_version":"0.150.0"}}}}"#
        ),
        r#"{"timestamp":"2026-10-01T07:00:01.000Z","type":"turn_context","payload":{"turn_id":"turn-1","model":"gpt-test","effort":"medium"}}"#.to_owned(),
        count(2, [3_000, 0, 300, 0], [3_000, 0, 300, 0]),
        count(3, [7_000, 2_500, 800, 200], [4_000, 2_500, 500, 200]),
        r#"{"timestamp":"2026-10-01T07:00:04.000Z","type":"compacted","payload":{"message":""}}"#.to_owned(),
        count(5, [6_000, 2_000, 900, 200], [1_000, 500, 100, 0]),
        count(6, [7_500, 3_000, 1_000, 200], [1_500, 1_000, 100, 0]),
    ];
    std::fs::write(
        day.join(format!("rollout-2026-10-01T00-00-00-{thread}.jsonl")),
        lines.join("\n") + "\n",
    )
    .unwrap();

    let ingested = ingest_codex(home.path()).unwrap();
    let totals = ledger_totals(&ingested.ledger).unwrap();
    assert_eq!(totals.total.requests, 4);
    let tokens = totals.total.tokens;
    assert_eq!(tokens.uncached_input, Some(3_000 + 1_500 + 500 + 500));
    assert_eq!(tokens.cache_read, Some(2_500 + 500 + 1_000));
    assert_eq!(tokens.output, Some(300 + 500 + 100 + 100));
    let resets = ingested
        .ledger
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code.token() == "codex-counter-epoch-reset")
        .count();
    assert_eq!(resets, 1, "the decrease still opens a new epoch with a diagnostic");
}
