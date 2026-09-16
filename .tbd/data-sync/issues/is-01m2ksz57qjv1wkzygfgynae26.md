---
type: is
id: is-01m2ksz57qjv1wkzygfgynae26
title: Write README, CHANGELOG and the release publishing runbook
kind: task
status: open
priority: 2
version: 4
spec_path: docs/project/specs/active/plan-2026-09-16-first-release-publishing.md
labels:
  - release
  - docs
dependencies: []
parent_id: is-01m2ksyxh3c3qvgg8qhp5fgbsx
created_at: 2026-09-16T00:30:48.518Z
updated_at: 2026-09-16T16:54:45.383Z
---
Rollout: the documentation a release needs. Plan Rollout Plan; design §5.6 and §1.1.

Acceptance:
- README states what urollup is and is not, install instructions per channel, the Phase 1 command surface, and the privacy posture (artifacts carry no prompts, tool arguments or result bodies; the capture store is owner-only and local).
- CHANGELOG.md records each release's package version and, separately, its report, bundle, query and identity contract versions.
- A publishing runbook covers the protected release environment, the dispatch dry run, rerun rules, trusted-publishing setup, the first crates.io upload token, and recovery when one channel fails mid-release.
- Docs follow the repository's common documentation guidelines and pass the flowmark gate in `make check`.

## Notes

The focused publishing spec requires README examples for uvx --isolated urollup@X.Y.Z, uv tool install urollup==X.Y.Z, cargo install --locked urollup@X.Y.Z, and verified GitHub archive download. Release notes describe the aggregate first-release feature set and known limitations, not bugs introduced and fixed before publication.
