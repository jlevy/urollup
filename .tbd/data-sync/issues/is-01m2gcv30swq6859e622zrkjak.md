---
type: is
id: is-01m2gcv30swq6859e622zrkjak
title: Review ccusage source for reusable Rust parsing, dedupe and pricing code
kind: task
status: closed
priority: 1
version: 3
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
delegate: claude-code@spud10.local
labels:
  - code-review
dependencies: []
parent_id: is-01m2ew7x8t5sh3h5997e0rz0t2
hold: null
hold_until: null
created_at: 2026-09-14T16:43:40.439Z
updated_at: 2026-09-14T17:10:13.747Z
started_at: 2026-09-14T16:43:55.680Z
closed_at: 2026-09-14T17:10:13.745Z
close_reason: "Review complete (notes in scratchpad/reviews/ccusage.md): portable MIT Rust pieces (ordered parallel file reader, line prefilter, table rendering, fs_fixture macro), Claude dedupe and Codex delta logic to rebuild order-independently, ~600 tests to adapt; corrections: advisor iterations excluded from top-level usage, progress records nest subagent messages, /btw replays, dedupe change at a4b8420; pitfalls: nested null drops usage, divergent daily parser, strict timestamp parsing, runtime LiteLLM fetch, guessed service tier."
resolution: null
duplicate_of: null
---
Review the read-only clone attic/ccusage at bd7f89b (MIT; Rust workspace crates ccusage-core, ccusage-adapter-all, ccusage-cli and others) for Rust code urollup can port with attribution: Claude and Codex (and other agent) adapters, dedupe keys, cumulative counter handling, fork and replay handling, pricing data loading, report rendering, tests and fixtures. Note known bugs and divergences from urollup's contracts.
