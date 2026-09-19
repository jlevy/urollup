---
type: is
id: is-01m2xsj6xgcxet8hy8dxcbeyh4
title: Enlarge sequential read window for source scans
kind: task
status: closed
priority: 1
version: 5
spec_path: docs/project/specs/active/plan-2026-09-16-scalable-ingestion.md
delegate: claude-code@spud10.local
labels:
  - milestone-0.1
  - performance
dependencies: []
parent_id: is-01m2pkgv1mh7268dh4sxbptdmg
hold: null
hold_until: null
created_at: 2026-09-19T21:36:08.623Z
updated_at: 2026-09-19T21:39:26.203Z
started_at: 2026-09-19T21:36:13.759Z
closed_at: 2026-09-19T21:39:26.202Z
close_reason: 1 MiB BufReader on reader_for. Quiet WH 635 MiB / 21.2 s vs lsaz remasure 648 / 18.2 (wall rose). Codex-only 602 MiB / 13.7 s vs 573 / 14.3 (peak rose). posix_fadvise not added. Reverted to 128 KiB.
resolution: canceled
duplicate_of: null
---
Phase 2 I/O leftover named by the uro-lsaz profile. Codex worker samples are ~40% kernel read in reader.rs fill_buf. BufReader is 128 KiB today. Not mmap. Not another validate_record replacement (uro-s5vb).

Raise the sequential read window on crates/urollup-core/src/sources/reader.rs reader_for (plain and zstd). posix_fadvise(SEQUENTIAL) only if it can be issued without new unsafe in this crate and without a new dependency; Darwin often no-ops fadvise, so the window is the cut that can move wall on the reference laptop.

Files: crates/urollup-core/src/sources/reader.rs ReadOptions / reader_for.

Acceptance: release WH and Codex-only wall fall; peak does not rise; reader and adapter tests stay green. Revert if wall or peak rises.
