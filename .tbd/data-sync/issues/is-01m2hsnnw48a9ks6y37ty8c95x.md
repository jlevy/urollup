---
type: is
id: is-01m2hsnnw48a9ks6y37ty8c95x
title: "PR #2 review R1: uv.lock not reproducible without global uv config"
kind: bug
status: closed
priority: 1
version: 3
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
delegate: claude-code@spud10.local
labels:
  - pr-review
dependencies: []
parent_id: is-01m2hsnnbq6dkhthgdawf0dkxr
hold: null
hold_until: null
created_at: 2026-09-15T05:47:09.059Z
updated_at: 2026-09-15T16:15:32.888Z
started_at: 2026-09-15T05:47:45.279Z
closed_at: 2026-09-15T16:15:32.887Z
close_reason: "Fixed in 5e96b6f: uv settings moved to committed uv.toml and all uv invocations use --config-file uv.toml (metaproc pattern); the suggested uv lock --no-config would also drop the project's own cool-off. Lock verified with and without user config."
resolution: null
duplicate_of: null
---
uv.lock records 19 exclude-newer-package exemptions from ~/.config/uv/uv.toml while pyproject.toml declares one; uv lock --check --no-config and uv sync --locked --no-config fail. Regenerate with uv lock --no-config and guard in make check or CI. Review: https://github.com/jlevy/urollup/pull/2#issuecomment-5675367219
