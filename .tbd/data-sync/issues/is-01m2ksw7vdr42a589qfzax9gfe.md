---
type: is
id: is-01m2ksw7vdr42a589qfzax9gfe
title: Add merge, validate and schema commands
kind: task
status: open
priority: 1
version: 2
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - phase-1
  - milestone-0.2
dependencies:
  - type: blocks
    target: is-01m2f0tj0cf0v7wbz9vbpywjqs
parent_id: is-01m2ke45tasy262pas37jxwss5
created_at: 2026-09-16T00:29:12.940Z
updated_at: 2026-09-16T00:32:01.863Z
---
Milestone 0.2: the remaining artifact commands, completing the milestone. Design §5.1, §5.3, §5.4, §5.7 and §6.3.

Acceptance:
- merge reads only its --source inputs and never adds precomputed totals. Summary merge drops covered extents and adds only disjoint ones; bundle merge unions observations and reruns reconciliation; `--format bundle` requires every input to carry observations, otherwise merge writes a summary; incompatible inputs exit 2.
- One selection yields identical report data from raw logs, merged summaries and merged bundles, including bundles exported on two machines under the default paths profile and grouped by account, project and model.
- Merging a summary into an aggregate that already contains it changes nothing.
- validate runs schema validation, the compiled JSON Schema, the Rust cross-field checks and the bundle reader's path, size and digest checks, and rejects portable-value violations, unsafe paths and links, and files whose digests disagree with the manifest.
- schema prints the compiled contract schemas for a named contract.
- The worked example in §5.1 (export, per-session export, merge across two bundles, daily over the merged bundle, validate) runs end to end as a golden.
