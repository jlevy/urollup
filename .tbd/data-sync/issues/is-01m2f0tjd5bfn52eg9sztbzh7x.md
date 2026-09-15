---
type: is
id: is-01m2f0tjd5bfn52eg9sztbzh7x
title: Validate Pi adapters and imported multi-account fixtures
kind: task
status: open
priority: 2
version: 4
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - phase-2
dependencies:
  - type: blocks
    target: is-01m2khtbp8g70sv96scqqpnkhr
parent_id: is-01m2f0tdnzmd4d9fy3afh49bfx
created_at: 2026-09-14T03:54:26.084Z
updated_at: 2026-09-15T22:09:00.322Z
---
pi-session and pi-events adapters, plus imported multi-account and cloud-export fixtures, each validated individually.

## Notes

Findings from the 2026-09-14 source reviews (ccusage, Codex, Pi, agentfdr, session-report, squares, metaproc); rules are specified in the plan and data contracts, and sources are in the research briefs.
- Pi dedupe: provider + responseId, else lineage root + digest of copy-invariant fields; never a bare entry ID; link parents by file basename (parentSession or legacy branchedFrom); same-id files without a parent field are export/import copies; copied history belongs to the parent; nested copies (retainedTail, extension details, compaction_end, turn_end, agent_end) never count.
- Handle v1, v2 and v3 sessions; version-bound pi-events parsing (pre-0.84.0, 0.84.0-0.84.1, 0.84.2+) where only message_end is final; getSessionStats per-file sum as a reconciliation check; label pre-0.81.0 sessions as missing tool and summary usage and pre-0.70.0 output as possibly inflated.
- Usage carriers on tool results and compaction or branch summaries have unknown call counts and no model; metaproc's message_final is a partial revision; zero-usage seeded messages are not requests; Pi cost is a source estimate (zero on nonzero tokens gets a diagnostic).
- ccusage parity (2026-09-15): add ccusage pi daily and session cases (with --pi-path at the fixture copy) to the ccusage reconciliation harness (uro-jumy, uro-ne7g); seed the Pi fork replay ledger entry, fixed on ccusage main in 809eeb6 but not in 20.0.20.
