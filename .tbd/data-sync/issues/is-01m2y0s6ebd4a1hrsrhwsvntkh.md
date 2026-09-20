---
type: is
id: is-01m2y0s6ebd4a1hrsrhwsvntkh
title: Replace the 2 GiB ingest fuse with a RAM-relative ceiling
kind: task
status: in_progress
priority: 1
version: 2
spec_path: docs/project/specs/active/plan-2026-09-16-scalable-ingestion.md
labels:
  - milestone-0.1
  - memory
dependencies: []
parent_id: is-01m2p3evvpmcn8cf59wc1196f6
created_at: 2026-09-19T23:42:17.545Z
updated_at: 2026-09-19T23:42:26.596Z
---
User-facing ingest capacity replaces the fixed 2 GiB compact-row ceiling.

Default is 25% of physical RAM, falling back to 2 GiB when RAM cannot be read (CI/containers). `--max-ram` accepts a byte size (512M, 8G, 8GiB) or a percent (25%). `--max-rows N` is an exact per-agent row-count override. When both are set, the stricter (smaller) ceiling wins. Optional `UROLLUP_MAX_RAM` is parsed like other UROLLUP_* overrides and is replaced by `--max-ram`.

Keep the same failure mode: exit 1, diagnostic that names the limit and suggests narrower `--source` / `--no-default-sources`. Accounting stays per-agent. Convert the byte budget with `budget / size_of(RequestObservation)`.

Do not retry canceled 512 MiB field-shrink experiments.
