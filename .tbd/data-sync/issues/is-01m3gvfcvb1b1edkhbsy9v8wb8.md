---
type: is
id: is-01m3gvfcvb1b1edkhbsy9v8wb8
title: Revise whole-history release acceptance for practical memory scaling
kind: task
status: closed
priority: 1
version: 5
spec_path: docs/project/specs/active/plan-2026-09-16-scalable-ingestion.md
delegate: claude-code@spud10.local
labels: []
dependencies: []
parent_id: is-01m2p3evvpmcn8cf59wc1196f6
hold: null
hold_until: null
created_at: 2026-09-27T07:15:07.753Z
updated_at: 2026-09-27T07:43:15.743Z
started_at: 2026-09-27T07:16:06.962Z
closed_at: 2026-09-27T07:43:15.732Z
close_reason: "Policy and tracking update completed in commit 04a3fdd on PR #15. Full make check passed, including all 30 negative gate probes; final Markdown/link/footer and QA-shell checks passed. All 15 hosted checks passed in CI run 36303556679. The 512 MiB/10-second representative gates are retired. Process-wide safety uro-6pi8, scale proofs uro-z1h1 and representative acceptance uro-erqo remain open release prerequisites. Spill and throughput optimizations are roadmap follow-ups. Private evidence was preserved locally and not published."
resolution: null
duplicate_of: null
---
Apply the maintainer-approved 2026-09-27 policy: retire the representative 512 MiB and 10-second blockers; require manageable reference-corpus footprint, observation-density scaling and a 100 GiB projection within 25% of physical RAM; track process-wide admission safety before 0.1 and hybrid spill as a follow-up. Update the plan, QA, release dependencies and guidance consistently. Keep private corpus measurements local. This bead is policy and planning only; runtime safety and scale proofs remain separate open deliverables.

## Notes

Committed as 04a3fdd and pushed to PR #15. Full make check passed in an external-volume validation copy, including Rust/QA/parity tests, synthetic scale gates, audits and all 30 negative gate probes. Final docs formatting, 914 local links/anchors, eight footers and all 16 QA shell examples checked. Review found no unresolved policy/documentation issues after correcting acceptance ordering, stale links and QA examples. Required runtime work is open under uro-6pi8 and uro-z1h1; representative evidence stays under uro-erqo and spill under follow-up uro-924y. Private benchmark files and validation evidence are retained locally outside disposable scratch. Hosted CI is pending; do not close the release gates.
