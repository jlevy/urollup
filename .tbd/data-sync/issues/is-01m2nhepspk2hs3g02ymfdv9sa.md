---
type: is
id: is-01m2nhepspk2hs3g02ymfdv9sa
title: Add importable Python bindings over urollup-core for in-process consumers
kind: feature
status: open
priority: 2
version: 4
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - python-bindings
dependencies: []
parent_id: is-01m2kszagjf5mxhe33418nswt5
created_at: 2026-09-16T16:40:29.748Z
updated_at: 2026-09-16T18:22:58.204Z
---
Preserve and eventually implement a Python integration surface for metaproc and other in-process consumers without making the standalone urollup CLI Python-dependent. Evaluate a separate PyO3/maturin extension crate and PyPI distribution (likely urollup-core importing as urollup_core), define a small versioned report/query API over urollup-core, smoke-test installed wheels, and compare an initial subprocess JSON integration with bindings using measured performance and ergonomics. Keep the PyPI urollup binary wheel independently runnable via exact-version uvx and uv tool install.
