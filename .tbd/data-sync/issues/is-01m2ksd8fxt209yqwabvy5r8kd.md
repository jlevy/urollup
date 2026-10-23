---
type: is
id: is-01m2ksd8fxt209yqwabvy5r8kd
title: "Acceptance G3: CLI usability pass"
kind: task
status: open
priority: 2
version: 2
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - milestone-0.5
  - acceptance
dependencies:
  - type: blocks
    target: is-01m5nfwrjysbgvzjy9b6y1axxd
parent_id: is-01m2ke5h6e0v5vq32nfbmy6rs2
created_at: 2026-09-16T00:21:02.074Z
updated_at: 2026-10-23T23:02:38.889Z
---
Make everyday CLI use friendly: the common case needs no flags, help lists commands with examples, default tables are readable at terminal width, diagnostics name the flag or fix to use, nothing prompts, exit codes follow design §6.5, and a small report meets the performance targets. Includes a usability review of help text and error messages against the design's CLI goals.
