---
type: is
id: is-01m4kj8rvykmrj81xmf58pfkpe
title: Read the nested thread_spawn parent link of old-format Codex subagents
kind: task
status: open
priority: 3
version: 1
spec_path: docs/project/specs/active/plan-2026-09-16-first-release-publishing.md
labels: []
dependencies: []
parent_id: is-01m2ke36qgfvdvnhw7c7v5esm6
created_at: 2026-10-10T18:47:38.620Z
updated_at: 2026-10-10T18:47:38.620Z
---
Found in PR #32 review C (C1). Codex subagents written before roughly 0.150 name their parent only in session_meta.source.subagent.thread_spawn.parent_thread_id, without the top-level parent_thread_id urollup reads (line.rs). PR #32 treats such threads as having lineage so they are never roots or parent evidence; this bead makes urollup read the nested link as the parent relationship (relationships, orphan detection, roots, copy owners), so their children can be decided instead of reported as gaps. A local real-history check found no such parent in the maintainer's archive.
