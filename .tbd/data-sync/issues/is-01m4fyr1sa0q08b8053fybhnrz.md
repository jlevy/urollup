---
type: is
id: is-01m4fyr1sa0q08b8053fybhnrz
title: "PR #18 D1: a twin whose first record cannot be read is neither verified nor reported"
kind: bug
status: closed
priority: 2
version: 2
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels: []
dependencies: []
parent_id: is-01m4fyr1emrns7vc1e3x4gxqh0
created_at: 2026-10-09T09:08:44.457Z
updated_at: 2026-10-09T09:09:13.127Z
closed_at: 2026-10-09T09:09:13.125Z
close_reason: "Fixed in 277c9f0 (PR #18 D1): three-way first-record re-read; UnreadableTwin failure; oversized lines passed over as by the scan; a substituted primary keeps failures other than an early end; BrieflyAbsent kept. Reply https://github.com/jlevy/urollup/pull/18#issuecomment-6077917013"
resolution: null
duplicate_of: null
---
Medium. Review D https://github.com/jlevy/urollup/pull/18#pullrequestreview-5467941265. crates/urollup-core/src/sources/reader.rs:934-949, :967, :401. Oversized or undecodable twin first records were exempted like unfinished compressor output; a corrupt primary's failure was dropped on substitution; plain beside another source's twin with an oversized first line regressed to complete.
