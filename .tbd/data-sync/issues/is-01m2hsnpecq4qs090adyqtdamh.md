---
type: is
id: is-01m2hsnpecq4qs090adyqtdamh
title: "PR #2 review R2: Default redaction and per-machine HMAC keys break cross-machine grouping; default project naming unspecified"
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
created_at: 2026-09-15T05:47:09.643Z
updated_at: 2026-09-15T16:28:29.448Z
started_at: 2026-09-15T16:13:57.576Z
closed_at: 2026-09-15T16:28:29.447Z
close_reason: "Fixed in 2b6f811: default paths redaction removes paths and needs no key; project exported as a plain name (manifest mapping, git top-level basename, cwd basename); keyed labels only for names and native-ids with UROLLUP_REDACTION_KEY or --redaction-key-file; cross-machine example and merge test updated; HMAC key open question resolved."
resolution: null
duplicate_of: null
---
plan merge example (local + cloud bundles, --group-by project) exits 2 because paths redaction labels cwd with per-machine keys (arch redaction, compatibility error). Define default project as a plain name resolved at export, decide HMAC key location for Phase 1, and state which group-by dimensions survive default redaction across machines. Review: https://github.com/jlevy/urollup/pull/2#issuecomment-5675367219
