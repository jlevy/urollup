//! Recorded pricing context must survive ingestion without consulting machine settings.

use std::fmt::Write;
use std::fs;

use urollup_core::adapters::codex_rollout;
use urollup_core::ledger::names::Name;

fn jsonl(lines: &[serde_json::Value]) -> String {
    let mut text = String::new();
    for line in lines {
        writeln!(text, "{line}").expect("writing synthetic JSON into a String cannot fail");
    }
    text
}

#[test]
fn claude_pricing_fields_survive_ingestion_without_implied_provider() {
    let root = tempfile::tempdir().unwrap();
    let session = "11111111-1111-4111-8111-111111111111";
    let lines = [
        serde_json::json!({"type":"assistant","sessionId":session,"uuid":"synthetic-record","requestId":"synthetic-request","message":{
            "id":"synthetic-message","model":"synthetic-model","usage":{
                "input_tokens":10,"output_tokens":2,"speed":"fast","service_tier":"standard","inference_geo":"us"
            }
        }}),
    ];
    fs::write(root.path().join(format!("{session}.jsonl")), jsonl(&lines)).unwrap();
    let ingested = urollup_core::adapters::claude_project::ingest_root(root.path()).unwrap();
    assert_eq!(ingested.ledger.requests.len(), 1);
    let request = ingested.ledger.requests.values().next().unwrap();
    let pricing = request.pricing.as_ref().expect("recorded Claude pricing metadata");
    assert_eq!(pricing.speed, Some(Name::new("fast")));
    assert_eq!(pricing.service_tier, Some(Name::new("standard")));
    assert_eq!(pricing.inference_geo, Some(Name::new("us")));
    assert_eq!(pricing.provider, None, "the agent name does not prove the billing provider");
}

#[test]
fn claude_missing_metadata_is_not_inherited_from_the_previous_request() {
    let root = tempfile::tempdir().unwrap();
    let session = "11111111-1111-4111-8111-111111111111";
    let lines: Vec<_> = (0..3).map(|index| {
        let mut usage = serde_json::json!({"input_tokens":10,"output_tokens":index + 1});
        if index != 1 {
            usage["speed"] = serde_json::json!("unrecognized-speed");
        }
        serde_json::json!({"type":"assistant","sessionId":session,"uuid":format!("record-{index}"),"requestId":format!("request-{index}"),"message":{
            "id":format!("message-{index}"),"model":"synthetic-model","usage":usage
        }})
    }).collect();
    fs::write(root.path().join(format!("{session}.jsonl")), jsonl(&lines)).unwrap();
    let ingested = urollup_core::adapters::claude_project::ingest_root(root.path()).unwrap();
    assert_eq!(ingested.ledger.requests.len(), 3);
    let mut shared = Vec::new();
    for request in ingested.ledger.requests.values() {
        let usage = urollup_core::ledger::tokens::TokenMeasures::from(
            request.usage.as_ref().unwrap().revision.usage,
        );
        if usage.output == Some(2) {
            assert!(request.pricing.is_none());
        } else {
            let pricing = request.pricing.as_ref().unwrap();
            assert_eq!(pricing.speed, Some(Name::new("unrecognized-speed")));
            shared.push(pricing);
        }
    }
    assert!(
        std::sync::Arc::ptr_eq(shared[0], shared[1]),
        "identical contexts share storage, not assumptions"
    );
}

#[test]
fn codex_turns_keep_the_provider_and_tier_recorded_at_that_turn() {
    let root = tempfile::tempdir().unwrap();
    let thread = "11111111-1111-4111-8111-111111111111";
    let lines = [
        serde_json::json!({"type":"session_meta","payload":{"id":thread,"model_provider":"custom-provider"}}),
        serde_json::json!({"type":"event_msg","payload":{"type":"thread_settings_applied","thread_id":thread,"thread_settings":{"service_tier":"fast"}}}),
        serde_json::json!({"type":"turn_context","payload":{"turn_id":"one","model":"synthetic-model"}}),
        serde_json::json!({"type":"event_msg","payload":{"type":"thread_settings_applied","thread_id":thread,"thread_settings":{"service_tier":"standard"}}}),
        serde_json::json!({"type":"turn_context","payload":{"turn_id":"two","model":"synthetic-model"}}),
        // A delayed usage record still belongs to the first turn's context.
        serde_json::json!({"type":"token_usage_record","payload":{"thread_id":thread,"root_turn_id":"one","response_id":"first","usage":{"input_tokens":10,"output_tokens":1}}}),
        serde_json::json!({"type":"token_usage_record","payload":{"thread_id":thread,"root_turn_id":"two","response_id":"second","usage":{"input_tokens":20,"output_tokens":1}}}),
    ];
    fs::write(root.path().join(format!("rollout-{thread}.jsonl")), jsonl(&lines)).unwrap();
    let ingested = codex_rollout::ingest_root(root.path()).unwrap();
    assert_eq!(ingested.ledger.requests.len(), 2);
    for request in ingested.ledger.requests.values() {
        let pricing = request.pricing.as_ref().expect("recorded pricing context");
        assert_eq!(pricing.provider, Some(Name::new("custom-provider")));
        let input = urollup_core::ledger::tokens::TokenMeasures::from(
            request.usage.as_ref().unwrap().revision.usage,
        )
        .uncached_input;
        assert_eq!(
            pricing.service_tier,
            Some(Name::new(if input == Some(10) { "fast" } else { "standard" }))
        );
        assert!(!pricing.conflicted);
    }
}

#[test]
fn child_tier_comes_only_from_settings_the_child_owns() {
    let child = "11111111-1111-4111-8111-111111111111";
    let parent = "22222222-2222-4222-8222-222222222222";
    let settings = |ordinal: Option<u64>, thread: Option<&str>| {
        let mut line = serde_json::json!({"type":"event_msg","payload":{"type":"thread_settings_applied","thread_settings":{"service_tier":"fast"}}});
        if let Some(ordinal) = ordinal {
            line["ordinal"] = ordinal.into();
        }
        if let Some(thread) = thread {
            line["payload"]["thread_id"] = thread.into();
        }
        line
    };
    // The child's history starts at ordinal 3; its own turn is at 4.
    let cases = [
        // A native child's copied prefix keeps the parent's header and settings, which
        // name the parent.
        (
            vec![
                serde_json::json!({"ordinal":1,"type":"session_meta","payload":{"id":parent,"model_provider":"copied-provider"}}),
                settings(Some(2), Some(parent)),
            ],
            None,
        ),
        // A setting that names no thread before the boundary may be the parent's.
        (vec![settings(Some(1), None)], None),
        // One without an ordinal is on neither side of the boundary.
        (vec![settings(None, None)], None),
        // A migrated child's own settings name it and precede its boundary.
        (vec![settings(Some(1), Some(child))], Some("fast")),
        // At the boundary, a setting that names no thread is the child's.
        (vec![settings(Some(3), None)], Some("fast")),
    ];
    for (prefix, expected) in cases {
        let root = tempfile::tempdir().unwrap();
        let mut lines = vec![
            serde_json::json!({"ordinal":0,"type":"session_meta","payload":{"id":child,"model_provider":"openai","subagent_history_start_ordinal":3}}),
        ];
        lines.extend(prefix);
        lines.extend([
            serde_json::json!({"ordinal":4,"type":"turn_context","payload":{"turn_id":"child-turn","model":"synthetic-model"}}),
            serde_json::json!({"ordinal":5,"type":"token_usage_record","payload":{"thread_id":child,"turn_id":"child-turn","response_id":"child-response","usage":{"input_tokens":10,"output_tokens":1}}}),
        ]);
        fs::write(root.path().join(format!("rollout-{child}.jsonl")), jsonl(&lines)).unwrap();
        let ingested = codex_rollout::ingest_root(root.path()).unwrap();
        assert_eq!(ingested.ledger.requests.len(), 1);
        let request = ingested.ledger.requests.values().next().unwrap();
        let pricing = request.pricing.as_ref().unwrap();
        assert_eq!(pricing.provider, Some(Name::new("openai")), "{lines:?}");
        assert_eq!(pricing.service_tier, expected.map(Name::new), "{lines:?}");
    }
}

#[test]
fn counter_usage_keeps_cache_categories_and_unknown_recorded_tiers() {
    let root = tempfile::tempdir().unwrap();
    let thread = "11111111-1111-4111-8111-111111111111";
    let mut lines = vec![
        serde_json::json!({"type":"session_meta","payload":{"id":thread,"model_provider":"openai"}}),
    ];
    for (index, tier) in [(1, "fast"), (2, "unrecognized-tier")] {
        lines.push(serde_json::json!({"type":"event_msg","payload":{"type":"thread_settings_applied","thread_id":thread,"thread_settings":{"service_tier":tier}}}));
        // An unrelated settings update must not erase the recorded tier.
        lines.push(serde_json::json!({"type":"event_msg","payload":{"type":"thread_settings_applied","thread_id":thread,"thread_settings":{}}}));
        lines.push(serde_json::json!({"type":"turn_context","payload":{"turn_id":format!("turn-{index}"),"model":format!("synthetic-{index}")}}));
        lines.push(serde_json::json!({"type":"event_msg","payload":{"type":"token_count","info":{
            "total_token_usage":{"input_tokens":100*index,"cached_input_tokens":20*index,"cache_write_input_tokens":10*index,"output_tokens":5*index,"total_tokens":105*index},
            "last_token_usage":{"input_tokens":100,"cached_input_tokens":20,"cache_write_input_tokens":10,"output_tokens":5,"total_tokens":105}
        }}}));
    }
    fs::write(root.path().join(format!("rollout-{thread}.jsonl")), jsonl(&lines)).unwrap();
    let ingested = codex_rollout::ingest_root(root.path()).unwrap();
    assert_eq!(ingested.ledger.requests.len(), 2);
    for request in ingested.ledger.requests.values() {
        let usage = urollup_core::ledger::tokens::TokenMeasures::from(
            request.usage.as_ref().unwrap().revision.usage,
        );
        assert_eq!(usage.uncached_input, Some(70));
        assert_eq!(usage.cache_read, Some(20));
        assert_eq!(usage.cache_write_unspecified, Some(10));
        assert_eq!(usage.total().unwrap(), Some(105));
        let first = request.model.as_ref().unwrap().name == Name::new("synthetic-1");
        assert_eq!(
            request.pricing.as_ref().unwrap().service_tier,
            Some(Name::new(if first { "fast" } else { "unrecognized-tier" }))
        );
    }
}

/// Every fixture request whose expected result records a service tier or speed retains
/// exactly that recorded value, and one expected with a null tier retains none.
#[test]
fn fixture_requests_keep_their_expected_recorded_tier_and_speed() {
    use urollup_core::ledger::tokens::TokenMeasures;
    let fixtures = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let mut checked = 0;
    for dialect in ["claude-project", "codex-rollout"] {
        let mut cases: Vec<_> = fs::read_dir(fixtures.join(dialect))
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| path.join("expected.json").is_file())
            .collect();
        cases.sort();
        for case in cases {
            let expected: serde_json::Value =
                serde_json::from_slice(&fs::read(case.join("expected.json")).unwrap()).unwrap();
            let rows: Vec<_> = expected["requests"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|row| row.get("service_tier").is_some() || row.get("speed").is_some())
                .collect();
            if rows.is_empty() {
                continue;
            }
            let ingested = if dialect == "codex-rollout" {
                codex_rollout::ingest_root(&case)
            } else {
                urollup_core::adapters::claude_project::ingest_root(&case)
            }
            .unwrap();
            for row in rows {
                let tokens = |field: &str| row["tokens"][field].as_u64();
                let matches: Vec<_> = ingested
                    .ledger
                    .requests
                    .values()
                    .filter(|request| {
                        request.usage.as_ref().is_some_and(|usage| {
                            let usage = TokenMeasures::from(usage.revision.usage);
                            usage.uncached_input == tokens("uncached_input")
                                && usage.cache_read == tokens("cache_read")
                                && usage.output == tokens("output")
                        })
                    })
                    .collect();
                assert_eq!(matches.len(), 1, "{case:?}: one request has the tokens of {row}");
                let pricing = matches[0].pricing.as_deref();
                for (field, actual) in [
                    ("service_tier", pricing.and_then(|context| context.service_tier)),
                    ("speed", pricing.and_then(|context| context.speed)),
                ] {
                    if let Some(value) = row.get(field) {
                        assert_eq!(
                            actual,
                            value.as_str().map(Name::new),
                            "{case:?} {field}: {row}"
                        );
                    }
                }
                checked += 1;
            }
        }
    }
    assert_eq!(checked, 13, "fixture requests with an expected tier or speed");
}
