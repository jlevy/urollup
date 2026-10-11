//! Recorded pricing context must survive ingestion without consulting machine settings.

use std::fmt::Write;
use std::fs;

use urollup_core::adapters::codex_rollout;
use urollup_core::ledger::entities::{ServiceTier, TierBasis};
use urollup_core::ledger::names::Name;

/// A Codex tier, which records the thread's selected tier.
fn thread_tier(name: &str) -> ServiceTier {
    ServiceTier { name: Name::new(name), basis: TierBasis::ThreadSetting }
}

/// A Codex settings event: a complete snapshot, which omits `service_tier` when none is
/// selected, as Codex 0.154 writes it.
fn settings(thread: &str, tier: Option<&str>, provider: &str) -> serde_json::Value {
    let mut snapshot = serde_json::json!({
        "model":"synthetic-model","model_provider_id":provider,"approval_policy":"never",
        "cwd":"/home/example/project","reasoning_effort":"high"
    });
    if let Some(tier) = tier {
        snapshot["service_tier"] = tier.into();
    }
    serde_json::json!({"type":"event_msg","payload":{"type":"thread_settings_applied","thread_id":thread,"thread_settings":snapshot}})
}

/// Each request's input and its retained tier name.
fn tiers_by_input(
    ingested: &urollup_core::adapters::Ingested,
) -> Vec<(Option<u64>, Option<String>)> {
    let mut tiers: Vec<_> = ingested
        .ledger
        .requests
        .values()
        .map(|request| {
            let usage = urollup_core::ledger::tokens::TokenMeasures::from(
                request.usage.as_ref().expect("every request has usage").revision.usage,
            );
            let tier = request
                .pricing
                .and_then(|pricing| pricing.service_tier)
                .map(|tier| tier.name.as_str().to_owned());
            (usage.uncached_input, tier)
        })
        .collect();
    tiers.sort();
    tiers
}

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
    assert_eq!(
        pricing.service_tier,
        Some(ServiceTier { name: Name::new("standard"), basis: TierBasis::Request }),
        "Claude records the tier the request was served at"
    );
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
        std::ptr::eq(shared[0].context(), shared[1].context()),
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
            Some(thread_tier(if input == Some(10) { "fast" } else { "standard" }))
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
        assert_eq!(pricing.service_tier, expected.map(thread_tier), "{lines:?}");
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
        lines.push(settings(thread, Some(tier), "openai"));
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
            Some(thread_tier(if first { "fast" } else { "unrecognized-tier" }))
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
                    (
                        "service_tier",
                        pricing.and_then(|context| context.service_tier.map(|tier| tier.name)),
                    ),
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

#[test]
fn a_settings_snapshot_without_a_tier_clears_it_and_one_without_settings_keeps_it() {
    let root = tempfile::tempdir().unwrap();
    let thread = "11111111-1111-4111-8111-111111111111";
    let turn = |index: u64| {
        [
            serde_json::json!({"type":"turn_context","payload":{"turn_id":format!("turn-{index}"),"model":"synthetic-model"}}),
            serde_json::json!({"type":"token_usage_record","payload":{"thread_id":thread,"turn_id":format!("turn-{index}"),"response_id":format!("response-{index}"),"usage":{"input_tokens":10 * index,"output_tokens":1}}}),
        ]
    };
    let mut lines = vec![
        serde_json::json!({"type":"session_meta","payload":{"id":thread,"model_provider":"openai"}}),
        settings(thread, Some("priority"), "openai"),
    ];
    lines.extend(turn(1));
    // A resume without a tier writes the whole snapshot without `service_tier`.
    lines.push(settings(thread, None, "openai"));
    lines.extend(turn(2));
    lines.push(settings(thread, Some("flex"), "openai"));
    // An event without a settings object records no snapshot, so the tier stands.
    lines.push(serde_json::json!({"type":"event_msg","payload":{"type":"thread_settings_applied","thread_id":thread}}));
    lines.extend(turn(3));
    fs::write(root.path().join(format!("rollout-{thread}.jsonl")), jsonl(&lines)).unwrap();
    let ingested = codex_rollout::ingest_root(root.path()).unwrap();
    assert_eq!(
        tiers_by_input(&ingested),
        [
            (Some(10), Some("priority".to_owned())),
            (Some(20), None),
            (Some(30), Some("flex".to_owned()))
        ]
    );
}

#[test]
fn a_later_settings_snapshot_updates_the_provider() {
    let root = tempfile::tempdir().unwrap();
    let thread = "11111111-1111-4111-8111-111111111111";
    let lines = [
        serde_json::json!({"type":"session_meta","payload":{"id":thread,"model_provider":"openai"}}),
        serde_json::json!({"type":"turn_context","payload":{"turn_id":"one","model":"synthetic-model"}}),
        serde_json::json!({"type":"token_usage_record","payload":{"thread_id":thread,"turn_id":"one","response_id":"first","usage":{"input_tokens":10,"output_tokens":1}}}),
        // A resume with a provider override records it in the next snapshot.
        settings(thread, None, "other-provider"),
        serde_json::json!({"type":"turn_context","payload":{"turn_id":"two","model":"synthetic-model"}}),
        serde_json::json!({"type":"token_usage_record","payload":{"thread_id":thread,"turn_id":"two","response_id":"second","usage":{"input_tokens":20,"output_tokens":1}}}),
    ];
    fs::write(root.path().join(format!("rollout-{thread}.jsonl")), jsonl(&lines)).unwrap();
    let ingested = codex_rollout::ingest_root(root.path()).unwrap();
    let mut providers: Vec<_> = ingested
        .ledger
        .requests
        .values()
        .map(|request| {
            let usage = urollup_core::ledger::tokens::TokenMeasures::from(
                request.usage.as_ref().unwrap().revision.usage,
            );
            (usage.uncached_input, request.pricing.and_then(|pricing| pricing.provider))
        })
        .collect();
    providers.sort();
    assert_eq!(
        providers,
        [(Some(10), Some(Name::new("openai"))), (Some(20), Some(Name::new("other-provider")))]
    );
}

#[test]
fn a_settings_event_naming_no_thread_sets_the_tier_of_a_rollout_without_a_boundary() {
    let root = tempfile::tempdir().unwrap();
    let thread = "11111111-1111-4111-8111-111111111111";
    let mut unnamed = settings(thread, Some("priority"), "openai");
    unnamed["payload"].as_object_mut().unwrap().remove("thread_id");
    let lines = [
        serde_json::json!({"type":"session_meta","payload":{"id":thread,"model_provider":"openai"}}),
        unnamed,
        serde_json::json!({"type":"turn_context","payload":{"turn_id":"one","model":"synthetic-model"}}),
        serde_json::json!({"type":"token_usage_record","payload":{"thread_id":thread,"turn_id":"one","response_id":"first","usage":{"input_tokens":10,"output_tokens":1}}}),
    ];
    fs::write(root.path().join(format!("rollout-{thread}.jsonl")), jsonl(&lines)).unwrap();
    let ingested = codex_rollout::ingest_root(root.path()).unwrap();
    assert_eq!(tiers_by_input(&ingested), [(Some(10), Some("priority".to_owned()))]);
}

/// Report totals and diagnostic codes, which a pricing conflict must leave unchanged
/// apart from its own diagnostic.
fn totals_and_codes(ingested: &urollup_core::adapters::Ingested) -> (String, Vec<&'static str>) {
    let totals = urollup_core::accounting::totals::ledger_totals(&ingested.ledger)
        .expect("ledger totals add up");
    let mut codes: Vec<_> =
        ingested.ledger.diagnostics.iter().map(|diagnostic| diagnostic.code.token()).collect();
    codes.sort_unstable();
    (format!("{:?}", totals.total), codes)
}

#[test]
fn a_claude_pricing_conflict_marks_the_request_without_changing_totals() {
    let session = "11111111-1111-4111-8111-111111111111";
    let ingest = |second_speed: &str| {
        let root = tempfile::tempdir().unwrap();
        // Two block records of one response, which reconcile into one request.
        let lines: Vec<_> = [(0, "fast"), (1, second_speed)].map(|(index, speed)| {
            serde_json::json!({"type":"assistant","sessionId":session,"uuid":format!("record-{index}"),"requestId":"request","timestamp":"2026-09-20T15:00:00Z","message":{
                "id":"message","model":"synthetic-model","usage":{"input_tokens":10,"output_tokens":5,"speed":speed}
            }})
        }).into();
        fs::write(root.path().join(format!("{session}.jsonl")), jsonl(&lines)).unwrap();
        urollup_core::adapters::claude_project::ingest_root(root.path()).unwrap()
    };
    let agreeing = ingest("fast");
    let conflicting = ingest("standard");
    let request = conflicting.ledger.requests.values().next().unwrap();
    let pricing = request.pricing.unwrap();
    assert!(pricing.conflicted);
    let (totals, codes) = totals_and_codes(&conflicting);
    let (agreeing_totals, agreeing_codes) = totals_and_codes(&agreeing);
    assert_eq!(totals, agreeing_totals, "a conflict never changes totals");
    assert_eq!(agreeing_codes, Vec::<&str>::new());
    assert_eq!(codes, ["conflicting-pricing-context"]);
    let diagnostic = &conflicting.ledger.diagnostics[0];
    assert_eq!(
        diagnostic.detail,
        "original records disagree on speed (fast, standard), so none of them can choose a rate"
    );
}

#[test]
fn a_codex_pricing_conflict_marks_the_request_without_changing_totals() {
    let thread = "019f0000-0000-7000-8000-002100000001";
    let revert = "019f0000-0000-7000-8000-002100000002";
    let ingest = |second_tier: &str| {
        let root = tempfile::tempdir().unwrap();
        let rollout = |tier: &str| {
            jsonl(&[
                serde_json::json!({"type":"session_meta","payload":{"id":thread,"model_provider":"openai"}}),
                settings(thread, Some(tier), "openai"),
                serde_json::json!({"type":"turn_context","payload":{"turn_id":"one","model":"synthetic-model"}}),
                serde_json::json!({"timestamp":"2026-09-20T15:00:20.000Z","type":"token_usage_record","payload":{"thread_id":thread,"turn_id":"one","response_id":"shared","usage":{"input_tokens":100,"cached_input_tokens":20,"cache_write_input_tokens":10,"output_tokens":5}}}),
            ])
        };
        // An original rollout and a revert file of the same thread both record the response.
        fs::write(
            root.path().join(format!("rollout-2026-09-20T08-00-00-{thread}.jsonl")),
            rollout("priority"),
        )
        .unwrap();
        fs::write(
            root.path().join(format!("rollout-2026-09-20T08-10-00-{thread}_{revert}.jsonl")),
            rollout(second_tier),
        )
        .unwrap();
        codex_rollout::ingest_root(root.path()).unwrap()
    };
    let agreeing = ingest("priority");
    let conflicting = ingest("default");
    assert_eq!(conflicting.ledger.requests.len(), 1);
    assert!(conflicting.ledger.requests.values().next().unwrap().pricing.unwrap().conflicted);
    let (totals, codes) = totals_and_codes(&conflicting);
    let (agreeing_totals, agreeing_codes) = totals_and_codes(&agreeing);
    assert_eq!(totals, agreeing_totals, "a conflict never changes totals");
    let mut expected = agreeing_codes;
    expected.push("conflicting-pricing-context");
    expected.sort_unstable();
    assert_eq!(codes, expected);
}
