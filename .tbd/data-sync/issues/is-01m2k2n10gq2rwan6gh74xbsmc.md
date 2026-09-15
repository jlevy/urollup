---
type: is
id: is-01m2k2n10gq2rwan6gh74xbsmc
title: "PR #3 review R15: --source directory semantics for artifacts unstated"
kind: bug
status: closed
priority: 2
version: 3
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels: []
dependencies: []
parent_id: is-01m2k1xg6q5hna1tf7v1sd9x4e
created_at: 2026-09-15T17:43:19.311Z
updated_at: 2026-09-15T17:45:33.418Z
closed_at: 2026-09-15T17:45:33.418Z
close_reason: "R15 fixed in 017fc62: --source directory walked, contents identified by content"
resolution: null
duplicate_of: null
---
PR #3 round 2 finding R15 (Low). docs/urollup-design.md:1183 example runs merge --source summaries on a directory of summaries, but 2.1 (:343-352) never says a directory is walked for artifacts. Fix: state that a directory passed with --source is walked and each file or .urollup folder is identified by content (dialect from first records, artifact from contract header).
