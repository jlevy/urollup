---
type: is
id: is-01m4h90w59qqxdgpfgeymxz6a9
title: "PR #25 A1: ambiguous-owner fixture notes describe intended --session behavior as current"
kind: bug
status: in_progress
priority: 2
version: 2
delegate: claude-code@spud10.local
labels: []
dependencies: []
parent_id: is-01m4h90j2wpkd56gh2rswddyj1
hold: null
hold_until: null
created_at: 2026-10-09T21:27:33.800Z
updated_at: 2026-10-09T21:27:57.280Z
started_at: 2026-10-09T21:27:57.277Z
---
Medium. crates/urollup-core/tests/fixtures/claude-project/ambiguous-owner/expected.json:194 and codex-rollout/ambiguous-owner/expected.json:210 say a single-session selection reports the shared response as possible; today it counts it as owned with Possible 0 (uro-s71z). No golden runs --session on these cases. Review: https://github.com/jlevy/urollup/pull/25#pullrequestreview-5475319334 (PR #25).
