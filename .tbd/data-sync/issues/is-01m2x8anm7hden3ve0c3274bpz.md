---
type: is
id: is-01m2x8anm7hden3ve0c3274bpz
title: Type Codex rate_limits without a Value map
kind: task
status: in_progress
priority: 0
version: 5
spec_path: docs/project/specs/active/plan-2026-09-16-scalable-ingestion.md
delegate: claude-code@spud10.local
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
hold: null
hold_until: null
created_at: 2026-09-19T16:34:55.750Z
updated_at: 2026-09-19T16:45:03.163Z
started_at: 2026-09-19T16:45:03.162Z
---
Codex Line is already borrowed and typed except rate_limits. RateLimitsSeed deserializes the object as serde_json::Value, Payload.rate_limits holds Map<String, Value>, and rate_limits() re-serializes that map to compare consecutive snapshots. Token-count lines are the Codex observation hot path.

Files and functions:
- crates/urollup-core/src/adapters/codex_rollout/line.rs: Payload.rate_limits and RateLimitsSeed. Read the object with a typed seed (limit_id/limit_name, primary, secondary, and unknown fields needed for native JSON), not Value::deserialize.
- crates/urollup-core/src/adapters/codex_rollout.rs: struct RateLimits, fn rate_limits, SourceDecoder::record_kind TokenCount arm. Keep consecutive-snapshot sharing via Arc and ProviderLimitObservation.native as compact sorted JSON text.
- Tests: identical_consecutive_rate_limits_share_one_snapshot; line.rs property tests that a Value parse and the typed seed agree.

Acceptance: no Map<String, Value> or Value document on the Codex decode hot path; limit observations stay byte-identical for fixtures; make check; privacy-safe whole-history RSS on uro-l0gd.
