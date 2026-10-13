---
type: is
id: is-01m2ksz138y5h2rwsesrv1eve7
title: Add release.yml publishing to GitHub Releases, crates.io and PyPI
kind: task
status: in_progress
priority: 1
version: 10
spec_path: docs/project/specs/active/plan-2026-09-16-first-release-publishing.md
labels:
  - release
dependencies:
  - type: blocks
    target: is-01m2ksz2s56jbdehka0ddtx51j
  - type: blocks
    target: is-01m2ksz57qjv1wkzygfgynae26
parent_id: is-01m2ksyxh3c3qvgg8qhp5fgbsx
created_at: 2026-09-16T00:30:44.327Z
updated_at: 2026-10-13T09:48:39.480Z
---
Implement one release workflow for GitHub Releases, crates.io and PyPI.\n\nAcceptance:\n- Pinned Cargo 1.90 or newer runs cargo package --locked --workspace and cargo publish --locked --workspace so Cargo verifies the full set and orders urollup-core before urollup; non-atomic partial publication is recoverable.\n- release.yml stages validated archives, SHA256SUMS, wheels and an immutable artifact manifest, then records post-publish channel results separately in release evidence.\n- PyPI publishes only the real Maturin bindings=bin wheels. Wheel tests inspect the script payload and RECORD, install into an empty environment, resolve urollup on PATH to the wheel binary, and exercise exact-version uvx plus persistent uv tool install.\n- PyPI uses a pending trusted publisher. The first crates.io upload uses one shortest-expiry token with only publish-new and exact urollup-core and urollup name scopes, then both crates move to trusted-publishing-only mode and the token is revoked.\n- A dispatch rehearsal skips only external writes. Tag publishing runs from a protected release environment. Independent channel jobs resume identical partial state and stop on conflicts or unknown registry state.\n- Build jobs are read-only, actions and release tools are immutably pinned, and publish authority is scoped per job. No wrapper, downloader, sdist, empty bindings package, Homebrew, npm, cargo-binstall, musllinux or Windows arm64 channel is added.

## Notes

Packaging review 2026-09-16: make the PyPI urollup artifact a maturin bindings=bin wheel that contains the Rust executable, and smoke-test both exact-version uvx execution and uv tool install from the built wheel. Keep that CLI distribution separate from future importable Python bindings tracked by uro-8vvf, so the native CLI remains Python-independent and the extension can choose its own ABI/matrix. Do not add a Python downloader wrapper or npm postinstall path.

2026-10-13: Phase 1 (credential-free release path) in progress on branch feat/release-rehearsal: tested scripts/release*.mjs, .github/workflows/release.yml with a dispatch rehearsal and tag-gated publish jobs, make release-check (in make check, with gate probes) and make release-rehearsal. No credentials, trusted publishers or tags are configured; publication waits on milestone acceptance.

2026-10-13 update: PR #36 (https://github.com/jlevy/urollup/pull/36, head 0a9d5b8) implements Phase 1; dispatch rehearsal run 38377526193 passed: plan, make check gate, 10 builds, 10 native validations, crates.io packaging rehearsal and assemble, with every tag-only job skipped. PR CI passed. Awaiting maintainer review and the Phase 2 setup listed in docs/publishing.md; not merged.
