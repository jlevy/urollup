---
type: is
id: is-01m4fvzrqrgprt476kpzyhf3tz
title: "PR #16 B3: tests do not assert selection-level combined/self obligations; reset dimension inert in direct mode"
kind: bug
status: closed
priority: 3
version: 2
spec_path: docs/project/specs/active/plan-2026-09-16-scalable-ingestion.md
labels: []
dependencies: []
parent_id: is-01m4fvzmya1w12qgt45dt2929v
created_at: 2026-10-09T08:20:31.607Z
updated_at: 2026-10-09T09:05:35.288Z
closed_at: 2026-10-09T09:05:35.288Z
close_reason: "fixed in 5d8cdeb: selection_totals and SessionIndex self/descendant assertions on counter and direct paths, reset after advance, direct-path matrix variation. Reply: https://github.com/jlevy/urollup/pull/16#issuecomment-6077861115"
resolution: null
duplicate_of: null
---
Low. Review B https://github.com/jlevy/urollup/pull/16#pullrequestreview-5467217389. paginated_forks.rs:222-317 asserts only ledger totals; no selection_totals or spawn relationship / descendant scope assertions; the reset variation at :239 is ignored on the direct path.
