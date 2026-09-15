---
type: is
id: is-01m2hsnvcn9y0xsssg6gj3n78h
title: "PR #2 review R9: README and AGENTS.md misstate repo; skill advertises unpinned runner; flowmark unpinned"
kind: bug
status: closed
priority: 2
version: 3
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
delegate: claude-code@spud10.local
labels:
  - pr-review
dependencies: []
parent_id: is-01m2hsnnbq6dkhthgdawf0dkxr
hold: null
hold_until: null
created_at: 2026-09-15T05:47:14.707Z
updated_at: 2026-09-15T16:15:33.185Z
started_at: 2026-09-15T16:13:57.874Z
closed_at: 2026-09-15T16:15:33.184Z
close_reason: "Fixed in 5e96b6f: README and AGENTS.md say no product code with one exploration; flowmark-rs 0.4.0 pinned in the uv dev group; AGENTS.md forbids the softschema skill's @latest fallback in this repo."
resolution: null
duplicate_of: null
---
README and AGENTS.md say no code although explorations/log-throughput has Rust; AGENTS.md instructs flowmark --auto without pinning flowmark; the softschema skill falls back to softschema@latest and its allowed-tools pattern does not match uv run --frozen softschema. Review: https://github.com/jlevy/urollup/pull/2#issuecomment-5675367219
