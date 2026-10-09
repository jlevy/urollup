---
type: is
id: is-01m4fyr247xbqmcmwry72fsfdz
title: "PR #18 D2: source-incomplete rows count every source but describe only one kind of loss"
kind: bug
status: closed
priority: 3
version: 2
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels: []
dependencies: []
parent_id: is-01m4fyr1emrns7vc1e3x4gxqh0
created_at: 2026-10-09T09:08:44.806Z
updated_at: 2026-10-09T09:09:13.430Z
closed_at: 2026-10-09T09:09:13.429Z
close_reason: "Fixed in 6305c1f (PR #18 D2): one source-incomplete diagnostic per adapter naming each loss kind with its count. Cross-agent report row residual tracked as uro-h6m9. Reply https://github.com/jlevy/urollup/pull/18#issuecomment-6077917013"
resolution: null
duplicate_of: null
---
Low. Review D https://github.com/jlevy/urollup/pull/18#pullrequestreview-5467941265. crates/urollup-core/src/adapters.rs:93-104 with ledger/diagnostics.rs:151-171.
