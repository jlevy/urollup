---
type: is
id: is-01m5nfyzfnv9tg6gjkhvjgbegb
title: Keep worker line buffers from regrowing process memory on macOS
kind: bug
status: open
priority: 1
version: 1
spec_path: docs/project/specs/active/plan-2026-10-23-alpha-stabilization.md
labels:
  - memory
  - alpha
  - milestone-0.1
dependencies: []
parent_id: is-01m5nfwqwjgaf1665db3fhvaan
created_at: 2026-10-23T23:01:31.252Z
updated_at: 2026-10-23T23:01:31.252Z
---
reader.rs:991-997 (LineBuffer::reset) shrinks a grown buffer back to 256 KiB after each long line, and a new LineBuffer is made per scan, peek and first-record re-read (reader.rs:551, 694, 1179). With the macOS system allocator each shrink-and-regrow leaves memory behind: twenty 24 MB lines peaked near 490 MB and --max-ram 2M over 24 such lines peaked at 822 MB RSS and exited 0; keeping capacity stays flat. This contradicts 'worker line buffers are bounded' (uro-1sm8) and the admission model's assumption that releasing the large-record permit frees its buffers. Fix: one retained buffer per worker, or a process-wide large-record buffer owned by the permit; add a macOS footprint regression with many lines over 8 MiB; check glibc.
