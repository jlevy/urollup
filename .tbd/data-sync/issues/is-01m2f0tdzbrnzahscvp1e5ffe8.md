---
type: is
id: is-01m2f0tdzbrnzahscvp1e5ffe8
title: Scaffold repository to the engineering baseline
kind: task
status: open
priority: 2
version: 9
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - phase-1
  - milestone-0.1
dependencies:
  - type: blocks
    target: is-01m2f0te8wwa4jmh6zzamt27zb
  - type: blocks
    target: is-01m2f0tej82vk57g0eaapfr88r
  - type: blocks
    target: is-01m2keezphdvk2jf3rj921mws0
  - type: blocks
    target: is-01m2keggm34ksgr8bfxr9sr7jz
parent_id: is-01m2ke36qgfvdvnhw7c7v5esm6
created_at: 2026-09-14T03:54:21.546Z
updated_at: 2026-09-15T21:10:34.343Z
---
Workspace (urollup-core, urollup), toolchain pin, lint and format configuration, make check and make fix, CI workflows, supply-chain policy, npm dev project, pytest and flowmark-rs in the uv project, bench/ layout and AGENTS.md routes; prove each gate fails on a committed violation. See plan: Project setup and engineering conventions.

## Notes

Findings from the 2026-09-14 source reviews (ccusage, Codex, Pi, agentfdr, session-report, squares, metaproc); rules are specified in the plan and data contracts, and sources are in the research briefs.
- THIRD-PARTY-NOTICES with ccusage and agentfdr MIT texts, Pi MIT text, and Codex and Anthropic Apache-2.0 notices for ported code; header convention naming source path and commit.
- Provenance notice recording the source repository and commit for every file, test or fixture ported from jlevy repos (metaproc, squares, metabrowser).
- CI fails when a golden snapshot is missing; extend the supply-chain validator to workflow permissions and waiver expiry; keep unwinding in the release profile.

- Serving separability (confirmed 2026-09-14): two crates only; the `serve` module in crates/urollup is behind a default-on `serve` Cargo feature with optional HTTP, async-runtime and web-asset dependencies (dep: gated), following fdu's deletable-feature pattern; add a CI job that builds and tests `urollup --no-default-features` and fails if core or that tree contains HTTP or async-runtime crates.
