---
type: is
id: is-01m4mx4enqewq3bb62nvn1nj7m
title: Claude malformed lines are skipped without a diagnostic
kind: bug
status: open
priority: 2
version: 3
spec_path: docs/project/specs/active/plan-2026-09-16-first-release-publishing.md
labels: []
dependencies:
  - type: blocks
    target: is-01m5nfwqwjgaf1665db3fhvaan
parent_id: is-01m2ke36qgfvdvnhw7c7v5esm6
created_at: 2026-10-11T07:16:45.878Z
updated_at: 2026-10-23T23:03:03.785Z
---
Found in PR #19 review A (A2): the Codex adapter reports malformed-line and pending-tail diagnostics, but the Claude adapter skips an unparseable transcript line without any diagnostic, so a damaged Claude transcript can lose usage while coverage stays complete. Bring Claude to parity with Codex: emit malformed-line for an interior unparseable line and pending-tail for an unfinished last line (design §2.2), with tests and a golden, and decide whether a malformed line that could carry usage makes coverage partial.

## Notes

2026-10-23 alpha readiness review: confirmed on beaede2 (a truncated interior usage line drops that request's usage with complete: true and no diagnostic). The same review found that a Codex malformed line emits malformed-line but coverage still reads complete, although ManifestEntry::is_complete says otherwise (manifest.rs:371-376); make a malformed line that matches the usage prefilter a coverage gap for both agents. In scope for alpha Phase 1 (uro-q2jo, plan-2026-10-23-alpha-stabilization.md).
