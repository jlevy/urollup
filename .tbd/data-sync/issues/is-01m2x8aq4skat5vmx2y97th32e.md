---
type: is
id: is-01m2x8aq4skat5vmx2y97th32e
title: Clear leftover Value helpers off the decode path
kind: task
status: open
priority: 2
version: 2
spec_path: docs/project/specs/active/plan-2026-09-16-scalable-ingestion.md
labels:
  - milestone-0.1
  - performance
dependencies: []
parent_id: is-01m2pkgv1mh7268dh4sxbptdmg
created_at: 2026-09-19T16:34:57.304Z
updated_at: 2026-09-19T20:58:51.984Z
---
Phase 2 leftover after Phase 1 typed bodies. parse_record in sources/decode.rs still builds a Value; Claude read_subagent_meta still parses the sidecar as Value; Claude line.rs number checks still go through Value::from. None of these should be on the usage-line hot path after the Phase 1 Claude/Codex beads. This bead removes or confines what remains, and keeps parse_record only as a test oracle if still needed.

Files:
- crates/urollup-core/src/sources/decode.rs: parse_record, field/text/unsigned helpers
- crates/urollup-core/src/adapters/claude_project.rs: read_subagent_meta / read_subagent_meta_with_limit
- crates/urollup-core/src/adapters/claude_project/line.rs: any remaining Value::from number visits
- Call-site audit: rg parse_record and serde_json::Value under crates/urollup-core/src/adapters

Acceptance: adapter decode hot path has no Value document; sidecar parse stays bounded; make check.

## Notes

2026-09-19: Unblocked from uro-n1cp. Depends on file-level leftovers uro-nuhn, uro-nzo1, uro-a3fo.
