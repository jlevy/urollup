---
type: is
id: is-01m2f0snm56a9zh7xm2zh4hw13
title: Review and confirm proposed plan decisions
kind: task
status: open
priority: 1
version: 5
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - planning
dependencies: []
parent_id: is-01m2ew7x8t5sh3h5997e0rz0t2
created_at: 2026-09-14T03:53:56.612Z
updated_at: 2026-09-15T22:09:06.896Z
---
Maintainer review of the 27 proposed decisions in the plan spec's Decisions to Confirm and Open Questions section (naming, identity and ownership, pricing, web security, benchmarks, summary and bundle formats, engineering baseline, release scope, session detection and CLI surface), and answers to the open questions (cloud export formats, pricing bases, receipts, --timezone default, redaction HMAC key). Update the plan and design doc (docs/urollup-design.md) for any changed decision, then mark the plan status Approved.

## Notes

2026-09-15 ccusage parity review added five candidate decisions to design §9.1 (now 14 candidates), each with a recommendation and reflected in the design:
- Statusline Command: urollup statusline from Claude Code hook input (session segment in 0.5, today in Phase 2), recorded rate_limits, no blocks or burn rate (uro-vccm, uro-iui3).
- MCP Surface: no MCP server in Phases 1-2; a later read-only stdio server over QuerySpec (uro-8ypz).
- Additional Agent Adapters: only Claude Code, Codex and Pi through Phase 2; further agents one tested dialect at a time; SQLite-backed agents wait for Decision 20 (uro-uyq7).
- Report Presentation: responsive and --compact tables, locale-independent formats, --color auto|never, --no-cost, --last, pooled cache-read share (uro-00fc).
- Configuration Defaults: no defaults file; saved --query files are presets.
See design §10.6 ccusage Use-Case Coverage.
