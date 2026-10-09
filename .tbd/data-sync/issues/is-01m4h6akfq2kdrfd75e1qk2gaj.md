---
type: is
id: is-01m4h6akfq2kdrfd75e1qk2gaj
title: SUPPLY-CHAIN-SECURITY.md says jiff has no time zone database features, but they are enabled
kind: task
status: open
priority: 3
version: 1
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels: []
dependencies: []
parent_id: is-01m2ke36qgfvdvnhw7c7v5esm6
created_at: 2026-10-09T20:40:26.870Z
updated_at: 2026-10-09T20:40:26.870Z
---
Found by PR #24 review A: SUPPLY-CHAIN-SECURITY.md (milestone 0.1 core table) says jiff is built with std only and no time zone database features, while crates/urollup-core/Cargo.toml enables tz-fat, tz-system, tzdb-concatenated and tzdb-zoneinfo. Correct the record (and revisit with the Windows tz bead).
