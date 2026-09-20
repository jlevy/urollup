---
type: is
id: is-01m2y4yf4mejx5e6b80an6f3w4
title: Remap Claude malformed-record evidence to the global source table
kind: bug
status: open
priority: 1
version: 3
spec_path: docs/project/specs/active/plan-2026-09-16-scalable-ingestion.md
labels: []
dependencies:
  - type: blocks
    target: is-01m2pkgv1mh7268dh4sxbptdmg
parent_id: is-01m2pkgv1mh7268dh4sxbptdmg
created_at: 2026-09-20T00:55:04.595Z
updated_at: 2026-09-20T00:58:49.344Z
---
Audit R2, PR #11. reader.rs stamps per-source EvidenceRef.source=0. Claude normalize remaps record/fact evidence but not manifest.entries[].first_malformed before cloning the manifest into SourceArtifact. Two synthetic Claude files with malformed second lines resolve only 1 of 2 first_malformed references to their actual source; Codex already remaps these entries. Stamp Claude manifest first_malformed through ledger.source_table before cloning snapshots. Add a multi-source test asserting each referenced source ID equals its manifest source ID for manifest and SourceArtifact, on one and several workers.
