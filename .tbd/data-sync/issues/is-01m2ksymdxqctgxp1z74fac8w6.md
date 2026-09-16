---
type: is
id: is-01m2ksymdxqctgxp1z74fac8w6
title: Import multi-account and cloud-export fixtures
kind: task
status: open
priority: 3
version: 1
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - phase-2
dependencies: []
parent_id: is-01m2ksy5yrxn48thxc311sqsv6
created_at: 2026-09-16T00:30:31.356Z
updated_at: 2026-09-16T00:30:31.356Z
---
Phase 2: validate imported artifacts individually, since no cloud export format is claimed without a test. Design §2.1 (Projects and Accounts), §9.3 (Cloud Export Formats) and §10.2.

Acceptance:
- Multi-account fixtures prove accounts are attributed explicitly or left unknown, never guessed from model or subscription, and that grouping by account works across machines under the default paths profile.
- Cloud-export fixtures enter as ordinary manifested artifacts through the source manifest; an untested format is reported as unsupported rather than read.
- Open question §9.3 (cloud export formats) is answered from the fixtures or recorded as still open for the maintainer, without editing the design here.
