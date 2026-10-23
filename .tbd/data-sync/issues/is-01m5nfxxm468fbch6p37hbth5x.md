---
type: is
id: is-01m5nfxxm468fbch6p37hbth5x
title: Warn when Codex or Claude Code log formats drift past what the adapters know
kind: feature
status: open
priority: 2
version: 1
spec_path: docs/project/specs/active/plan-2026-10-23-alpha-stabilization.md
labels:
  - coverage
  - alpha
  - milestone-0.1
dependencies: []
parent_id: is-01m5nfwqwjgaf1665db3fhvaan
created_at: 2026-10-23T23:00:56.578Z
updated_at: 2026-10-23T23:00:56.578Z
---
Codex review finding 4 (2026-10-23): a token_usage_record whose usage object has no recognized key becomes an owned request with no tokens (usage_observation, codex_rollout.rs:3043-3045) while requests_without_usage stays 0; string-typed counts read as absent; unknown record types are skipped (RecordType::Other, :983); every source is SourceCapability::Supported (:2791) with no version check, though Codex changed its format at 0.153 and 0.155. Fix: treat a usage object with no recognized count as no usage; tally unknown usage-bearing record types in a diagnostic; record the newest tested cli_version (Codex) and version (Claude Code) and emit one diagnostic for sources newer than that. Keep totals unchanged.
