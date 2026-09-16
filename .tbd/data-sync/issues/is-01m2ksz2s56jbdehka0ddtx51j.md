---
type: is
id: is-01m2ksz2s56jbdehka0ddtx51j
title: Add build-provenance attestations and tag-checked SemVer
kind: task
status: open
priority: 1
version: 4
spec_path: docs/project/specs/active/plan-2026-09-16-first-release-publishing.md
labels:
  - release
dependencies:
  - type: blocks
    target: is-01m2ksz7d707rz38cfe8gh2xbj
parent_id: is-01m2ksyxh3c3qvgg8qhp5fgbsx
created_at: 2026-09-16T00:30:46.051Z
updated_at: 2026-09-16T16:54:16.217Z
---
Rollout: verification and versioning. Plan Rollout Plan; design §5.6.

Acceptance:
- Every archive and wheel carries a build-provenance attestation produced from the tagged commit's --locked build; no GPG or minisign signature.
- SemVer comes from [workspace.package] version and build.rs checks it against the tag, failing the build on a mismatch.
- Report, bundle, query and identity contract versions are independent of the package version and recorded in CHANGELOG.md; before 1.0 a minor release may change CLI or JSON contracts.
- A verification runbook step reproduces an archive's digest from the tag.
