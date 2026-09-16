---
type: is
id: is-01m2kt36757fjhcxh84pdrjzsa
title: Add release.yml publishing to GitHub Releases, crates.io and PyPI
kind: task
status: closed
priority: 1
version: 4
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - release
dependencies:
  - type: blocks
    target: is-01m2kt3755bbnzb40cabqqznfg
  - type: blocks
    target: is-01m2kt38db9ackwph9949dw1ew
parent_id: is-01m2kt3407fwqsknjr8nvra2sy
created_at: 2026-09-16T00:33:00.640Z
updated_at: 2026-09-16T03:01:57.765Z
closed_at: 2026-09-16T03:01:57.764Z
close_reason: Created twice when the bead-mapping run was interrupted and rerun; uro-080d is the original.
resolution: duplicate
duplicate_of: is-01m2ksz138y5h2rwsesrv1eve7
---
Rollout: one workflow, three channels. Plan Rollout Plan; Decision 27.

Acceptance:
- One release.yml publishes from a protected `release` environment to GitHub Release archives with SHA256SUMS (the channel the cloud reporting skill pins), crates.io urollup-core and urollup in one invocation, and PyPI binary wheels.
- Registries use trusted publishing, except a short-lived scoped token for the first crates.io upload.
- A dispatch dry run skips only upload; reruns skip identical artifacts and fail on different bytes under one version.
- Homebrew, npm and cargo-binstall wait for demand.
- Workflows use read-only permissions, SHA-pinned actions and --locked builds.
- urollup and urollup-core were unregistered on crates.io and PyPI on 2026-09-13, so first publication claims both names.
