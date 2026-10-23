---
type: is
id: is-01m4h6ak4v64tkh41cqv8zewat
title: Named --timezone fails on Windows and the default zone silently becomes UTC
kind: bug
status: open
priority: 2
version: 3
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels: []
dependencies:
  - type: blocks
    target: is-01m5nfwqwjgaf1665db3fhvaan
parent_id: is-01m2ke36qgfvdvnhw7c7v5esm6
created_at: 2026-10-09T20:40:26.522Z
updated_at: 2026-10-23T23:03:04.976Z
---
Found by PR #24 review A: jiff is built without a bundled tz database, so on Windows (CI windows-2025) named zones such as America/Los_Angeles fail with 'no time zone database configured', and the default system zone behaves as UTC without saying so. Decide whether to bundle the database (jiff tzdb-bundle-platform or tzdb-bundle-always, through the supply-chain process) or to report the limitation, and add a Windows test of named zones.

## Notes

2026-10-23 alpha readiness review: an invalid TZ environment value also silently becomes Etc/Unknown with exit 0 (query.rs:47-49). Report an unusable default zone instead of guessing. In scope for alpha Phase 1 (uro-q2jo).
