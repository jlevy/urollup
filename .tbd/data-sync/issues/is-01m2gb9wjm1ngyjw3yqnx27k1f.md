---
type: is
id: is-01m2gb9wjm1ngyjw3yqnx27k1f
title: "Spike: measure local log volume and parse throughput to phase the capture cache"
kind: task
status: closed
priority: 1
version: 5
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
delegate: claude-code@spud10.local
labels:
  - spike
dependencies:
  - type: blocks
    target: is-01m2gb9xfat93wrwm1m3gy87vd
  - type: blocks
    target: is-01m2gedk1zrfwcgqqm4ajvx6wj
parent_id: is-01m2ew7x8t5sh3h5997e0rz0t2
hold: null
hold_until: null
created_at: 2026-09-14T16:16:48.211Z
updated_at: 2026-09-14T19:56:21.448Z
started_at: 2026-09-14T16:16:54.292Z
closed_at: 2026-09-14T19:56:21.447Z
close_reason: "Spike complete: M1 Pro, 19.5 GB of local logs; uncached prefiltered typed parse covers past 30 days in ~3 s and all history in ~7 s on 10 threads under heavy load; capture cache 340 MB (1.7%), ~1.4x faster on the month and ~1.9x on all history; recommend uncached engine in Phase 1, bundles write captured records, default-on cache later unless retention is wanted sooner. Code kept in explorations/log-throughput."
resolution: null
duplicate_of: null
---
Measure the volume and composition of this machine's Claude Code (~/.claude/projects) and Codex (~/.codex/sessions) logs, overall and for the past 30 days, and prototype uncached parse throughput (streaming line parse extracting usage fields, single and multi-threaded, with and without a byte prefilter) versus reading content-stripped zstd JSONL captured records. Record only aggregate numbers, never content, paths or IDs. Outcome: a research brief with measurements and a recommendation on whether the default-on capture cache belongs in Phase 1 or a later phase.
