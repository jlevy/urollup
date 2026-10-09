---
type: is
id: is-01m4fsqscr9rpzfsa49z34rzy7
title: "PR #15 A9: scalable plan Progress lines contradict the retired-gate disclaimer"
kind: bug
status: closed
priority: 2
version: 3
spec_path: docs/project/specs/active/plan-2026-09-20-usage-analysis-workflow.md
delegate: claude-code@spud10.local
labels: []
dependencies: []
parent_id: is-01m4fsq5hrr2dsczqgy5bxcnv1
hold: null
hold_until: null
created_at: 2026-10-09T07:41:12.983Z
updated_at: 2026-10-09T07:57:03.191Z
started_at: 2026-10-09T07:48:32.788Z
closed_at: 2026-10-09T07:57:03.190Z
close_reason: "Fixed on PR #15 branch (head e17d47811bc12173ab11c5aa8b00ad35df0b4150) in f9652fc: retired gates marked historical, uro-a3fo done, G1 dependency updated. Disposition reply: https://github.com/jlevy/urollup/pull/15#issuecomment-6076871120"
resolution: null
duplicate_of: null
---
docs/project/specs/active/plan-2026-09-16-scalable-ingestion.md:684-685,715,721,732-733 still say Phase 2 owns 512 MiB and 10 s, G1 waits on the 512 MiB gate, and uro-a3fo is open (closed; :479 marks it done); disclaimer at :520-522. Severity: Low. PR #15, review A (https://github.com/jlevy/urollup/pull/15#pullrequestreview-5467133135), pinned head 3776b333aa08e24b3bc0a9f4d4ce29e49b44fc58.
