---
type: is
id: is-01m5nfzrs8a0nks2ncs35jn84s
title: Render aligned terminal tables with totals rows
kind: task
status: open
priority: 1
version: 2
spec_path: docs/project/specs/active/plan-2026-10-23-alpha-stabilization.md
labels:
  - cli
  - alpha
  - milestone-0.1
dependencies:
  - type: blocks
    target: is-01m5nfzs4fxz4ykev69xy9ye5r
parent_id: is-01m5nfwr7vgz75j01zt8p449kk
created_at: 2026-10-23T23:01:57.159Z
updated_at: 2026-10-23T23:02:37.386Z
---
render.rs (66, 89, 112) prints pipe-separated, unaligned rows with no totals row, so daily, sessions and report breakdowns are hard to read; G3 (uro-o69v) requires default tables readable at terminal width. Render aligned columns with right-aligned numbers, a totals row, and a layout that fits the terminal width (uro-00fc's ccusage width code note applies); keep piped output plain and stable and JSON unchanged. Update goldens.
