---
type: is
id: is-01m4h6ajb1tdgxgnyj01fqckyn
title: Exact --session selection misses other sessions' claims on a shared response
kind: bug
status: open
priority: 2
version: 1
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels: []
dependencies: []
parent_id: is-01m2ke36qgfvdvnhw7c7v5esm6
created_at: 2026-10-09T20:40:25.691Z
updated_at: 2026-10-09T20:40:25.691Z
---
Found while implementing PR #25 (uro-gop8): exact --session selection narrows discovery to the selected session family before decode, so it never sees another session's claim on the same response. With two sessions that both record one response as their own, 'sessions --session S1' and '--session S2' each count it as owned and report Possible 0, while the whole-history run reports it as ambiguous. Design §4.2 expects a partially selected ambiguous request to be reported as possible, not counted. Pinned by the ambiguous-owner fixtures and the parity ledger entry claude-ambiguous-owner-session-selection. Fix options: widen narrowing to families that share response keys, or report selection-level uncertainty when owners outside the selection cannot be ruled out.
