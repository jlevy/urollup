---
type: is
id: is-01m4fsy6bqyv66kcg2sbnvgbhj
title: "PR #14 A5: choose and record the Cursor SQLite reader under supply-chain policy"
kind: bug
status: open
priority: 2
version: 5
spec_path: docs/project/specs/active/plan-2026-09-19-cursor-dialect.md
delegate: null
labels: []
dependencies:
  - type: blocks
    target: is-01m2y1rag1z513ncz2mr4k7vk9
parent_id: is-01m2y1qw9mgfwsbpdgam8nn13r
hold: null
hold_until: null
created_at: 2026-10-09T07:44:42.871Z
updated_at: 2026-10-09T07:50:57.348Z
started_at: 2026-10-09T07:45:09.948Z
---
Severity: Medium. PR #14, review A: https://github.com/jlevy/urollup/pull/14#pullrequestreview-5467142502

Deferred from PR #14 (planning gap). Blocks the adapter bead uro-p9ay.

Where: plan-2026-09-19-cursor-dialect.md Phase 1 snapshot and adapter items (:384-387); SUPPLY-CHAIN-SECURITY.md "Changing Dependencies" (:42-58) and the Reviewed Versions tables; plan-2026-09-13-urollup-cli-and-web.md Phase 3 storage item (:314-315, storage chosen after benchmarks); docs/urollup-design.md Decision 20.

Problem: decoding cursorDiskKV needs a SQLite engine in urollup-core, and the plan never names it or routes it through supply-chain policy.

Proposed dependency: rusqlite with bundled SQLite (libsqlite3-sys "bundled"), which compiles C at build time through cc, as zstd-sys already does. Alternatives to weigh: a system libsqlite3, or a pure-Rust reader.

Do:
- Run the SUPPLY-CHAIN-SECURITY.md procedure: necessity check and source/ownership inspection, a version public at least 14 days, MSRV 1.85 build and tests, cargo-deny, make supply-chain/audit/check, and a Reviewed Versions record that names the build-time C compilation.
- Record the choice under the Decision 20 exception tracked on uro-2hck (a named Cursor state-store reader, not generic database input).
- Reconcile it with the product plan's Phase 3 embedded-storage decision so the project does not end up with two embedded database engines.
- Consider an optional Cargo feature so default builds do not carry SQLite until Cursor ships; keep make dependency-guard passing.

## Notes

Deferred from PR #14 review A (tracking parent uro-l2tq). Listed in plan-2026-09-19-cursor-dialect.md under Open Questions Before Implementation at 712016e; blocks uro-p9ay. Waits on a design pass before adapter implementation; PR #14 is docs-only.
