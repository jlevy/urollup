---
type: is
id: is-01m3gvfcvb1b1edkhbsy9v8wb8
title: Revise whole-history release acceptance for practical memory scaling
kind: task
status: in_progress
priority: 1
version: 3
spec_path: docs/project/specs/active/plan-2026-09-16-scalable-ingestion.md
delegate: claude-code@spud10.local
labels: []
dependencies: []
parent_id: is-01m2p3evvpmcn8cf59wc1196f6
hold: null
hold_until: null
created_at: 2026-09-27T07:15:07.753Z
updated_at: 2026-09-27T07:28:07.675Z
started_at: 2026-09-27T07:16:06.962Z
---
Apply the maintainer-approved 2026-09-27 policy: retire the representative 512 MiB and 10-second blockers; require manageable reference-corpus footprint, observation-density scaling and a 100 GiB projection within 25% of physical RAM; track process-wide admission safety before 0.1 and hybrid spill as a follow-up. Update the plan, QA, release dependencies and guidance consistently. Keep private corpus measurements local. This bead is policy and planning only; runtime safety and scale proofs remain separate open deliverables.

## Notes

Policy and documentation review: no unresolved findings. The accepted criteria distinguish raw bytes from retained state, projection from measurement, and planned process-wide admission from current per-agent row-shell enforcement. Existing small synthetic gates remain intact; G1 and full accounting/parity QA remain separate from Phase 2 to avoid circular acceptance. Review fixed stale links, privacy/evidence storage wording and executable QA placeholders. Local link/anchor and footer checks passed; all 16 QA shell blocks parse. Full make check is in progress, with Rust/QA/parity/scale checks passed and negative gate probes running. Runtime work remains explicitly open under uro-6pi8 and uro-z1h1; spill is follow-up uro-924y.
