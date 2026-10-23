---
type: is
id: is-01m5nfyw7nwmf1p3jp20m2d6dv
title: Degrade discovery-walk errors to coverage gaps instead of aborting
kind: bug
status: open
priority: 1
version: 1
spec_path: docs/project/specs/active/plan-2026-10-23-alpha-stabilization.md
labels:
  - robustness
  - alpha
  - milestone-0.1
dependencies: []
parent_id: is-01m5nfwqwjgaf1665db3fhvaan
created_at: 2026-10-23T23:01:27.924Z
updated_at: 2026-10-23T23:01:27.924Z
---
roots.rs:135, 148 and 162 push every symlink_metadata or canonicalize error, NotFound included, into unreadable, and claude_project.rs:607-612 and codex_rollout.rs:719-724 turn the first one into a fatal AdapterError, so both agents' commands exit 1. A synthetic race (a project directory with many entries while one file is created and deleted) failed 10 of 30 runs; a chmod 000 project directory (or one left root-owned by sudo) fails every run. The reader already treats sources that vanish after discovery as benign. Fix: treat NotFound during the walk as absent; stat only entries that could be sources; turn an unreadable directory or file into a per-source coverage gap and diagnostic.
