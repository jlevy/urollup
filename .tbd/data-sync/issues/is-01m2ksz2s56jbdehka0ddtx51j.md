---
type: is
id: is-01m2ksz2s56jbdehka0ddtx51j
title: Add build-provenance attestations and tag-checked SemVer
kind: task
status: in_progress
priority: 1
version: 6
spec_path: docs/project/specs/active/plan-2026-09-16-first-release-publishing.md
labels:
  - release
dependencies:
  - type: blocks
    target: is-01m2ksz7d707rz38cfe8gh2xbj
parent_id: is-01m2ksyxh3c3qvgg8qhp5fgbsx
created_at: 2026-09-16T00:30:46.051Z
updated_at: 2026-10-13T09:48:40.256Z
---
Rollout: verification and versioning. Plan Rollout Plan; design §5.6.

Acceptance:
- Every archive and wheel carries a build-provenance attestation produced from the tagged commit's --locked build; no GPG or minisign signature.
- SemVer comes from [workspace.package] version and build.rs checks it against the tag, failing the build on a mismatch.
- Report, bundle, query and identity contract versions are independent of the package version and recorded in CHANGELOG.md; before 1.0 a minor release may change CLI or JSON contracts.
- A verification runbook step reproduces an archive's digest from the tag.

## Notes

2026-10-13: Phase 1 (credential-free release path) in progress on branch feat/release-rehearsal: tested scripts/release*.mjs, .github/workflows/release.yml with a dispatch rehearsal and tag-gated publish jobs, make release-check (in make check, with gate probes) and make release-rehearsal. No credentials, trusted publishers or tags are configured; publication waits on milestone acceptance.

2026-10-13 update: PR #36 (https://github.com/jlevy/urollup/pull/36, head 0a9d5b8) implements Phase 1; dispatch rehearsal run 38377526193 passed: plan, make check gate, 10 builds, 10 native validations, crates.io packaging rehearsal and assemble, with every tag-only job skipped. PR CI passed. Awaiting maintainer review and the Phase 2 setup listed in docs/publishing.md; not merged.
