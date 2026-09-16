---
type: is
id: is-01m2ksz7d707rz38cfe8gh2xbj
title: Define and run the pre-1.0 release checklist and shadow-mode validation
kind: task
status: open
priority: 2
version: 1
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - release
dependencies: []
parent_id: is-01m2ksyxh3c3qvgg8qhp5fgbsx
created_at: 2026-09-16T00:30:50.788Z
updated_at: 2026-09-16T00:30:50.788Z
---
Rollout: the gate before binaries with the embedded UI are published. Plan Rollout Plan.

Acceptance:
- A checklist requires fixture, packaged-binary, feature-matrix and Testing Strategy release checks to pass before publishing, and is run for each release.
- urollup first runs in shadow mode against retained logs and existing reports, which stay in use until supported reports are validated; every difference is explained from source records.
- Dependencies stay pinned under the supply-chain policy as crates and frontend tools are chosen, and every deny.toml ignore names a bead and a removal condition.
- A release missing a reference-laptop performance gate ships only with the target revised from recorded results.
