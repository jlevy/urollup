---
type: is
id: is-01m4fsw3p021jzbytkbk142r8s
title: "PR #18 A9: attach the owning thread to UnreadableSource gaps when known"
kind: bug
status: open
priority: 3
version: 3
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels: []
dependencies: []
parent_id: is-01m4fsve6qyh1655009mcb1vk3
created_at: 2026-10-09T07:43:34.590Z
updated_at: 2026-10-09T08:21:09.413Z
---
Suggestion (Low). Review A https://github.com/jlevy/urollup/pull/18#pullrequestreview-5467179374. adapters.rs:87-91; accounting/totals.rs:211-216. Gap thread None applies to every selection in the run.

## Notes

Deferred from PR #18 (review A, A9). Not small and clean: scoping an UnreadableSource gap to its owning thread is safe only when that thread's family membership is known. A damaged rollout or transcript usually has no readable header, so its parent link is unknown; a gap scoped to its own thread would leave a parent --session selection reporting complete while its unreadable child lost data. Today the gap has thread None and marks every selection partial, which is conservative. Waits on a design decision for how gaps from sources with unknown family membership apply to selections (accounting/totals.rs selection_totals). The gaps were introduced by uro-1h5s.
