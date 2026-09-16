---
type: is
id: is-01m2kszrh0aap5bsep4j7mcnkz
title: Add Homebrew, npm, cargo-binstall and Windows arm64 releases (on demand)
kind: feature
status: open
priority: 3
version: 1
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - later
dependencies: []
parent_id: is-01m2kszagjf5mxhe33418nswt5
created_at: 2026-09-16T00:31:08.319Z
updated_at: 2026-09-16T00:31:08.319Z
---
On demand, not scheduled: additional release channels and targets beyond the confirmed release scope. Design Decision 27; plan Rollout Plan; §10.6 records npm and Nix installation as intentionally unsupported for now.

Acceptance (when taken up):
- Each channel is added only on demand, under the same supply-chain policy, trusted publishing and attestation rules as the initial channels.
- Windows arm64 is built and smoke-tested on a native runner like the other targets.
