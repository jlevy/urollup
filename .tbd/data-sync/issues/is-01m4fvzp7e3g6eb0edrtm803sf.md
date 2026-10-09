---
type: is
id: is-01m4fvzp7e3g6eb0edrtm803sf
title: "PR #16 A3: a Codex report can be partial with no visible reason"
kind: bug
status: open
priority: 2
version: 1
spec_path: docs/project/specs/active/plan-2026-09-16-scalable-ingestion.md
labels: []
dependencies: []
parent_id: is-01m4fvzmya1w12qgt45dt2929v
created_at: 2026-10-09T08:20:29.037Z
updated_at: 2026-10-09T08:20:29.037Z
---
Medium. Review A https://github.com/jlevy/urollup/pull/16#pullrequestreview-5467191112. codex_rollout.rs:1431 drops CopyWithoutOriginal; query.rs:215-224 CoverageSummary has no reasons. P1 at head: complete=false, diagnostics=[].
