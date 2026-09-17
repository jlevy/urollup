use std::collections::{BTreeMap, BTreeSet};

use proptest::prelude::*;

use super::{
    KeyGraph, LatestRevision, LineageLink, ObservationRole, OwnerEvidence, ReconcileError,
    ReconcileInput, RequestObservation, RevisionChoice, RevisionSelector, reconcile,
};
use crate::accounting::totals::{Completeness, PartialReason, ledger_totals, selection_totals};
use crate::ledger::coverage::{CoverageGap, UnobservedReason};
use crate::ledger::diagnostics::DiagnosticCode;
use crate::ledger::entities::{
    AccountAttribution, Basis, Confidence, Counting, Ownership, ProviderLimitObservation,
    Relationship, RelationshipKind, RevisionStatus, Thread, ToolAction,
};
use crate::ledger::identity::{AnalyticalId, IdPrefix, IdentityKey, KeyComponent, StoredIdentity};
use crate::ledger::scope::tests::{PROVIDER_RESPONSE, THREAD_DIGEST};
use crate::ledger::scope::{DerivedKey, IdentityBasis, ScopedKey};
use crate::ledger::tokens::TokenMeasures;
use crate::sources::evidence::EvidenceRef;
use crate::test_support::shuffle;

fn source(n: u8) -> AnalyticalId {
    IdentityKey::new(IdPrefix::Source, "test-source", vec![KeyComponent::Integer(i64::from(n))])
        .derive_id()
        .unwrap()
}

fn thread(name: &str) -> AnalyticalId {
    IdentityKey::new(IdPrefix::Thread, "test-thread", vec![KeyComponent::text(name)])
        .derive_id()
        .unwrap()
}

fn evidence(src: u8, offset: u64) -> EvidenceRef {
    EvidenceRef { source: source(src), offset, length: 10 }
}

fn response_key(id: &str) -> DerivedKey {
    PROVIDER_RESPONSE
        .key(vec![KeyComponent::text("anthropic"), KeyComponent::text(id)])
        .unwrap()
        .derive()
        .unwrap()
}

fn digest_key(owner: &str, digest: &str) -> DerivedKey {
    THREAD_DIGEST
        .key(vec![KeyComponent::text(thread(owner).to_string()), KeyComponent::text(digest)])
        .unwrap()
        .derive()
        .unwrap()
}

fn usage(input: u64, output: u64) -> TokenMeasures {
    TokenMeasures { uncached_input: Some(input), output: Some(output), ..TokenMeasures::default() }
}

/// An original observation of `response` in `src` at `offset`, owned by thread `t1`.
fn observed(src: u8, offset: u64, response: &str, output: u64) -> RequestObservation {
    let mut observation = RequestObservation::new(evidence(src, offset), "test");
    observation.keys = vec![response_key(response)].into();
    observation.owner = OwnerEvidence::Proven(thread("t1"));
    observation.usage = Some(usage(100, output));
    observation
}

fn stored(prefix: IdPrefix, kind: &str, name: &str) -> StoredIdentity {
    StoredIdentity::derive(IdentityKey::new(prefix, kind, vec![KeyComponent::text(name)])).unwrap()
}

fn thread_observation(name: &str, src: u8, offset: u64, property: u8) -> Thread {
    let identity = stored(IdPrefix::Thread, "test-thread", name);
    Thread {
        identity,
        basis: IdentityBasis::Native,
        aliases: Vec::new(),
        native_key: BTreeMap::from([("session_id".to_owned(), name.to_owned())]),
        source: match property {
            0 => Basis::Unknown,
            value => Basis::Observed(format!("source-{}", value % 2)),
        },
        initiator: Basis::Unknown,
        purpose: Basis::Unknown,
        execution_environment: Basis::Observed("local".to_owned()),
        project: Basis::Unknown,
        account: Basis::Unknown,
        evidence: vec![evidence(src, offset)],
    }
}

fn action_observation(name: &str, src: u8, offset: u64, property: u8) -> ToolAction {
    ToolAction {
        identity: stored(IdPrefix::Action, "test-action", name),
        basis: IdentityBasis::Native,
        call_id: (property != 0).then(|| format!("call-{}", property % 2)),
        tool_name: (property == 0).then(|| "Read".to_owned()),
        request: None,
        evidence: vec![evidence(src, offset)],
    }
}

fn relationship_observation(from: u8, to: u8, src: u8, offset: u64, proven: bool) -> Relationship {
    Relationship {
        kind: RelationshipKind::Spawn,
        from: thread(&format!("t{from}")),
        to: thread(&format!("t{to}")),
        confidence: if proven { Confidence::Proven } else { Confidence::Inferred },
        evidence: vec![evidence(src, offset)],
    }
}

fn limit_observation(stream: u8, src: u8, offset: u64, value: u8) -> ProviderLimitObservation {
    ProviderLimitObservation {
        limit_name: Some(format!("limit-{stream}")),
        window: Some("primary".to_owned()),
        observed_at: Basis::Unknown,
        owner_thread: Some(thread(&format!("t{stream}"))),
        owner_request: None,
        native: serde_json::json!({ "used_percent": value }).to_string().into_boxed_str(),
        evidence: evidence(src, offset),
    }
}

fn run(requests: Vec<RequestObservation>) -> super::Ledger {
    reconcile(ReconcileInput { requests, ..ReconcileInput::default() }, &LatestRevision).unwrap()
}

fn codes(ledger: &super::Ledger) -> Vec<DiagnosticCode> {
    ledger.diagnostics.iter().map(|d| d.code).collect()
}

#[test]
fn rereads_ignore_request_key_order_and_duplicates() {
    let mut original = observed(0, 10, "msg_1", 7);
    original.keys.push(digest_key("t1", "digest_1"));
    let mut reread = original.clone();
    let mut keys: Vec<_> = reread.keys.iter().cloned().collect();
    keys.reverse();
    keys.push(keys[0].clone());
    reread.keys = keys.into();
    let expected = run(vec![original.clone(), original.clone()]);
    assert_eq!(run(vec![original, reread]), expected);
}

#[test]
fn streamed_usage_updates_collapse_into_one_request_with_revisions() {
    let mut records = Vec::new();
    for (sequence, output) in [(1, 5), (2, 9), (3, 30)] {
        let mut record = observed(0, sequence * 100, "msg_1", output);
        record.sequence = Some(sequence);
        records.push(record);
    }
    let ledger = run(records);
    assert_eq!(ledger.requests.len(), 1);
    let request = ledger.requests.values().next().unwrap();
    let selected = request.usage.as_ref().unwrap();
    assert_eq!(selected.revision.usage.output, Some(30));
    assert_eq!(selected.status, RevisionStatus::Final);
    // Every usage-bearing original is evidence; the selected one is counted.
    assert_eq!(request.evidence.len(), 3);
    assert_eq!(request.basis, IdentityBasis::Native);
    assert_eq!(request.ownership, Ownership::Owned { thread: thread("t1") });
    assert_eq!(*request.id(), response_key("msg_1").id);
}

#[test]
fn unordered_revisions_are_selected_by_rule_in_canonical_order() {
    let ledger = run(vec![observed(1, 0, "msg_1", 7), observed(0, 50, "msg_1", 3)]);
    let request = ledger.requests.values().next().unwrap();
    let selected = request.usage.as_ref().unwrap();
    assert_eq!(selected.status, RevisionStatus::Selected);
    assert_eq!(selected.rule, "latest-revision");
    // Canonical order sorts by source ID first, so the choice never depends on read order.
    let last = [evidence(0, 50), evidence(1, 0)].into_iter().max().unwrap();
    assert_eq!(selected.revision.evidence, last);
}

#[test]
fn copies_are_evidence_and_never_counted() {
    let original = observed(0, 0, "msg_1", 10);
    let mut copy = observed(1, 400, "msg_1", 999);
    copy.role = ObservationRole::Copy;
    copy.owner = OwnerEvidence::Candidates(BTreeSet::from([thread("child")]));
    let ledger = run(vec![copy, original]);
    let request = ledger.requests.values().next().unwrap();
    assert_eq!(request.usage.as_ref().unwrap().revision.usage.output, Some(10));
    assert_eq!(*request.copies, [evidence(1, 400)]);
    assert_eq!(request.evidence.len(), 1);
    // Proven ownership wins over the copy's candidate.
    assert_eq!(request.ownership, Ownership::Owned { thread: thread("t1") });
    let totals = ledger_totals(&ledger).unwrap();
    assert_eq!((totals.total.requests, totals.total.tokens.output), (1, Some(10)));
    assert_eq!(ledger.coverage.copies, 1);
}

#[test]
fn a_request_seen_only_as_copies_counts_nothing_and_is_diagnosed() {
    let mut copy = observed(1, 0, "msg_1", 50);
    copy.role = ObservationRole::Copy;
    let ledger = run(vec![copy]);
    let request = ledger.requests.values().next().unwrap();
    assert_eq!(request.counting, Counting::CopyOnly);
    assert!(request.usage.is_none());
    assert_eq!(codes(&ledger), vec![DiagnosticCode::CopyWithoutOriginal]);
    let totals = ledger_totals(&ledger).unwrap();
    assert_eq!((totals.total.requests, totals.copy_only.requests), (0, 1));
}

#[test]
fn a_record_read_twice_counts_once() {
    let once = run(vec![observed(0, 0, "msg_1", 10)]);
    let twice = run(vec![observed(0, 0, "msg_1", 10), observed(0, 0, "msg_1", 10)]);
    assert_eq!(once.requests, twice.requests);
    assert_eq!(twice.coverage.rereads, 1);
}

#[test]
fn a_record_location_with_changed_content_is_diagnosed() {
    let ledger = run(vec![observed(0, 0, "msg_1", 10), observed(0, 0, "msg_1", 11)]);
    assert_eq!(codes(&ledger), vec![DiagnosticCode::ConflictingReread]);
    assert_eq!(ledger.coverage.conflicting_rereads, 1);
    assert_eq!(ledger.requests.values().next().unwrap().evidence.len(), 1);
}

#[test]
fn a_shared_key_with_disagreeing_invariants_is_ambiguous_not_merged() {
    // A gateway reusing one message ID in two sessions.
    let mut a = observed(0, 0, "msg_1", 10);
    a.invariants.push(("session", "s-a".to_owned()));
    let mut b = observed(1, 0, "msg_1", 20);
    b.invariants.push(("session", "s-b".to_owned()));
    let ledger = run(vec![a, b]);

    assert_eq!(ledger.requests.len(), 2);
    assert!(ledger.requests.values().all(|r| r.basis == IdentityBasis::Ambiguous));
    assert!(!ledger.requests.contains_key(&response_key("msg_1").id));
    assert_eq!(ledger.candidate_sets.len(), 1);
    assert_eq!(
        codes(&ledger),
        vec![DiagnosticCode::ConflictingSharedKey, DiagnosticCode::UnresolvedCandidate]
    );
    let counted: Vec<_> =
        ledger.requests.values().filter(|r| r.counting == Counting::Counted).collect();
    assert_eq!(counted.len(), 1);
    // The lowest ID is counted when the bases tie.
    assert_eq!(counted[0].id(), ledger.requests.keys().next().unwrap());

    let totals = ledger_totals(&ledger).unwrap();
    assert_eq!((totals.total.requests, totals.unresolved.requests), (1, 1));
    assert_eq!(
        totals.completeness,
        Completeness::Partial(BTreeSet::from([PartialReason::UnresolvedUsage]))
    );
}

#[test]
fn a_candidate_set_counts_the_strongest_basis_before_the_lowest_id() {
    let mut native = observed(0, 0, "msg_1", 10);
    native.candidate_tokens.insert("content-digest-1".to_owned());
    let mut fallback = RequestObservation::new(evidence(1, 0), "test");
    fallback.keys = vec![digest_key("t1", "content-digest-1")].into();
    fallback.usage = Some(usage(100, 10));
    fallback.candidate_tokens.insert("content-digest-1".to_owned());
    let ledger = run(vec![fallback, native]);

    assert_eq!(ledger.candidate_sets.len(), 1);
    let native_id = response_key("msg_1").id;
    for request in ledger.requests.values() {
        let expected = if *request.id() == native_id {
            Counting::Counted
        } else {
            Counting::Unresolved { counted: native_id.clone() }
        };
        assert_eq!(request.counting, expected);
    }
}

#[test]
fn ownership_is_owned_ambiguous_or_unknown() {
    let owned = observed(0, 0, "owned", 1);
    let mut contested = observed(0, 100, "contested", 1);
    let mut contested_copy = observed(1, 100, "contested", 1);
    contested_copy.owner = OwnerEvidence::Proven(thread("t2"));
    contested.owner = OwnerEvidence::Proven(thread("t1"));
    let mut candidates = observed(0, 200, "candidates", 1);
    candidates.owner = OwnerEvidence::Candidates(BTreeSet::from([thread("t1"), thread("t3")]));
    let mut unknown = observed(0, 300, "unknown", 1);
    unknown.owner = OwnerEvidence::None;
    let ledger = run(vec![owned, contested, contested_copy, candidates, unknown]);

    let ownership = |response: &str| {
        let id = response_key(response).id;
        ledger.requests[&id].ownership.clone()
    };
    assert_eq!(ownership("owned"), Ownership::Owned { thread: thread("t1") });
    assert_eq!(
        ownership("contested"),
        Ownership::Ambiguous { candidates: BTreeSet::from([thread("t1"), thread("t2")]) }
    );
    assert_eq!(
        ownership("candidates"),
        Ownership::Ambiguous { candidates: BTreeSet::from([thread("t1"), thread("t3")]) }
    );
    assert_eq!(ownership("unknown"), Ownership::Unknown);
    assert_eq!(codes(&ledger), vec![DiagnosticCode::ConflictingOwners]);

    let totals = ledger_totals(&ledger).unwrap();
    assert_eq!(
        (totals.owned.requests, totals.ambiguous.requests, totals.unknown.requests),
        (1, 2, 1)
    );
    assert_eq!(totals.total.requests, 4);

    // A selection counts owned requests and ambiguous ones wholly inside it; the rest are
    // possible and never added.
    let t1 = selection_totals(&ledger, &BTreeSet::from([thread("t1")])).unwrap();
    assert_eq!((t1.counted.requests, t1.possible.requests), (1, 2));
    assert_eq!(
        t1.completeness,
        Completeness::Partial(BTreeSet::from([PartialReason::PossibleUsage]))
    );
    let wide =
        selection_totals(&ledger, &BTreeSet::from([thread("t1"), thread("t2"), thread("t3")]))
            .unwrap();
    assert_eq!((wide.counted.requests, wide.possible.requests), (3, 0));
    assert_eq!(wide.completeness, Completeness::Complete);
}

#[test]
fn conflicting_accounts_are_diagnosed_not_split() {
    let mut a = observed(0, 0, "msg_1", 1);
    a.account = Some("acct-1".to_owned());
    let mut b = observed(1, 0, "msg_1", 1);
    b.account = Some("acct-2".to_owned());
    let ledger = run(vec![a, b]);
    assert_eq!(ledger.requests.len(), 1);
    let request = ledger.requests.values().next().unwrap();
    assert_eq!(
        request.account,
        AccountAttribution::Conflicting(BTreeSet::from(["acct-1".to_owned(), "acct-2".to_owned()]))
    );
    assert_eq!(codes(&ledger), vec![DiagnosticCode::ConflictingAccounts]);
}

#[test]
fn lineage_links_merge_keys_and_keep_aliases() {
    let parent = observed(0, 0, "msg_1", 10);
    let mut child_copy = RequestObservation::new(evidence(1, 0), "test");
    child_copy.keys = vec![digest_key("child", "d1")].into();
    child_copy.role = ObservationRole::Copy;
    let native_id = response_key("msg_1").id;
    let fallback_id = digest_key("child", "d1").id;
    let input = ReconcileInput {
        requests: vec![child_copy, parent],
        links: vec![LineageLink {
            a: fallback_id.clone(),
            b: native_id.clone(),
            evidence: vec![evidence(1, 0)],
        }],
        ..ReconcileInput::default()
    };
    let ledger = reconcile(input, &LatestRevision).unwrap();
    assert_eq!(ledger.requests.len(), 1);
    let request = &ledger.requests[&native_id];
    assert_eq!(*request.aliases, [fallback_id]);
    assert_eq!(*request.copies, [evidence(1, 0)]);
}

#[test]
fn unobserved_gaps_and_unknown_usage_make_totals_partial() {
    let mut no_usage = observed(0, 0, "msg_1", 1);
    no_usage.usage = None;
    let input = ReconcileInput {
        requests: vec![no_usage],
        gaps: vec![CoverageGap {
            reason: UnobservedReason::EphemeralThread,
            thread: None,
            evidence: Vec::new(),
        }],
        ..ReconcileInput::default()
    };
    let ledger = reconcile(input, &LatestRevision).unwrap();
    let totals = ledger_totals(&ledger).unwrap();
    assert_eq!(totals.total.tokens, TokenMeasures::default());
    assert_eq!(
        totals.completeness,
        Completeness::Partial(BTreeSet::from([
            PartialReason::RequestWithoutUsage,
            PartialReason::UnobservedGap
        ]))
    );
}

/// A §3.4-style dialect rule: the largest output, then the last in canonical order, with a
/// disagreement when input differs among revisions.
struct LargestOutput;

impl RevisionSelector for LargestOutput {
    fn rule(&self) -> &'static str {
        "largest-output"
    }

    fn select(&self, revisions: &[&RequestObservation]) -> RevisionChoice {
        let output = |r: &RequestObservation| r.usage.as_ref().and_then(|u| u.output);
        let selected = revisions
            .iter()
            .enumerate()
            .max_by_key(|(_, r)| output(r))
            .map_or(0, |(index, _)| index);
        let inputs: BTreeSet<_> =
            revisions.iter().map(|r| r.usage.as_ref().and_then(|u| u.uncached_input)).collect();
        let disagreements =
            if inputs.len() > 1 { vec!["uncached_input differs".to_owned()] } else { Vec::new() };
        RevisionChoice { selected, status: RevisionStatus::Selected, disagreements }
    }
}

#[test]
fn a_dialect_selector_plugs_in_and_reports_disagreements() {
    let mut first = observed(0, 0, "msg_1", 40);
    first.usage = Some(usage(100, 40));
    let mut second = observed(0, 10, "msg_1", 12);
    second.usage = Some(usage(101, 12));
    let input = ReconcileInput { requests: vec![second, first], ..ReconcileInput::default() };
    let ledger = reconcile(input, &LargestOutput).unwrap();
    let selected = ledger.requests.values().next().unwrap().usage.clone().unwrap();
    assert_eq!((selected.rule, selected.revision.evidence), ("largest-output", evidence(0, 0)));
    assert_eq!(codes(&ledger), vec![DiagnosticCode::RevisionDisagreement]);
}

struct OutOfRange;

impl RevisionSelector for OutOfRange {
    fn rule(&self) -> &'static str {
        "out-of-range"
    }

    fn select(&self, revisions: &[&RequestObservation]) -> RevisionChoice {
        RevisionChoice {
            selected: revisions.len(),
            status: RevisionStatus::Final,
            disagreements: Vec::new(),
        }
    }
}

#[test]
fn engine_errors_are_values() {
    let input =
        ReconcileInput { requests: vec![observed(0, 0, "m", 1)], ..ReconcileInput::default() };
    assert!(matches!(
        reconcile(input.clone(), &OutOfRange),
        Err(ReconcileError::InvalidRevisionChoice { selected: 1, count: 1, .. })
    ));
    let mut wrong = observed(0, 0, "m", 1);
    wrong.keys = vec![
        ScopedKey {
            precedence: 0,
            basis: IdentityBasis::Native,
            key: IdentityKey::new(IdPrefix::Thread, "t", vec![KeyComponent::text("x")]),
        }
        .derive()
        .unwrap(),
    ]
    .into();
    let input = ReconcileInput { requests: vec![wrong], ..ReconcileInput::default() };
    assert!(matches!(reconcile(input, &LatestRevision), Err(ReconcileError::WrongPrefix { .. })));
}

#[test]
fn non_request_entities_merge_evidence_and_diagnose_conflicting_properties() {
    let mut first_thread = thread_observation("t1", 0, 0, 1);
    first_thread.project = Basis::Observed("alpha".to_owned());
    let mut second_thread = thread_observation("t1", 1, 0, 0);
    second_thread.project = Basis::Observed("beta".to_owned());

    let first_action = action_observation("a1", 0, 10, 1);
    let second_action = action_observation("a1", 1, 10, 0);
    let input = ReconcileInput {
        threads: vec![second_thread, first_thread],
        relationships: vec![
            relationship_observation(0, 1, 0, 20, false),
            relationship_observation(0, 1, 1, 20, true),
        ],
        tool_actions: vec![second_action, first_action],
        ..ReconcileInput::default()
    };
    let ledger = reconcile(input, &LatestRevision).unwrap();

    let reconciled_thread = &ledger.threads[&thread("t1")];
    assert_eq!(reconciled_thread.source, Basis::Observed("source-1".to_owned()));
    assert_eq!(reconciled_thread.project, Basis::Unknown);
    let mut thread_evidence = vec![evidence(0, 0), evidence(1, 0)];
    thread_evidence.sort();
    assert_eq!(reconciled_thread.evidence, thread_evidence);
    assert_eq!(ledger.relationships.len(), 1);
    assert_eq!(ledger.relationships[0].confidence, Confidence::Proven);
    let mut relationship_evidence = vec![evidence(0, 20), evidence(1, 20)];
    relationship_evidence.sort();
    assert_eq!(ledger.relationships[0].evidence, relationship_evidence);
    let action = ledger.tool_actions.values().next().unwrap();
    assert_eq!(action.call_id.as_deref(), Some("call-1"));
    assert_eq!(action.tool_name.as_deref(), Some("Read"));
    let mut action_evidence = vec![evidence(0, 10), evidence(1, 10)];
    action_evidence.sort();
    assert_eq!(action.evidence, action_evidence);
    assert_eq!(codes(&ledger), vec![DiagnosticCode::ConflictingSharedKey]);
}

#[test]
fn entity_references_follow_reconciled_aliases() {
    let mut fallback_thread = thread_observation("old", 0, 0, 1);
    let canonical_thread = thread_observation("new", 0, 10, 1);
    fallback_thread.basis = IdentityBasis::Fallback;
    fallback_thread.aliases = vec![canonical_thread.identity.clone()];
    fallback_thread.native_key.clear();

    let request_alias = digest_key("new", "digest").id;
    let request_id = response_key("msg_1").id;
    let mut request = observed(0, 20, "msg_1", 1);
    request.keys.push(digest_key("new", "digest"));
    request.owner = OwnerEvidence::Proven(thread("old"));
    let mut action = action_observation("a1", 0, 30, 0);
    action.request = Some(request_alias.clone());
    let mut limit = limit_observation(0, 0, 40, 1);
    limit.owner_thread = Some(thread("old"));
    limit.owner_request = Some(request_alias);
    let mut relationship = relationship_observation(0, 1, 0, 50, true);
    relationship.from = thread("old");

    let ledger = reconcile(
        ReconcileInput {
            threads: vec![fallback_thread, canonical_thread],
            relationships: vec![relationship],
            requests: vec![request],
            tool_actions: vec![action],
            limit_observations: vec![limit],
            ..ReconcileInput::default()
        },
        &LatestRevision,
    )
    .unwrap();

    assert!(ledger.threads.contains_key(&thread("new")));
    assert_eq!(ledger.relationships[0].from, thread("new"));
    assert_eq!(ledger.requests[&request_id].ownership, Ownership::Owned { thread: thread("new") });
    assert_eq!(ledger.tool_actions.values().next().unwrap().request, Some(request_id.clone()));
    assert_eq!(ledger.limit_observations[0].owner_thread, Some(thread("new")));
    assert_eq!(ledger.limit_observations[0].owner_request, Some(request_id));
}

#[test]
fn only_consecutive_identical_limit_snapshots_collapse() {
    let ledger = reconcile(
        ReconcileInput {
            limit_observations: vec![
                limit_observation(0, 0, 30, 1),
                limit_observation(0, 0, 0, 1),
                limit_observation(0, 0, 20, 2),
                limit_observation(0, 0, 10, 1),
            ],
            ..ReconcileInput::default()
        },
        &LatestRevision,
    )
    .unwrap();
    assert_eq!(ledger.limit_observations.len(), 3);
    assert_eq!(
        ledger
            .limit_observations
            .iter()
            .map(|observation| observation.evidence.offset)
            .collect::<Vec<_>>(),
        vec![0, 20, 30]
    );
}

/// Observations drawn from a small universe, so keys, copies, conflicts, rereads and
/// candidate tokens collide often.
fn arbitrary_observation() -> impl Strategy<Value = RequestObservation> {
    (
        (0u8..3, 0u64..6),
        prop::option::of(0u8..4),
        prop::option::of(0u8..3),
        any::<bool>(),
        0u8..4,
        prop::option::of((0u64..50, 0u64..50)),
        prop::option::of(0u64..4),
        prop::option::weighted(0.15, 0u8..2),
        prop::option::weighted(0.2, 0u8..2),
        prop::option::weighted(0.2, 0u8..2),
    )
        .prop_map(
            |(
                (src, slot),
                response,
                digest,
                copy,
                owner,
                used,
                sequence,
                session,
                token,
                account,
            )| {
                let mut observation = RequestObservation::new(evidence(src, slot * 100), "test");
                if let Some(response) = response {
                    observation.keys.push(response_key(&format!("msg_{response}")));
                }
                if let Some(digest) = digest {
                    observation.keys.push(digest_key("t1", &format!("d{digest}")));
                }
                if copy {
                    observation.role = ObservationRole::Copy;
                }
                observation.owner = match owner {
                    0 => OwnerEvidence::None,
                    1 => OwnerEvidence::Proven(thread("t1")),
                    2 => OwnerEvidence::Proven(thread("t2")),
                    _ => OwnerEvidence::Candidates(BTreeSet::from([thread("t1"), thread("t3")])),
                };
                observation.usage = used.map(|(input, output)| usage(input, output));
                observation.sequence = sequence;
                if let Some(session) = session {
                    observation.invariants.push(("session", format!("s{session}")));
                }
                if let Some(token) = token {
                    observation.candidate_tokens.insert(format!("c{token}"));
                }
                observation.account = account.map(|a| format!("acct-{a}"));
                observation
            },
        )
}

fn arbitrary_entity_input() -> impl Strategy<Value = ReconcileInput> {
    (
        prop::collection::vec((0u8..3, 0u8..3, 0u8..3, 0u64..6), 0..12),
        prop::collection::vec((0u8..3, 0u8..3, 0u8..3, 0u64..6, any::<bool>()), 0..12),
        prop::collection::vec((0u8..3, 0u8..3, 0u8..3, 0u64..6), 0..12),
        prop::collection::vec((0u8..2, 0u8..3, 0u64..6, 0u8..3), 0..12),
    )
        .prop_map(|(threads, relationships, actions, limits)| ReconcileInput {
            threads: threads
                .into_iter()
                .map(|(name, property, src, slot)| {
                    thread_observation(&format!("t{name}"), src, slot * 10, property)
                })
                .collect(),
            relationships: relationships
                .into_iter()
                .map(|(from, to, src, slot, proven)| {
                    relationship_observation(from, to, src, slot * 10, proven)
                })
                .collect(),
            tool_actions: actions
                .into_iter()
                .map(|(name, property, src, slot)| {
                    action_observation(&format!("a{name}"), src, slot * 10, property)
                })
                .collect(),
            limit_observations: limits
                .into_iter()
                .map(|(stream, src, slot, value)| limit_observation(stream, src, slot * 10, value))
                .collect(),
            ..ReconcileInput::default()
        })
}

proptest! {
    // Each case reconciles up to 48 observations in an unoptimized test build; 64 cases
    // keep the four properties to a few seconds in `make check`.
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn the_ledger_does_not_depend_on_observation_order(
        observations in prop::collection::vec(arbitrary_observation(), 0..24),
        seed in any::<u64>(),
    ) {
        let mut shuffled = observations.clone();
        shuffle(&mut shuffled, seed);
        prop_assert_eq!(run(observations), run(shuffled));
    }

    #[test]
    fn reconciling_repeated_imports_is_idempotent(
        observations in prop::collection::vec(arbitrary_observation(), 0..24),
    ) {
        let once = run(observations.clone());
        let mut doubled = observations.clone();
        doubled.extend(observations);
        let twice = run(doubled);
        prop_assert_eq!(&once.requests, &twice.requests);
        prop_assert_eq!(&once.candidate_sets, &twice.candidate_sets);
        prop_assert_eq!(&once.diagnostics, &twice.diagnostics);
        prop_assert_eq!(ledger_totals(&once).unwrap(), ledger_totals(&twice).unwrap());
    }

    #[test]
    fn entity_reconciliation_is_order_independent(
        input in arbitrary_entity_input(),
        seed in any::<u64>(),
    ) {
        let expected = reconcile(input.clone(), &LatestRevision).unwrap();
        let mut shuffled = input;
        shuffle(&mut shuffled.threads, seed);
        shuffle(&mut shuffled.relationships, seed.wrapping_add(1));
        shuffle(&mut shuffled.tool_actions, seed.wrapping_add(2));
        shuffle(&mut shuffled.limit_observations, seed.wrapping_add(3));
        prop_assert_eq!(expected, reconcile(shuffled, &LatestRevision).unwrap());
    }

    #[test]
    fn repeated_entity_imports_are_idempotent(input in arbitrary_entity_input()) {
        let expected = reconcile(input.clone(), &LatestRevision).unwrap();
        let mut doubled = input.clone();
        doubled.threads.extend(input.threads);
        doubled.relationships.extend(input.relationships);
        doubled.tool_actions.extend(input.tool_actions);
        doubled.limit_observations.extend(input.limit_observations);
        prop_assert_eq!(expected, reconcile(doubled, &LatestRevision).unwrap());
    }

    #[test]
    fn every_ledger_request_is_keyed_by_its_id_and_aliases_are_distinct(
        observations in prop::collection::vec(arbitrary_observation(), 0..24),
    ) {
        let ledger = run(observations);
        for (id, request) in ledger.requests.iter() {
            prop_assert_eq!(id, &request.id);
            for alias in &request.aliases {
                prop_assert_eq!(alias.prefix(), IdPrefix::Request);
                prop_assert_ne!(alias, id);
            }
        }
    }

    #[test]
    fn totals_conserve_requests_and_any_unknown_member_is_partial(
        observations in prop::collection::vec(arbitrary_observation(), 0..24),
    ) {
        let ledger = run(observations);
        let totals = ledger_totals(&ledger).unwrap();
        let by_status = totals.owned.tokens
            .checked_add(&totals.ambiguous.tokens).unwrap()
            .checked_add(&totals.unknown.tokens).unwrap();
        prop_assert_eq!(
            totals.owned.requests + totals.ambiguous.requests + totals.unknown.requests,
            totals.total.requests
        );
        prop_assert_eq!(by_status, totals.total.tokens);
        let counted = ledger.requests.values().filter(|r| r.counting == Counting::Counted);
        if counted.clone().any(|r| r.usage.is_none()) {
            prop_assert_ne!(totals.completeness, Completeness::Complete);
        }
        let unresolved = ledger.requests.values().filter(|r| matches!(r.counting, Counting::Unresolved { .. })).count();
        prop_assert_eq!(u64::try_from(unresolved).unwrap(), totals.unresolved.requests);
    }
}

#[test]
fn different_keys_deriving_one_id_are_a_collision() {
    let key = response_key("msg_1");
    let mut graph = KeyGraph::default();
    let node = graph.register(&key).unwrap();
    assert_eq!(graph.register(&key).unwrap(), node);
    let forged = DerivedKey { check: key.check.wrapping_add(1), ..key };
    assert!(matches!(
        graph.register(&forged),
        Err(crate::ledger::identity::IdentityError::DigestCollision { .. })
    ));
}

#[test]
fn key_graph_roots_are_the_lowest_id_in_any_link_order() {
    let keys: Vec<DerivedKey> = (0..6).map(|n| response_key(&format!("msg_{n}"))).collect();
    let lowest = keys.iter().map(|key| key.id.clone()).min().unwrap();
    for reversed in [false, true] {
        let mut graph = KeyGraph::default();
        let mut nodes: Vec<u32> = keys.iter().map(|key| graph.register(key).unwrap()).collect();
        if reversed {
            nodes.reverse();
        }
        for pair in nodes.windows(2) {
            graph.link(pair[0], pair[1]);
        }
        for node in nodes {
            let root = graph.find(node);
            assert_eq!(graph.id(root), &lowest);
        }
    }
}
