---
type: is
id: is-01m2nyw6d2sjzx29tj860njhvz
title: Document and manually validate v0.1 common use cases
kind: task
status: closed
priority: 1
version: 4
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - milestone-0.1
  - qa
  - docs
dependencies: []
parent_id: is-01m2ke36qgfvdvnhw7c7v5esm6
created_at: 2026-09-16T20:35:03.201Z
updated_at: 2026-09-19T07:40:01.852Z
closed_at: 2026-09-19T07:40:01.851Z
close_reason: "README workflows, milestone 0.1 QA playbook and dated QA report shipped on PR #8 (efa5ef8). Remaining real-log acceptance is uro-d36a and uro-ky6c."
resolution: null
duplicate_of: null
---
On stacked PR #8, reconcile the top-level README with the proven ccusage use-case inventory and current tbd Rust CLI/documentation guidance; document only commands urollup 0.1 actually supports and name deferred gaps. Add tests/qa/general-qa.runbook.md with exact manual commands, expected invariants, negative cases, troubleshooting and success criteria. Execute it against this repository's consented real local logs, capture only privacy-safe aggregate evidence in a dated report, fix any failures, reinstall the global developer binary, synchronize the stack with current main, and leave both PRs mergeable with green CI.

## Notes

2026-09-16: README common workflows (with the 0.1 input safety limit), the reusable QA playbook tests/qa/milestone-0.1-local-acceptance.qa.md (named per the milestone rather than general-qa.runbook.md) and the dated QA report are committed on PR #8 (efa5ef8). make check passed; bounded real-log matrix, terminal checks, default-corpus guard and global install smoke test passed. Stack kept with GitHub merge history (no force-push). Remaining: PR #8 CI green, mark PRs #4 and #8 ready for review. Full-corpus --all and make parity-local stay blocked on uro-o6x5.
