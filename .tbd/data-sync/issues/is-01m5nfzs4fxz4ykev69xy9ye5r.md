---
type: is
id: is-01m5nfzs4fxz4ykev69xy9ye5r
title: Make the sessions view identify sessions
kind: task
status: open
priority: 1
version: 1
spec_path: docs/project/specs/active/plan-2026-10-23-alpha-stabilization.md
labels:
  - cli
  - alpha
  - milestone-0.1
dependencies: []
parent_id: is-01m5nfwr7vgz75j01zt8p449kk
created_at: 2026-10-23T23:01:57.518Z
updated_at: 2026-10-23T23:01:57.518Z
---
The sessions table shows only hashed thr- IDs sorted by hash, with no native session ID or date, so a person cannot map a row to a Claude Code or Codex session; JSON already has session and last_date. Show the native session ID (abbreviated), agent, project, first and last date, requests, tokens and the cost estimate; sort by last activity, newest last; mark partial rows. Keep thr- IDs in JSON.
