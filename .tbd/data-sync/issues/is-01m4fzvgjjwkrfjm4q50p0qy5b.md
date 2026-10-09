---
type: is
id: is-01m4fzvgjjwkrfjm4q50p0qy5b
title: "PR #16 C5: parity ledger retirement condition names only one of its two causes"
kind: bug
status: closed
priority: 3
version: 2
spec_path: docs/project/specs/active/plan-2026-09-16-scalable-ingestion.md
labels: []
dependencies: []
parent_id: is-01m4fzvd803vx5er2c7k9cycgn
created_at: 2026-10-09T09:28:06.479Z
updated_at: 2026-10-09T09:47:40.805Z
closed_at: 2026-10-09T09:47:40.803Z
close_reason: "fixed in 1bec262: ledger retirement names both causes (2,700 first-step and 1,050 invalid-boundary); parity run passes. Reply: https://github.com/jlevy/urollup/pull/16#issuecomment-6078490540"
resolution: null
duplicate_of: null
---
Low. Review C https://github.com/jlevy/urollup/pull/16#pullrequestreview-5468251421. tests/parity/ledger.toml:620: the codex-unverified-fork-boundary deltas combine the first child's excluded first step (-2,700 total) and the second child's invalid-boundary exclusion (-1,050); retirement names only the first.
