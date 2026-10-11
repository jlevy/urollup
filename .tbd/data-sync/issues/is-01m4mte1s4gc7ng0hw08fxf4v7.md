---
type: is
id: is-01m4mte1s4gc7ng0hw08fxf4v7
title: "Fix PR #20 review findings before the Cursor adapter lands"
kind: task
status: open
priority: 3
version: 1
spec_path: docs/project/specs/active/plan-2026-09-19-cursor-dialect.md
labels:
  - post-0.1
dependencies: []
parent_id: is-01m2y1qw9mgfwsbpdgam8nn13r
created_at: 2026-10-11T06:29:34.627Z
updated_at: 2026-10-11T06:29:34.627Z
---
PR #20 (closed 2026-10-10, branch cursor-state-adapter at 2ecf386; plan on branch cursor-dialect at 2d4134a) has two formal reviews: A (senior) https://github.com/jlevy/urollup/pull/20#pullrequestreview-5481992270 and B (security) https://github.com/jlevy/urollup/pull/20#pullrequestreview-5482002310. Blockers and High findings to fix before the adapter can land (behind a default-off cursor feature if the SQLite dependency is accepted, uro-kddq): admission never charges Cursor decode (B2, A11); SQLite hardening against hostile stores (trusted_schema=OFF, defensive mode, ordinary tables only, cache/mmap/length limits, progress handler) (B1); reads are not one snapshot and create -wal/-shm in Cursor's directory (A12); an explicit --source file is read whole (A2); unrelated selections decode the whole store (A3); CURSOR_CONVERSATION_ID makes Claude --current ambiguous (A4); composer model fallback must become unknown (A5, maintainer decision); totals depend on root order (A6); shared requestId keeps only the last bubble and identical bubbles double count (A7); coverage reported complete with dropped orphans (A8, B4); fixtures never exercise the SQLite path (A9); --session child lookup (B3); whole-run failure on one bad store (B5); plus the Medium and Low items in both reviews. --agent is out of scope (stays 0.5).
