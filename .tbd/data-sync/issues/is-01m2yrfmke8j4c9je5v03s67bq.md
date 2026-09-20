---
type: is
id: is-01m2yrfmke8j4c9je5v03s67bq
title: Reject ignored grouping flags and correct report capability help
kind: bug
status: open
priority: 2
version: 3
spec_path: docs/project/specs/active/plan-2026-09-20-usage-analysis-workflow.md
labels: []
dependencies:
  - type: blocks
    target: is-01m2yrherhddnx2j22g3xtqedr
  - type: blocks
    target: is-01m2ksd8fxt209yqwabvy5r8kd
parent_id: is-01m2ke36qgfvdvnhw7c7v5esm6
created_at: 2026-09-20T06:36:30.189Z
updated_at: 2026-09-20T06:37:55.607Z
---
R1: SelectionArgs exposes group_by for daily and sessions, but cli::execute consumes it only for report. Reproduced on tests/golden/samples/fixtures/claude-project/block-records: daily with and without --group-by model exits 0 with identical daily-only JSON. Reject unsupported combinations with exit 2 until implemented; document report groups as separate marginals and remove the tools claim from report help until tools are emitted. Add hermetic CLI regressions. Joint grouping is separate planned work.
