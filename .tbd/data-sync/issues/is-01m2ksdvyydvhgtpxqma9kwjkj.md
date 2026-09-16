---
type: is
id: is-01m2ksdvyydvhgtpxqma9kwjkj
title: Implement self-contained HTML report output
kind: task
status: open
priority: 2
version: 2
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - phase-2
dependencies:
  - type: blocks
    target: is-01m2ksdx4757n11wv9vnhmpftm
parent_id: is-01m2f0tdnzmd4d9fy3afh49bfx
created_at: 2026-09-16T00:21:22.011Z
updated_at: 2026-09-16T00:21:23.203Z
---
Candidate decision 'Static HTML Reports' (design §9.1): --format html with --output writes one self-contained page (inline CSS and JS, no network, no server) rendering the same QuerySpec results as other formats, honoring the redaction profile and evidence rules, byte-deterministic for a snapshot and query. Shares rendering with the serve web UI but stays behind the same optional feature boundary so a default CLI build gains no HTTP or async dependencies. Needs maintainer confirmation of the candidate before implementation.
