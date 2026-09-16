---
type: is
id: is-01m2kt1stedbst3rb3wacb3xxe
title: "Implement exact aggregation: cover, disjoint and unresolved extents"
kind: task
status: closed
priority: 1
version: 4
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - phase-1
  - milestone-0.2
dependencies:
  - type: blocks
    target: is-01m2kt23gtg5e014w43p0a46c9
  - type: blocks
    target: is-01m2kt2na4erqgcpszpzz6n728
parent_id: is-01m2ke45tasy262pas37jxwss5
created_at: 2026-09-16T00:32:15.162Z
updated_at: 2026-09-16T03:01:50.111Z
closed_at: 2026-09-16T03:01:50.110Z
close_reason: Created twice when the bead-mapping run was interrupted and rerun; uro-yl1i is the original.
resolution: duplicate
duplicate_of: is-01m2ksvsrxm9pqq4yhes5gfxce
---
Milestone 0.2: the merge algebra over summary extents, landed before any totals-only output. Design §5.3.

Acceptance:
- Same-thread, cover and disjoint predicates implemented exactly as specified, including two null-thread extents with equal candidate sets, and the rule that a null-thread extent without an index is never disjoint from another.
- Merge collapses identical extents, removes every extent covered by another, counts each remaining extent disjoint from all the others, and marks the rest status: unresolved, reporting their usage separately rather than adding it; --strict exits 3 on nonzero unresolved usage.
- The output, including its exports provenance, depends only on the set of uncovered extents.
- proptest laws: traversal-order invariance, and merge associativity, commutativity and idempotence for compatible inputs.
- Exact cases tested: different sessions, repeated inputs, a later export of a growing session, copied history in a resumed or forked session, and one session split into consecutive half-open windows.
- Unresolved cases tested: diverged cloud and local exports, overlapping windows, an older usage revision in the larger extent, differing ownership evidence, a missing index, and mixed identity versions; each resolves when the matching bundles merge, and none ever adds usage.
- Disagreeing bucket width, histogram scheme, identity or reconciliation versions are compatibility errors (exit 2).
