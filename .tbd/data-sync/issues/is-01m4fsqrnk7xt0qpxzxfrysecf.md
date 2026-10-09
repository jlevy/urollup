---
type: is
id: is-01m4fsqrnk7xt0qpxzxfrysecf
title: "PR #15 A7: usage guide token-field reference is incomplete"
kind: bug
status: closed
priority: 2
version: 3
spec_path: docs/project/specs/active/plan-2026-09-20-usage-analysis-workflow.md
delegate: claude-code@spud10.local
labels: []
dependencies: []
parent_id: is-01m4fsq5hrr2dsczqgy5bxcnv1
hold: null
hold_until: null
created_at: 2026-10-09T07:41:12.242Z
updated_at: 2026-10-09T07:57:02.493Z
started_at: 2026-10-09T07:48:31.998Z
closed_at: 2026-10-09T07:57:02.493Z
close_reason: "Fixed on PR #15 branch (head e17d47811bc12173ab11c5aa8b00ad35df0b4150) in e17d478 and 924857b: provider_only and total rows, JSON-only lifetimes, qualified Codex cached-split subtraction. Disposition reply: https://github.com/jlevy/urollup/pull/15#issuecomment-6076871120"
resolution: null
duplicate_of: null
---
docs/usage-analysis.md:58,64-80; README.md:67. Missing provider_only and total rows (query.rs:166-171); cache-write lifetimes are JSON-only (render.rs:138-145); Codex cached-input subtraction applies only when the cached split is reported (tokens.rs:356-361, codex_rollout.rs:1636-1644). Severity: Low. PR #15, review A (https://github.com/jlevy/urollup/pull/15#pullrequestreview-5467133135), pinned head 3776b333aa08e24b3bc0a9f4d4ce29e49b44fc58.
