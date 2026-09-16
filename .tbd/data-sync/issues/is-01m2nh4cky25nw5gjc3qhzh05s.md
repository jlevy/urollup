---
type: is
id: is-01m2nh4cky25nw5gjc3qhzh05s
title: Install milestone 0.1 CLI locally for manual testing
kind: task
status: closed
priority: 2
version: 3
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
delegate: claude-code@spud10.local
labels: []
dependencies: []
parent_id: is-01m2ke36qgfvdvnhw7c7v5esm6
hold: null
hold_until: null
created_at: 2026-09-16T16:34:51.645Z
updated_at: 2026-09-16T16:40:49.102Z
started_at: 2026-09-16T16:35:02.965Z
closed_at: 2026-09-16T16:40:49.101Z
close_reason: Installed urollup 0.1.0 from milestone-0.1 commit 32b8cc1 with cargo install --path crates/urollup --locked into /Users/levy/.cargo/bin/urollup; verified --version, --help, and an explicit-source JSON report against the brief-double-counting fixture.
resolution: null
duplicate_of: null
---
Install the current milestone-0.1 branch into the user's Cargo bin directory using the repository lockfile, then verify the installed path, version, help surface, and a read-only command suitable for manual testing.
