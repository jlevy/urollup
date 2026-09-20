---
type: is
id: is-01m2xqdnc3rt0x1zvx49m19n4a
title: Parse Claude sidecars without Value; confine parse_record to tests
kind: task
status: in_progress
priority: 2
version: 4
spec_path: docs/project/specs/active/plan-2026-09-16-scalable-ingestion.md
labels:
  - milestone-0.1
  - performance
  - memory
dependencies:
  - type: blocks
    target: is-01m2x8aq4skat5vmx2y97th32e
parent_id: is-01m2pkgv1mh7268dh4sxbptdmg
created_at: 2026-09-19T20:58:42.433Z
updated_at: 2026-09-20T04:29:54.754Z
---
Phase 2 leftover off the usage-line hot path but still on ingest. read_subagent_meta / read_subagent_meta_with_limit in crates/urollup-core/src/adapters/claude_project.rs parse the sidecar as serde_json::Value. sources/decode.rs::parse_record still builds a full document.

Type the sidecar fields the owner map needs, and keep parse_record as a test oracle only. Call-site audit: rg parse_record and serde_json::Value under crates/urollup-core/src/adapters.

## Notes

Implemented in isolated PR11 commit to be integrated by root. Typed SubagentMetadata retains only toolUseId/agentType using existing lenient Text/Skip visitors. Baseline filesystem compatibility test passed before refactor; new reader passes same cases incl missing, malformed/ignored values, duplicate last-wins, wrong types/scalars, escaping and exact byte cap. 200 core tests passed incl 2000 generated sidecar differential cases; isolated clippy all-targets and private-item rustdoc -D warnings passed. parse_record call audit found no runtime callers; now cfg(test) pub(crate), used only by decode.rs and Claude/Codex line.rs unit-test oracles. Existing quota/number Value usage remains out of this bead. No whole-history performance claim; integration and CI pending.
