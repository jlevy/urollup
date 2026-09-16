---
type: is
id: is-01m2kt39m8dys95na8979brga2
title: Define and run the pre-1.0 release checklist and shadow-mode validation
kind: task
status: closed
priority: 2
version: 2
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - release
dependencies: []
parent_id: is-01m2kt3407fwqsknjr8nvra2sy
created_at: 2026-09-16T00:33:04.133Z
updated_at: 2026-09-16T03:02:08.891Z
closed_at: 2026-09-16T03:02:08.891Z
close_reason: Created twice when the bead-mapping run was interrupted and rerun; uro-vboa is the original.
resolution: duplicate
duplicate_of: is-01m2ksz7d707rz38cfe8gh2xbj
---
Rollout: the gate before binaries with the embedded UI are published. Plan Rollout Plan.

Acceptance:
- A checklist requires fixture, packaged-binary, feature-matrix and Testing Strategy release checks to pass before publishing, and is run for each release.
- urollup first runs in shadow mode against retained logs and existing reports, which stay in use until supported reports are validated; every difference is explained from source records.
- Dependencies stay pinned under the supply-chain policy as crates and frontend tools are chosen, and every deny.toml ignore names a bead and a removal condition.
- A release missing a reference-laptop performance gate ships only with the target revised from recorded results.
