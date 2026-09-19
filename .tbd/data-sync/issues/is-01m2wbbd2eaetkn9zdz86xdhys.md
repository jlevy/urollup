---
type: is
id: is-01m2wbbd2eaetkn9zdz86xdhys
title: Drop boxed Claude per-record native IDs
kind: task
status: open
priority: 1
version: 2
spec_path: docs/project/specs/active/plan-2026-09-16-scalable-ingestion.md
labels:
  - milestone-0.1
  - performance
  - memory
dependencies:
  - type: blocks
    target: is-01m2wbbfnm8gs10hrmyrg14tma
parent_id: is-01m2pkgts2b87n25929xphbnpc
created_at: 2026-09-19T08:08:31.053Z
updated_at: 2026-09-19T08:08:38.335Z
---
Claude ParsedRecord still holds request_id: Option<Box<str>> and MessageId.text: Box<str> for every decoded record while the corpus waits to normalize. Identity already uses digests. Drop the boxed native text from the hot record type and keep digests (and interned names) only.

Acceptance: Claude ParsedRecord stays at or under its size budget without those boxes; owner-map and fixture results unchanged; make check.
