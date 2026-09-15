---
type: is
id: is-01m2khtbcq887cy28n7a6y5xpm
title: MCP stdio server over QuerySpec (Candidate, later)
kind: feature
status: open
priority: 3
version: 1
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - later
  - candidate
dependencies: []
parent_id: is-01m2f0tdnzmd4d9fy3afh49bfx
created_at: 2026-09-15T22:08:22.422Z
updated_at: 2026-09-15T22:08:22.422Z
---
Later, Candidate (design §9.1 "MCP Surface"; not scheduled, and only if confirmed): a urollup mcp stdio server exposing read-only query tools that compile to QuerySpec, as a deletable feature beside serve (Decision 21) with no evidence reads by default. Until then agents use CLI JSON output and the reporting skill. ccusage 20.0.20 has no MCP server; its @ccusage/mcp package was removed in v19.0.0 (commit d7e6993). Design §6.7 and §10.2; research brief "ccusage Feature Inventory".
