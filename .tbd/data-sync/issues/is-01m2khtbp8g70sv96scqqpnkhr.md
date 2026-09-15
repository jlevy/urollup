---
type: is
id: is-01m2khtbp8g70sv96scqqpnkhr
title: Adapters for agents beyond Claude Code, Codex and Pi (Candidate, later)
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
created_at: 2026-09-15T22:08:22.727Z
updated_at: 2026-09-15T22:08:22.727Z
---
Later, Candidate (design §9.1 "Additional Agent Adapters"; not scheduled, and only if confirmed): adapters for agents beyond Claude Code, Codex and Pi, the one gap in design §10.6.

- ccusage 20.0.20 reads 13 more agents: OpenCode, Amp, Droid, Codebuff, Hermes Agent, Goose, OpenClaw, Kilo, Kimi, Qwen, GitHub Copilot CLI, Gemini CLI and Grok Build CLI; unreleased main adds ZCode and Antigravity (research brief "ccusage Feature Inventory").
- Add one dialect at a time, only once fixtures from that agent's source or a consented corpus exist, each with capture and export strip policies, reconciliation rules and a parity case against the matching ccusage <agent> command path.
- SQLite-backed agents (OpenCode, Hermes Agent, Goose, Kilo) wait for database input (Decision 20, Phase 3 at the earliest).
- ccusage's MIT adapters are format-fact sources with attribution (Decision 3).
- Pick agents by maintainer use; record the choice per agent in the design before implementation.
