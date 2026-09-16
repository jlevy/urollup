use std::collections::BTreeSet;

use proptest::prelude::*;

use super::{
    LatestRevision, LineageLink, ObservationRole, OwnerEvidence, ReconcileError, ReconcileInput,
    RequestObservation, RevisionChoice, RevisionSelector, reconcile,
};
use crate::accounting::totals::{Completeness, PartialReason, ledger_totals, selection_totals};
use crate::ledger::coverage::{CoverageGap, UnobservedReason};
use crate::ledger::diagnostics::DiagnosticCode;
use crate::ledger::entities::{AccountAttribution, Counting, Ownership, RevisionStatus};
use crate::ledger::identity::{AnalyticalId, IdPrefix, IdentityKey, KeyComponent};
use crate::ledger::scope::tests::{PROVIDER_RESPONSE, THREAD_DIGEST};
use crate::ledger::scope::{IdentityBasis, ScopedKey};
use crate::ledger::tokens::{TokenMeasures, TokenUsage};
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

fn response_key(id: &str) -> ScopedKey {
    PROVIDER_RESPONSE.key(vec![KeyComponent::text("anthropic"), KeyComponent::text(id)]).unwrap()
}

fn digest_key(owner: &str, digest: &str) -> ScopedKey {
    THREAD_DIGEST
        .key(vec![KeyComponent::text(thread(owner).as_str()), KeyComponent::text(digest)])
        .unwrap()
}

fn usage(input: u64, output: u64) -> TokenUsage {
    TokenUsage {
        measures: TokenMeasures {
            uncached_input: Some(input),
            output: Some(output),
            ..TokenMeasures::default()
        },
        native: std::collections::BTreeMap::new(),
    }
}

/// An original observation of `response` in `src` at `offset`, owned by thread `t1`.
fn observed(src: u8, offset: u64, response: &str, output: u64) -> RequestObservation {
    let mut observation = RequestObservation::new(evidence(src, offset), "test");
    observation.keys = vec![response_key(response)];
    observation.owner = OwnerEvidence::Proven(thread("t1"));
    observation.usage = Some(usage(100, output));
    observation
}

fn run(requests: Vec<RequestObservation>) -> super::Ledger {
    reconcile(ReconcileInput { requests, ..ReconcileInput::default() }, &LatestRevision).unwrap()
}

fn codes(ledger: &super::Ledger) -> Vec<DiagnosticCode> {
    ledger.diagnostics.iter().map(|d| d.code).collect()
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
    assert_eq!(selected.revision.usage.measures.output, Some(30));
    assert_eq!(selected.status, RevisionStatus::Final);
    assert_eq!(request.revisions.len(), 3);
    assert_eq!(request.basis, IdentityBasis::Native);
    assert_eq!(request.ownership, Ownership::Owned { thread: thread("t1") });
    assert_eq!(*request.id(), response_key("msg_1").key.derive_id().unwrap());
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
    assert_eq!(request.usage.as_ref().unwrap().revision.usage.measures.output, Some(10));
    assert_eq!(request.copies, vec![evidence(1, 400)]);
    assert_eq!(request.revisions.len(), 1);
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
    assert_eq!(ledger.requests.values().next().unwrap().revisions.len(), 1);
}

#[test]
fn a_shared_key_with_disagreeing_invariants_is_ambiguous_not_merged() {
    // A gateway reusing one message ID in two sessions.
    let mut a = observed(0, 0, "msg_1", 10);
    a.invariants.insert("session".to_owned(), "s-a".to_owned());
    let mut b = observed(1, 0, "msg_1", 20);
    b.invariants.insert("session".to_owned(), "s-b".to_owned());
    let ledger = run(vec![a, b]);

    assert_eq!(ledger.requests.len(), 2);
    assert!(ledger.requests.values().all(|r| r.basis == IdentityBasis::Ambiguous));
    assert!(!ledger.requests.contains_key(&response_key("msg_1").key.derive_id().unwrap()));
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
    fallback.keys = vec![digest_key("t1", "content-digest-1")];
    fallback.usage = Some(usage(100, 10));
    fallback.candidate_tokens.insert("content-digest-1".to_owned());
    let ledger = run(vec![fallback, native]);

    assert_eq!(ledger.candidate_sets.len(), 1);
    let native_id = response_key("msg_1").key.derive_id().unwrap();
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
        let id = response_key(response).key.derive_id().unwrap();
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
    child_copy.keys = vec![digest_key("child", "d1")];
    child_copy.role = ObservationRole::Copy;
    let native_id = response_key("msg_1").key.derive_id().unwrap();
    let fallback_id = digest_key("child", "d1").key.derive_id().unwrap();
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
    assert_eq!(request.aliases.iter().map(|a| a.id.clone()).collect::<Vec<_>>(), vec![fallback_id]);
    assert_eq!(request.copies, vec![evidence(1, 0)]);
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
        let output = |r: &RequestObservation| r.usage.as_ref().and_then(|u| u.measures.output);
        let selected = revisions
            .iter()
            .enumerate()
            .max_by_key(|(_, r)| output(r))
            .map_or(0, |(index, _)| index);
        let inputs: BTreeSet<_> = revisions
            .iter()
            .map(|r| r.usage.as_ref().and_then(|u| u.measures.uncached_input))
            .collect();
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
    wrong.keys = vec![ScopedKey {
        precedence: 0,
        basis: IdentityBasis::Native,
        key: IdentityKey::new(IdPrefix::Thread, "t", vec![KeyComponent::text("x")]),
    }];
    let input = ReconcileInput { requests: vec![wrong], ..ReconcileInput::default() };
    assert!(matches!(reconcile(input, &LatestRevision), Err(ReconcileError::WrongPrefix { .. })));
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
                    observation.invariants.insert("session".to_owned(), format!("s{session}"));
                }
                if let Some(token) = token {
                    observation.candidate_tokens.insert(format!("c{token}"));
                }
                observation.account = account.map(|a| format!("acct-{a}"));
                observation
            },
        )
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
    fn every_ledger_id_rederives_from_its_stored_key(
        observations in prop::collection::vec(arbitrary_observation(), 0..24),
    ) {
        let ledger = run(observations);
        for (id, request) in &ledger.requests {
            prop_assert_eq!(id, &request.identity.id);
            prop_assert!(request.identity.verify().is_ok());
            for alias in &request.aliases {
                prop_assert!(alias.verify().is_ok());
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
