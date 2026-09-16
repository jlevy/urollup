---
type: is
id: is-01m2ks7q1qfsdbrvb62ed5g79a
title: Add goldens for the remaining exit-code contract
kind: task
status: open
priority: 2
version: 1
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - phase-1
  - milestone-0.5
  - golden
dependencies: []
parent_id: is-01m2ke5h6e0v5vq32nfbmy6rs2
created_at: 2026-09-16T00:18:00.374Z
updated_at: 2026-09-16T00:18:00.374Z
---
Milestone 0.5: the golden corpus covers exit 0 and exit 2 (usage errors, undetected sessions, unimplemented commands). Design §6.5 also defines 1 (runtime failure: an unreadable requested source, an I/O or write failure), 3 (--strict coverage gap and --require-priced) and 130 (interrupted), and the plan's Golden and end-to-end result checks asks for the exit-code contract as a whole.

- Exit 1: a session pointing a discovery variable or --source at a missing root, and an unreadable source inside a fixture case.
- Exit 3: --strict on a fixture case with unresolved usage, and --require-priced once prices land in milestone 0.4.
- Exit 130: decide whether an interrupted run can be asserted portably in tryscript, which runs commands through /bin/sh and cmd.exe; if not, cover it in the Rust process tests and record that here.
- Keep every command a bare urollup invocation, so the sessions stay portable (tests/golden/README.md).
