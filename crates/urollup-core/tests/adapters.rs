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
            actual.limit_name.as_deref(),
            expected["limit"].as_str(),
            "{name}: limit {index}"
        );
        assert_eq!(actual.window.as_deref(), expected["window"].as_str(), "{name}: window {index}");
        let native: Value = serde_json::from_str(&actual.native).expect("native fields are JSON");
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
    let model_usage = &request.usage.as_ref().unwrap().revision.model_usage;

    assert_eq!(model_usage[0].model.as_ref().unwrap().name, "claude-sonnet-4-5");
    assert_eq!(model_usage[0].usage.output, Some(530));
    assert_eq!(model_usage[1].model.as_ref().unwrap().name, "claude-opus-4-5");
    assert_eq!(model_usage[1].usage.output, Some(7_200));
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
