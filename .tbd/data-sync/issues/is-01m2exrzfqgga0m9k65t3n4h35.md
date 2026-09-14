---
type: is
id: is-01m2exrzfqgga0m9k65t3n4h35
title: Set up softschema 0.8.1 as a pinned developer tool
kind: chore
status: closed
priority: 2
version: 2
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - dev-tooling
dependencies: []
parent_id: is-01m2ew7x8t5sh3h5997e0rz0t2
created_at: 2026-09-14T03:01:08.214Z
updated_at: 2026-09-14T03:01:09.647Z
closed_at: 2026-09-14T03:01:09.644Z
close_reason: Skill installed; pyproject.toml pins softschema==0.8.1 (package=false, exclude-newer 14 days, softschema exempt); uv.lock resolved 15 packages; uv run --frozen softschema --version reports 0.8.1; AGENTS.md documents usage.
resolution: null
duplicate_of: null
---
Install the softschema project skill (.agents/skills, .claude/skills) and add a dev-only uv project (pyproject.toml, uv.lock) pinning softschema==0.8.1 with a 14-day exclude-newer cool-off and a first-party exemption for softschema; document uv run --frozen softschema in AGENTS.md.
