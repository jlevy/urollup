---
type: is
id: is-01m2y1r6v6fbtm50s568vskdfm
title: Record Cursor as a planned supported agent in the design
kind: task
status: open
priority: 2
version: 8
spec_path: docs/project/specs/active/plan-2026-09-19-cursor-dialect.md
delegate: unknown@spud10
labels:
  - post-0.1
dependencies:
  - type: blocks
    target: is-01m2y1rag1z513ncz2mr4k7vk9
parent_id: is-01m2y1qw9mgfwsbpdgam8nn13r
hold: null
hold_until: null
created_at: 2026-09-19T23:59:13.765Z
updated_at: 2026-10-11T06:29:17.783Z
started_at: 2026-09-20T07:01:46.133Z
---
After format facts lock, record a design decision (Cursor planned support) in docs/urollup-design.md §2.1, §3.4, and §10.1, keep the one-line pointers in the product plan current, and note on uro-uyq7 that Cursor left the generic later-adapters candidate.

## Notes

2026-09-20: the working tree implemented this and make check passed; a local run against a real Cursor store reconciled with no unresolved requests (values kept local). Superseded by PR #14 (plan) and draft PR #20 (adapter), stack #21.

2026-10-09, PR #14 review A1 (tracked as uro-0jq1; https://github.com/jlevy/urollup/pull/14#pullrequestreview-5467142502): this bead is the admission decision record that A1 asks for, and it now blocks the adapter bead uro-p9ay. PR #14 describes Cursor as a candidate pending this decision. To close it, record a maintainer-confirmed decision in docs/urollup-design.md §10.1 (choice, rationale, tradeoffs, Confirmed date), as Decision 28 did for Gemini CLI, and update the §2.1, §9.1 and §10.2 pointers. The tradeoffs must cover the plan's departures from the §9.1 candidate policy: no ccusage cursor parity case (§9.1 requires one per new dialect), opt-in rather than default discovery, and the usage gap (no cache fields, mostly zero bubble tokens, session-grain cost), plus Decision 20 ordering or its exception (uro-2hck). A checked plan item without a confirmed decision does not satisfy this bead.
