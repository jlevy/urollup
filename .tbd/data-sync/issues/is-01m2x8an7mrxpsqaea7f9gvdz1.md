---
type: is
id: is-01m2x8an7mrxpsqaea7f9gvdz1
title: Decode Claude usage lines without parse_record
kind: task
status: open
priority: 0
version: 4
spec_path: docs/project/specs/active/plan-2026-09-16-scalable-ingestion.md
labels:
  - milestone-0.1
  - performance
  - memory
dependencies:
  - type: blocks
    target: is-01m2wbbfnm8gs10hrmyrg14tma
  - type: blocks
    target: is-01m2x8ap0h71f5mpry8b57wb3k
  - type: blocks
    target: is-01m2x8apcp21hd5yacp01m0a9b
parent_id: is-01m2pkgts2b87n25929xphbnpc
created_at: 2026-09-19T16:34:55.347Z
updated_at: 2026-09-19T16:35:15.726Z
---
The remaining whole-history peak is Claude ingest while the compact Codex ledger is still resident. LineHead already proves a usage-bearing line is valid JSON, then SourceDecoder::decode calls parse_record and keeps a serde_json::Value for the whole assistant or progress document — including message content accounting never reads. That is ~413k Claude usage lines, eight workers at once.

Replace the second pass with a borrowed typed body in the same style as Codex Line.

Files and functions:
- crates/urollup-core/src/adapters/claude_project/line.rs: extend LineHead or add LineBody; custom DeserializeSeed visitors (not Value::deserialize). Read sessionId, uuid, requestId, timestamp, effort, cwd, message.{id,model,usage,content}, quotaLimits, isApiErrorMessage, and progress data.message.
- crates/urollup-core/src/adapters/claude_project.rs: SourceDecoder::decode (the parse_record call at the bears_usage branch), SourceDecoder::record, record_extras, advisor_usage, quota_limits, tool_use_ids, usage_limit_text. Keep QuotaLimits.native as compact sorted JSON text without a parent Value.
- crates/urollup-core/src/sources/decode.rs: parse_record stays as the equivalence oracle for line.rs tests, not the hot path.

Acceptance: no parse_record (or serde_json::Value document) on the Claude decode hot path; worker-count identity and Claude file-rename tests still pass; fixture snapshots and goldens stay semantically identical; make check; privacy-safe whole-history RSS recorded on uro-l0gd.
