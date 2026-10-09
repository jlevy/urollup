---
type: is
id: is-01m4h6ak4v64tkh41cqv8zewat
title: Named --timezone fails on Windows and the default zone silently becomes UTC
kind: bug
status: open
priority: 2
version: 1
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels: []
dependencies: []
parent_id: is-01m2ke36qgfvdvnhw7c7v5esm6
created_at: 2026-10-09T20:40:26.522Z
updated_at: 2026-10-09T20:40:26.522Z
---
Found by PR #24 review A: jiff is built without a bundled tz database, so on Windows (CI windows-2025) named zones such as America/Los_Angeles fail with 'no time zone database configured', and the default system zone behaves as UTC without saying so. Decide whether to bundle the database (jiff tzdb-bundle-platform or tzdb-bundle-always, through the supply-chain process) or to report the limitation, and add a Windows test of named zones.
