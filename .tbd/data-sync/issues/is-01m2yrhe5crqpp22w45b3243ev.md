---
type: is
id: is-01m2yrhe5crqpp22w45b3243ev
title: Run multiple usage views from one snapshot and reusable artifacts
kind: feature
status: open
priority: 1
version: 2
spec_path: docs/project/specs/active/plan-2026-09-20-usage-analysis-workflow.md
labels: []
dependencies:
  - type: blocks
    target: is-01m2yrherhddnx2j22g3xtqedr
parent_id: is-01m2yrezrf530kbz15erhh7hw6
created_at: 2026-09-20T06:37:29.130Z
updated_at: 2026-09-20T06:37:51.996Z
---
R2: expose shared-corpus library query execution and a documented CLI workflow over existing UsageSummary/BundleManifest artifacts. Decode/reconcile raw input once for an analysis run, independent of number of views/sessions. Later queries read only the named saved artifact when default sources are disabled; test with original roots unavailable. Persist snapshot/query identity, pricing context, dimensions and per-metric availability via existing schema owners; do not treat marginal JSON as a mergeable dataset. For calendar boundaries cutting 15-minute summary buckets, use request-level bundle evidence or explicitly refuse/label approximation. Keep benchmark wall-clock metadata separate from deterministic content. No new scratch database or Phase 3 cache dependency.
