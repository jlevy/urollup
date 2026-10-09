---
type: is
id: is-01m4fsy4wabkzmkjge5zszn57x
title: "PR #14 A1: design states Cursor admission without a confirmed decision"
kind: bug
status: closed
priority: 2
version: 3
spec_path: docs/project/specs/active/plan-2026-09-19-cursor-dialect.md
delegate: claude-code@spud10.local
labels: []
dependencies: []
parent_id: is-01m4fsxxjgrahm03vd1fr1vhms
hold: null
hold_until: null
created_at: 2026-10-09T07:44:41.353Z
updated_at: 2026-10-09T07:50:31.352Z
started_at: 2026-10-09T07:45:08.663Z
closed_at: 2026-10-09T07:50:31.351Z
close_reason: "Fixed in 712016e (review option 2): design §2.1, §9.1 and §10.2 and the product-plan pointer describe Cursor as a candidate pending a recorded decision; 'the next such agent' is gone; the plan overview, status and Phase 1 item defer admission to the decision record uro-jfaw, which now blocks uro-p9ay. The decision itself is not written here. Confirmed by pinned flowmark --auto --check . (exit 0), git diff --check (clean), and the relative link and anchor check of the four changed files (907 links, bad=0)."
resolution: null
duplicate_of: null
---
Severity: Medium. PR #14, review A: https://github.com/jlevy/urollup/pull/14#pullrequestreview-5467142502

Where: docs/urollup-design.md:2642-2643 (§9.1 Additional Agent Adapters), :3440 (§10.2 Future Enhancements), and the §2.1 pointer at :405-407; docs/project/specs/active/plan-2026-09-19-cursor-dialect.md:26-28, :37-38, :399-402.

Problem: the design says "Cursor is the next such agent" and schedules it "Phase 3 or later" without "if confirmed", inside a section marked Status: Candidate, but no confirmed decision admits Cursor. Gemini CLI was admitted through Decision 28 at planning time. The plan defers recording the decision to its own Phase 1.

Fix chosen (option 2 of the review): reword the design and plan additions as candidate pointers pending a recorded decision; drop "the next such agent". The decision record itself (choice, rationale, tradeoffs: no ccusage cursor parity case, opt-in discovery, the usage gap, Decision 20 ordering) stays with uro-jfaw, which now blocks the adapter bead uro-p9ay. This bead does not write the decision.
