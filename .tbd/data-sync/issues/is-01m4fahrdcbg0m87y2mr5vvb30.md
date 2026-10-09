---
type: is
id: is-01m4fahrdcbg0m87y2mr5vvb30
title: Read .jsonl.gz sources alongside .jsonl and .jsonl.zst
kind: bug
status: closed
priority: 1
version: 4
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels: []
dependencies: []
parent_id: is-01m2ke36qgfvdvnhw7c7v5esm6
created_at: 2026-10-09T03:15:46.730Z
updated_at: 2026-10-09T09:28:14.007Z
closed_at: 2026-10-09T09:28:14.005Z
close_reason: "Merged to main in 98f9428 via PRs #18 and #22 (stack #23) after senior, correctness, security and follow-up reviews with every finding dispositioned."
resolution: null
duplicate_of: null
---
Discovery recognizes only .jsonl and .jsonl.zst, so gzip-compressed Claude Code transcripts and Codex rollouts (.jsonl.gz) are silently excluded from every report; nothing in coverage or diagnostics says so. The uro-y2qj adapter scope listed gzip input but only zstd landed. Add a Gzip representation (multi-member, pure-Rust flate2/miniz_oxide decoder), share one suffix and decoder table across discovery, the snapshot reader, first-record re-reads, parallel weighting, Claude subagent sidecar lookup and the CLI catalog/classification readers, strip .gz from locators so a source keeps its ID across compression, and define precedence and twin verification when several representations of one locator coexist (plain > zstd > gzip). Cover with reader, discovery and adapter tests plus a CLI golden; follow SUPPLY-CHAIN-SECURITY.md for the new crates.

## Notes

Implemented in PR https://github.com/jlevy/urollup/pull/18 (branch fix/compressed-sources-codex-usage, based on main; also applies cleanly on PR 16 with its fork tests passing). Local gates pass except gate proofs and MSRV tests, left to CI. Close when PR 18 merges.
