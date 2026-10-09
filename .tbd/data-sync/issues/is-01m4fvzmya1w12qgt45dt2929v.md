---
type: is
id: is-01m4fvzmya1w12qgt45dt2929v
title: "Address PR #16 reviews A and B: Codex explicit fork boundary accounting"
kind: task
status: open
priority: 1
version: 11
spec_path: docs/project/specs/active/plan-2026-09-16-scalable-ingestion.md
labels: []
dependencies: []
parent_id: is-01m3jx84rbxf06bdgrx0q9kn74
child_order_hints:
  - is-01m4fvzncg9k85vmvgw454pkgs
  - is-01m4fvznrjmnf27r6pb8e8r51z
  - is-01m4fvzp7e3g6eb0edrtm803sf
  - is-01m4fvzpn8pv7beery5r588y0k
  - is-01m4fvzq1rqzm30s93md68qjjk
  - is-01m4fvzqfvhw9qqj85jsbf7cyw
  - is-01m4fvzqxmjgveswmev70f21xw
  - is-01m4fvzray67mxp26jmvbs0qvw
  - is-01m4fvzrqrgprt476kpzyhf3tz
  - is-01m4fvzs4atmmtb89aq7esz4eg
created_at: 2026-10-09T08:20:27.721Z
updated_at: 2026-10-09T08:20:32.008Z
---
Addressing PR #16 (layer 2 of stack #17) senior review A (https://github.com/jlevy/urollup/pull/16#pullrequestreview-5467191112) and correctness review B (https://github.com/jlevy/urollup/pull/16#pullrequestreview-5467217389), both pinned at 3c4e9cdf36b0288ad0c286f25247e85fab9fbc30. Decisions: revert the copy-only coverage rule (policy deferred to uro-xpd0); degrade fork-boundary anomalies per rollout instead of aborting; B5 deferred to uro-h2sf.
