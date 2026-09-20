---
type: is
id: is-01m2ymcxkb4yxschcfx5yh1vyp
title: Audit and trash obsolete stabilization build targets
kind: chore
status: closed
priority: 2
version: 3
spec_path: docs/project/reviews/review-2026-09-19-pr-stack-and-memory.md
labels: []
dependencies: []
parent_id: is-01m2y7eh0genxhf98p3h08fmn9
created_at: 2026-09-20T05:25:06.794Z
updated_at: 2026-09-20T05:26:16.518Z
closed_at: 2026-09-20T05:26:16.517Z
close_reason: Scoped audit complete; two obsolete urollup Cargo targets staged with trash and verified. Active or task-owned targets preserved; Trash left unemptied.
resolution: null
duplicate_of: null
---
Audit repository and /private/tmp Cargo targets, check process and task activity, preserve active builds and unique validation outputs, stage only obsolete reproducible artifacts with trash, and record destination and physical-space evidence.

## Notes

Audited repository and /private/tmp Cargo targets using allocated du sizes, complete ps/lsof snapshots and task state. Rechecked immediately before trash. Staged /Users/levy/wrk/github/urollup/target/gate-proofs (1,212,308 KiB) to /Users/levy/.Trash/gate-proofs; recreate with make gate-proofs. Staged /Users/levy/wrk/github/urollup/.claude/worktrees/agent-a65e8a6037cc8fd86/attic/probe-target (193,944 KiB) to /Users/levy/.Trash/probe-target; retained attic old-src/new-src manifests and locks to recreate via CARGO_TARGET_DIR and cargo build. Both untracked generated Cargo outputs, no active handles/processes. Total allocated directory sizes: 1,406,252 KiB (1.34 GiB); not a claim of uniquely reclaimable APFS bytes. Verified destinations with trash -l -v, originals absent. Kept current debug/release targets, parity reports, logs, sources and worktree registrations. No urollup Cargo targets remain under /private/tmp from this audit. Preserved FDU targets because related tasks and a Windows Cargo build are active; retained Squares target for its unarchived review task. Physical available space changed from 7,983,984 to 7,597,080 KiB during concurrent system work; moving to Trash frees no physical blocks. Existing target and concurrently appearing iTunes Trash entries were not touched. Trash remains unemptied. Local machine-readable inventory/audit/receipt: /private/tmp/urollup-build-target-{inventory,audit,trash-receipt}.json.
