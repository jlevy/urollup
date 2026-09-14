---
type: is
id: is-01m2ew9z9077dsvqre9d37n4sb
title: "Plan: harden the loopback web server against DNS rebinding and cross-origin access"
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
created_at: 2026-09-14T02:35:27.902Z
updated_at: 2026-09-14T02:51:47.252Z
started_at: 2026-09-14T02:38:24.464Z
closed_at: 2026-09-14T02:51:47.251Z
close_reason: "Web UI hardened: 127.0.0.1 with OS-assigned port, per-launch 256-bit token in URL fragment sent as Bearer, exact Host plus Origin/Sec-Fetch-Site checks, no CORS, redirect file for --open, bounded evidence reads; security tests added."
resolution: null
duplicate_of: null
---
The Web UI section relies on loopback-only binding, which does not prevent DNS rebinding or cross-origin requests from malicious pages, and the server exposes private usage data. Done when the plan requires Host and Origin validation, a no-CORS default, and any token/port-randomization decision, with corresponding tests in the Testing Strategy.
