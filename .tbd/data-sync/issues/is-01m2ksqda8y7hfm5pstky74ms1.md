---
type: is
id: is-01m2ksqda8y7hfm5pstky74ms1
title: Add ccusage gemini parity cases and ledger entries
kind: task
status: open
priority: 2
version: 1
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - phase-2
dependencies: []
parent_id: is-01m2f0tdnzmd4d9fy3afh49bfx
created_at: 2026-09-16T00:26:34.689Z
updated_at: 2026-09-16T00:26:34.689Z
---
Add ccusage gemini cases to the ccusage reconciliation harness once the Gemini adapters land (design 10.6, plan "ccusage reconciliation harness", Phase 2 row).

- Cases: ccusage gemini daily, monthly and session, with and without --since, on the synthetic gemini-session fixtures, with GEMINI_DATA_DIR pointed at the fixture copy's tmp root, plus the unified ccusage daily run.
- Seed ledger entries, all sourced from the source reviews brief "ccusage Gemini CLI parsing" (ccusage 20.0.20, commit bd7f89b):
  - dedupe: ccusage deduplicates by message id within one file only, so a bucket migration copy, a legacy .json beside its .jsonl, and a --session-file import each count twice.
  - semantics: ccusage treats input as excluding cached unless the record's total equals its parts, so a record with tool-use prompt tokens counts cached tokens as both input and cache read.
  - semantics: ccusage has no project grouping for Gemini (project is the constant "gemini"), subagent files become their own sessions, and a record with an unparsable timestamp falls back to the file mtime.
  - unsupported: Gemini usage that reaches only OpenTelemetry (compaction, routing and other utility roles) appears in neither tool.
- Each entry needs its retirement condition; ccusage main at d341949 still carries no Gemini parsing change, so none retires yet.
