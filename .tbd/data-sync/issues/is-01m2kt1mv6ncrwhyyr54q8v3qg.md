---
type: is
id: is-01m2kt1mv6ncrwhyyr54q8v3qg
title: Add scripts/check_contracts.py and the make contracts-check gate
kind: task
status: closed
priority: 2
version: 3
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - phase-1
  - milestone-0.2
dependencies:
  - type: blocks
    target: is-01m2kt23gtg5e014w43p0a46c9
parent_id: is-01m2ke45tasy262pas37jxwss5
created_at: 2026-09-16T00:32:10.084Z
updated_at: 2026-09-16T03:01:58.647Z
closed_at: 2026-09-16T03:01:58.646Z
close_reason: Created twice when the bead-mapping run was interrupted and rerun; uro-9kf6 is the original.
resolution: duplicate
duplicate_of: is-01m2ksvkp4cn5y43dqkq8feqj3
---
Milestone 0.2: implement the tested contract gate script and wire it into make and CI. Design §5.7 and Decision 25.

Acceptance:
- scripts/check_contracts.py runs `uv --config-file uv.toml run --frozen softschema compile <model> --contract <id> --out <schema> --check` for every contract, including each table record contract.
- It requires non-empty valid and invalid fixture lists, requires softschema validate to accept every valid fixture and to reject every invalid one with a validation verdict, and treats a tool failure as a gate failure rather than a pass (an inline `! softschema validate` passes when uv cannot find softschema or a glob matches nothing, which is why the script exists).
- It runs `uv --config-file uv.toml run --frozen pytest contracts`.
- The script itself is unit-tested; `make contracts-check` calls it rather than a Makefile shell loop, and CI runs the same target.
- A committed stale-schema probe proves the gate fails, following the scaffold's gate-proof pattern (uro-phi8).
