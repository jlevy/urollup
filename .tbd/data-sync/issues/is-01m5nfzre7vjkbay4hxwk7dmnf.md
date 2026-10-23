---
type: is
id: is-01m5nfzre7vjkbay4hxwk7dmnf
title: Decode Claude server tool use counts for list-price estimates
kind: task
status: open
priority: 3
version: 1
spec_path: docs/project/specs/active/plan-2026-10-23-alpha-stabilization.md
labels:
  - pricing
  - alpha
  - milestone-0.1
dependencies: []
parent_id: is-01m5nfwr7vgz75j01zt8p449kk
created_at: 2026-10-23T23:01:56.806Z
updated_at: 2026-10-23T23:01:56.806Z
---
Claude Code usage records carry server_tool_use counts (web search and web fetch requests), which the adapter never decodes although the source-review table marks them applied. Retain them per request so a price table can value them, and report them as unpriced when no rate exists. Tokens are unaffected.
