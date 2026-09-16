---
type: is
id: is-01m2ksz57qjv1wkzygfgynae26
title: Write README, CHANGELOG and the release publishing runbook
kind: task
status: open
priority: 2
version: 5
spec_path: docs/project/specs/active/plan-2026-09-16-first-release-publishing.md
labels:
  - release
  - docs
dependencies: []
parent_id: is-01m2ksyxh3c3qvgg8qhp5fgbsx
created_at: 2026-09-16T00:30:48.518Z
updated_at: 2026-09-16T18:22:59.203Z
---
Release documentation for the first public alpha.\n\nAcceptance:\n- README states what urollup is and is not, install instructions for each supported channel, the 0.1.0 command surface, known limitations and the privacy posture.\n- CHANGELOG.md records 0.1.0 and, separately, its report, bundle, query and identity contract versions.\n- The publishing runbook covers the protected release environment, credential-free rehearsal, rerun rules, PyPI pending publishing, the narrowly scoped first crates.io token, trusted-publishing-only migration and partial-channel recovery.\n- Release notes describe the aggregate first-release feature set and known limitations, not bugs introduced and fixed before publication.\n- Docs follow the repository common documentation guidelines and pass the flowmark gate in make check.

## Notes

The focused publishing spec requires README examples for uvx --isolated urollup@X.Y.Z, uv tool install urollup==X.Y.Z, cargo install --locked urollup@X.Y.Z, and verified GitHub archive download. Release notes describe the aggregate first-release feature set and known limitations, not bugs introduced and fixed before publication.
