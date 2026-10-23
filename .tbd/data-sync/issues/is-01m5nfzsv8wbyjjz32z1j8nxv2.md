---
type: is
id: is-01m5nfzsv8wbyjjz32z1j8nxv2
title: Add --since and --until to report, daily and sessions
kind: task
status: open
priority: 2
version: 1
spec_path: docs/project/specs/active/plan-2026-10-23-alpha-stabilization.md
labels:
  - cli
  - alpha
  - milestone-0.1
dependencies: []
parent_id: is-01m5nfwr7vgz75j01zt8p449kk
created_at: 2026-10-23T23:01:58.247Z
updated_at: 2026-10-23T23:01:58.247Z
---
Design §6.1 and the flag index define --since and --until as a half-open interval resolved to absolute instants in the report timezone, with inclusive local dates as the ccusage-compatible form (§10.6). No bead implements them for raw-log reports yet. Clip request usage to the interval (sessions report usage inside it), record the resolved interval in JSON query metadata, and test DST boundaries. Scope confirmed by the alpha selection decision bead; --whole-sessions stays with uro-feyo.
