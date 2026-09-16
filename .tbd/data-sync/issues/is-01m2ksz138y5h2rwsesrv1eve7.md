---
type: is
id: is-01m2ksz138y5h2rwsesrv1eve7
title: Add release.yml publishing to GitHub Releases, crates.io and PyPI
kind: task
status: open
priority: 1
version: 7
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
updated_at: 2026-09-16T16:54:44.999Z
---
One workflow publishes the first release through the three selected channels. Focused plan: Build, Validate, and Publish Flow.\n\nAcceptance:\n- release.yml stages validated GitHub archives, SHA256SUMS, wheels and a release manifest; publishes urollup-core and urollup to crates.io in one dependency-ordered invocation; and publishes only the real PyPI urollup Maturin bindings=bin wheels.\n- Exact-version uvx and persistent uv tool install are smoke-tested from local wheels before publication and from PyPI afterward. No Python downloader wrapper, importable module, npm postinstall path or sdist is added.\n- PyPI uses a pending trusted publisher for the first release. crates.io uses one expiring bootstrap token for the first upload, then both crates move to trusted-publishing-only mode and the token is revoked.\n- A dispatch rehearsal skips only external writes. Tag publishing runs from a protected release environment. Reruns skip byte-identical published files, resume missing channels and fail on checksum conflicts.\n- Build jobs are read-only, actions and release tools are immutably pinned, and publish authority is scoped per job. Homebrew, npm, cargo-binstall, musllinux wheels and Windows arm64 wait for demand.\n- urollup and urollup-core were unregistered on crates.io, and urollup was unregistered on PyPI, on 2026-09-13. Name availability is rechecked before tagging; no empty PyPI urollup-core placeholder is published.

## Notes

Packaging review 2026-09-16: make the PyPI urollup artifact a maturin bindings=bin wheel that contains the Rust executable, and smoke-test both exact-version uvx execution and uv tool install from the built wheel. Keep that CLI distribution separate from future importable Python bindings tracked by uro-8vvf, so the native CLI remains Python-independent and the extension can choose its own ABI/matrix. Do not add a Python downloader wrapper or npm postinstall path.
