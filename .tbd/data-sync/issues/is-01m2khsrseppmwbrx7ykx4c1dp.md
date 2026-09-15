---
type: is
id: is-01m2khsrseppmwbrx7ykx4c1dp
title: Add report presentation options (Candidate)
kind: task
status: open
priority: 3
version: 1
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - phase-1
  - milestone-0.5
  - candidate
dependencies: []
parent_id: is-01m2ke5h6e0v5vq32nfbmy6rs2
created_at: 2026-09-15T22:08:03.371Z
updated_at: 2026-09-15T22:08:03.371Z
---
Milestone 0.5, Candidate (design §9.1 "Report Presentation"; implement only once confirmed), per design §6.4 and §6.6:

- Terminal tables fit the terminal width; below 100 columns on a terminal, or with --compact, keep only period or group, token totals and cost; piped output never switches layout. Reuse the ccusage terminal width and table code already noted on uro-d135 (MIT notice).
- Locale-independent formatting (ISO 8601 dates, no --locale); JSON, JSONL and CSV values never display-formatted.
- --color auto|never and NO_COLOR turn color off; color still appears only on a terminal (§8.2).
- --no-cost omits amounts from tables and JSON while pricing coverage still appears.
- --last <n> selects the n most recent calendar periods in the report timezone and week start, resolved to absolute bounds in the normalized QuerySpec.
- Reports show the pooled cache-read share (cache reads over inclusive input) computed from token sums, never averaged across rows.
- CLI goldens for each option; flag index rows already in design §10.4.
