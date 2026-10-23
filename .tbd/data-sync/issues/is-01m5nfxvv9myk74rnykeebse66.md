---
type: is
id: is-01m5nfxvv9myk74rnykeebse66
title: Report symlinks skipped during discovery as diagnostics and coverage gaps
kind: bug
status: open
priority: 2
version: 1
spec_path: docs/project/specs/active/plan-2026-10-23-alpha-stabilization.md
labels:
  - coverage
  - alpha
  - milestone-0.1
dependencies: []
parent_id: is-01m5nfwqwjgaf1665db3fhvaan
created_at: 2026-10-23T23:00:54.760Z
updated_at: 2026-10-23T23:00:54.760Z
---
roots.rs:223-253 records broken, outside-root and already-visited links in the manifest's skipped_links, but adapters.rs:56 (release_discovery) clears them and snapshot_losses never reads them, so a Claude project directory symlinked outside the root, or a broken .jsonl link under Codex sessions/, contributes nothing with complete: true and no diagnostic. Design §2.2 says such links are reported as skipped. Fix: emit a skipped-link diagnostic counted by reason and a coverage gap; decide whether an outside-root link to a Claude project directory (a common way to move history to another disk) should be followed. Related: uro-zd3m vets non-regular link targets.
