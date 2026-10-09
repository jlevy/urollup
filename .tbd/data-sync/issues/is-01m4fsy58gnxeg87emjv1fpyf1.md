---
type: is
id: is-01m4fsy58gnxeg87emjv1fpyf1
title: "PR #14 A2: specify the Cursor state-store snapshot, manifest and evidence model"
kind: bug
status: open
priority: 2
version: 5
spec_path: docs/project/specs/active/plan-2026-09-19-cursor-dialect.md
delegate: null
labels: []
dependencies:
  - type: blocks
    target: is-01m2y1rag1z513ncz2mr4k7vk9
parent_id: is-01m2y1qw9mgfwsbpdgam8nn13r
hold: null
hold_until: null
created_at: 2026-10-09T07:44:41.743Z
updated_at: 2026-10-09T07:50:56.391Z
started_at: 2026-10-09T07:45:08.983Z
---
Severity: Medium. PR #14, review A: https://github.com/jlevy/urollup/pull/14#pullrequestreview-5467142502

Deferred from PR #14 (planning gap, not a wrong statement). Blocks the adapter bead uro-p9ay.

Where: plan-2026-09-19-cursor-dialect.md (Discovery Roots table, application user state row; Phase 1 snapshot item); docs/urollup-design.md §2.2 snapshot boundary (:439-461, "Evidence references are a source ID, byte offset and length inside the manifest"), §2.5 re-extraction (:490, captured records keep their source offset); crates/urollup-core/src/sources/evidence.rs:17-24 (EvidenceRef = source index, u64 offset, u32 length).

Problem: the plan says only "a WAL-safe snapshot of state.vscdb, snapshot manifests". The confirmed snapshot, evidence and capture model is byte-addressed; a cursorDiskKV row has a key, not a stable byte offset, and its pages move as Cursor writes.

Decide and record in the plan (a "State-Store Snapshot and Evidence" subsection):
- Snapshot method: a read transaction on a read-only connection, the SQLite backup API, or VACUUM INTO a temporary copy. Rule out immutable=1, which ignores the WAL.
- Copy cost of a multi-GiB store (about 4.2 GiB on the surveyed install) on every run.
- Manifest contents: for example database and WAL size and fingerprint, schema or user_version, and the read point.
- Evidence reference for a row: for example source plus key plus value digest, and how that fits or extends EvidenceRef and the capture/re-extraction layers.
- Memory budget: how decode stays within the ingest budget (--max-ram / --max-rows) with a third agent's working set, under the uro-zrr0 policy.

## Notes

Deferred from PR #14 review A (tracking parent uro-l2tq). Listed in plan-2026-09-19-cursor-dialect.md under Open Questions Before Implementation at 712016e; blocks uro-p9ay. Waits on a design pass before adapter implementation; PR #14 is docs-only.
