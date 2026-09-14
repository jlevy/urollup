---
type: is
id: is-01m2ewa7zw4n7ckptq387kyet3
title: "Plan: specify platform matrix, release channels and cloud binary acquisition"
kind: task
status: closed
priority: 2
version: 4
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
delegate: claude-code@spud10.local
labels:
  - plan-spec
dependencies:
  - type: blocks
    target: is-01m2ewe0xffh11dgc6acjznk3r
parent_id: is-01m2ew7x8t5sh3h5997e0rz0t2
hold: null
hold_until: null
created_at: 2026-09-14T02:35:36.824Z
updated_at: 2026-09-14T02:59:50.443Z
started_at: 2026-09-14T02:38:24.680Z
closed_at: 2026-09-14T02:59:50.442Z
close_reason: Rollout Plan lists five native-built targets, GitHub Releases/crates.io/PyPI wheels via trusted publishing, SHA256SUMS plus attestations, SemVer; cloud skill binary acquisition order with verified digests and unsupported-environment result.
resolution: null
duplicate_of: null
---
The rollout plan does not list target platforms or install channels, and the cloud skill needs a Linux binary but the plan never says how a sandbox obtains a pinned binary, possibly without network egress. Done when the plan lists target triples, release/distribution channels and verification (checksums, provenance), and the cloud skill's binary acquisition path with fallbacks.
