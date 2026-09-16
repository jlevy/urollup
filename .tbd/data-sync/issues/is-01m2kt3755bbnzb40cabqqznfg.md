---
type: is
id: is-01m2kt3755bbnzb40cabqqznfg
title: Add build-provenance attestations and tag-checked SemVer
kind: task
status: closed
priority: 1
version: 3
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - release
dependencies:
  - type: blocks
    target: is-01m2kt39m8dys95na8979brga2
parent_id: is-01m2kt3407fwqsknjr8nvra2sy
created_at: 2026-09-16T00:33:01.603Z
updated_at: 2026-09-16T03:01:58.216Z
closed_at: 2026-09-16T03:01:58.215Z
close_reason: Created twice when the bead-mapping run was interrupted and rerun; uro-co0m is the original.
resolution: duplicate
duplicate_of: is-01m2ksz2s56jbdehka0ddtx51j
---
Rollout: verification and versioning. Plan Rollout Plan; design §5.6.

Acceptance:
- Every archive and wheel carries a build-provenance attestation produced from the tagged commit's --locked build; no GPG or minisign signature.
- SemVer comes from [workspace.package] version and build.rs checks it against the tag, failing the build on a mismatch.
- Report, bundle, query and identity contract versions are independent of the package version and recorded in CHANGELOG.md; before 1.0 a minor release may change CLI or JSON contracts.
- A verification runbook step reproduces an archive's digest from the tag.
