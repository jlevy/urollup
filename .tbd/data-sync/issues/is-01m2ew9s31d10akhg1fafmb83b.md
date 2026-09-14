---
type: is
id: is-01m2ew9s31d10akhg1fafmb83b
title: "Plan: define portable bundle container and schema contract"
kind: task
status: closed
priority: 1
version: 4
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
delegate: claude-code@spud10.local
labels:
  - plan-spec
dependencies:
  - type: blocks
    target: is-01m2ewe0xffh11dgc6acjznk3r
parent_id: is-01m2ew7x8t5sh3h5997e0rz0t2
hold: null
hold_until: null
created_at: 2026-09-14T02:35:21.554Z
updated_at: 2026-09-14T03:00:31.481Z
started_at: 2026-09-14T02:38:24.228Z
closed_at: 2026-09-14T03:00:31.479Z
close_reason: Bundle is a deterministic *.urollup.zip (manifest.yaml, summary.yaml, schemas/, JSONL tables) in the same contract family; major version in contract ID plus additive revision, compatibility errors, redaction profiles preserving dedup IDs, archive safety checks.
resolution: null
duplicate_of: null
---
The Portable bundles section lists bundle contents but not the container (e.g. directory, tar, JSONL, SQLite), compression, schema definition/versioning, or how compatibility is checked. Mergeable bundles are the core feature and Phase 1 depends on them. Done when the plan states a concrete recommended container and schema contract (manifest, tables/record types, version rules, forward/backward compatibility, redaction of identifiers) and the Phase 1 bullet references it.
