---
type: is
id: is-01m2x8apcp21hd5yacp01m0a9b
title: Bound reused worker line buffers after large records
kind: task
status: open
priority: 2
version: 1
spec_path: docs/project/specs/active/plan-2026-09-16-scalable-ingestion.md
labels:
  - milestone-0.1
  - performance
  - memory
dependencies: []
parent_id: is-01m2pkgts2b87n25929xphbnpc
created_at: 2026-09-19T16:34:56.533Z
updated_at: 2026-09-19T16:34:56.533Z
---
Contingency if measurement still shows decode-time RSS from worker buffers. Scan::run reuses one Vec<u8> per source via buffer.clear(); it does not reserve 64 MiB, but a worker that has seen a huge line keeps that capacity for the rest of the file, and eight workers do this at once. ReadOptions::DEFAULT_MAX_RECORD_BYTES is 64 MiB.

Files and functions:
- crates/urollup-core/src/sources/reader.rs: Scan::run, read_line, ReadOptions::DEFAULT_MAX_RECORD_BYTES / new.
- crates/urollup-core/src/sources/parallel.rs: read_in_parallel, MAX_DEFAULT_WORKERS (8).

Acceptance: a privacy-safe UROLLUP_STATS or RSS comparison shows the buffer change is the cause of a cut, or the bead is closed as unnecessary after uro-l0gd. make check. Skip if 512 MiB is already met.
