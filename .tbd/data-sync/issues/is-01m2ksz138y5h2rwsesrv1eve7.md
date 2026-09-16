---
type: is
id: is-01m2ksz138y5h2rwsesrv1eve7
title: Add release.yml publishing to GitHub Releases, crates.io and PyPI
kind: task
status: open
priority: 1
version: 4
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - release
dependencies:
  - type: blocks
    target: is-01m2ksz2s56jbdehka0ddtx51j
  - type: blocks
    target: is-01m2ksz57qjv1wkzygfgynae26
parent_id: is-01m2ksyxh3c3qvgg8qhp5fgbsx
created_at: 2026-09-16T00:30:44.327Z
updated_at: 2026-09-16T16:40:35.920Z
---
Rollout: one workflow, three channels. Plan Rollout Plan; Decision 27.

Acceptance:
- One release.yml publishes from a protected `release` environment to GitHub Release archives with SHA256SUMS (the channel the cloud reporting skill pins), crates.io urollup-core and urollup in one invocation, and PyPI binary wheels.
- Registries use trusted publishing, except a short-lived scoped token for the first crates.io upload.
- A dispatch dry run skips only upload; reruns skip identical artifacts and fail on different bytes under one version.
- Homebrew, npm and cargo-binstall wait for demand.
- Workflows use read-only permissions, SHA-pinned actions and --locked builds.
- urollup and urollup-core were unregistered on crates.io and PyPI on 2026-09-13, so first publication claims both names.

## Notes

Packaging review 2026-09-16: make the PyPI urollup artifact a maturin bindings=bin wheel that contains the Rust executable, and smoke-test both exact-version uvx execution and uv tool install from the built wheel. Keep that CLI distribution separate from future importable Python bindings tracked by uro-8vvf, so the native CLI remains Python-independent and the extension can choose its own ABI/matrix. Do not add a Python downloader wrapper or npm postinstall path.
