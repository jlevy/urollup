---
type: is
id: is-01m2ks757afpt4nq41pktqk07a
title: "Decide the UROLLUP_* override namespace: a config-directory override and the harness binary variable"
kind: task
status: open
priority: 2
version: 1
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - milestone-0.1
  - decision
  - golden
dependencies: []
parent_id: is-01m2ke36qgfvdvnhw7c7v5esm6
created_at: 2026-09-16T00:17:42.121Z
updated_at: 2026-09-16T00:17:42.121Z
---
Two questions the golden testing audit (docs/project/research/research-2026-09-15-golden-testing-audit.md) raised and did not settle; they belong with the maintainer decision review (uro-b7kc).

- Config-directory override: Windows resolves its roaming and local application data directories through the shell API, so APPDATA and LOCALAPPDATA cannot redirect a program that asks the platform. The capture store is safe because design §2.5 gives it UROLLUP_CAPTURE_DIR, which the golden harness sets, but the config directory holding sources.yaml and prices.yaml (§2.1) has no override, so on Windows no test can prove a run ignored a real one. Decide whether to add UROLLUP_CONFIG_DIR to §2.1 and the flag index; if yes, the harness sets it and a golden proves a stray sources.yaml is not read.
- Harness variable name: scripts/run-golden.mjs passes UROLLUP_BIN to tryscript sessions, inside the UROLLUP_* namespace the design reserves for override variables. Nothing collides today; decide before the first override variable ships whether to rename it (touching tests/golden sessions and the golden-path-lookup gate probe) or to record that UROLLUP_BIN is reserved for the harness.
