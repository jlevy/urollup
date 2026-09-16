---
type: is
id: is-01m2ksma5tmkfs0acfc62p7z23
title: Decide how observations of a conflicting shared key are partitioned
kind: task
status: open
priority: 2
version: 1
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - milestone-0.1
  - design-decision
dependencies: []
parent_id: is-01m2ke36qgfvdvnhw7c7v5esm6
created_at: 2026-09-16T00:24:53.177Z
updated_at: 2026-09-16T00:24:53.177Z
---
Design 3.6 (Identity Basis and Linking, Candidate 9.1 Conflicting Shared Keys) says a shared key whose observations disagree on revision-invariant fields is ambiguous and yields a diagnostic, not a merge. uro-spce implements that strictly: every observation in the conflicting group gets its own artifact-local ID and they form one candidate set, so only one member counts and the rest are unresolved.

That undercounts when the conflict is between two agreeing subgroups, which is the common shape: several block records of one response in file A agree with each other, and a gateway reuse in file B disagrees with all of them. The strict rule then reports one record's usage and four unresolved, instead of two requests.

Decide between: (a) keep the strict rule, since unresolved usage is visible and --strict exits 3; (b) partition the group by its invariant-field fingerprint, so agreeing observations still merge and only the differing subgroups become separate ambiguous requests. Option (b) needs a rule for which fields form the fingerprint. Implementation is in crates/urollup-core/src/ledger/reconcile.rs (conflicting_fields and the split in reconcile); the test is a_shared_key_with_disagreeing_invariants_is_ambiguous_not_merged.
