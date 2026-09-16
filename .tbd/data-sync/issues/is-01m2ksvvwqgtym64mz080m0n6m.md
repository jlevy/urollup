---
type: is
id: is-01m2ksvvwqgtym64mz080m0n6m
title: Implement the bundle folder writer and reader with atomic publication
kind: task
status: open
priority: 1
version: 3
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - phase-1
  - milestone-0.2
dependencies:
  - type: blocks
    target: is-01m2ksw5c3g28x3wk3g9hhfybr
parent_id: is-01m2ke45tasy262pas37jxwss5
created_at: 2026-09-16T00:29:00.693Z
updated_at: 2026-09-16T00:29:10.400Z
---
Milestone 0.2: read and write *.urollup/ observation bundles. Design §5.4, §2.3 and §8.4.

Acceptance:
- Writes manifest.yaml (urollup:BundleManifest/v1), summary.yaml, schemas/*.schema.yaml for every contract used so the unpacked artifacts validate with `softschema validate` and no flags, and tables/*.jsonl.zst for records, sources, threads, relationships, requests, limits and diagnostics, with tools, provider_charges and resources only when requested and available.
- Deterministic content identity over uncompressed tables: one UTF-8 JSON object per line sorted by analytical ID, a fixed zstd level, and manifest row counts, sizes and SHA-256 over the uncompressed bytes, so two bundles with the same content compare equal even across zstd library versions. A bundle records no creation time.
- Atomic publication: every file written into an owner-only staging folder beside the destination, fsynced, verified against the manifest, and the folder renamed into place once; never assembled in place and never replacing an existing bundle.
- Readers open only manifest-listed files relative to the bundle folder and reject absolute or .. paths, symbolic links, unlisted or missing files, backslashes, drive letters, NUL bytes, case-fold duplicates and file-directory conflicts, and sizes or digests that disagree with the manifest; decompressed bytes are bounded before reading, so a partly copied bundle fails validation rather than reading as a smaller one.
- Omitted tables and dimensions are marked unavailable, not empty; --no-records omits the records table.
- Captured records re-extract from a bundle exactly as from original logs, keeping their source ID, offset, fingerprint and dialect version (§2.3).

## Notes

Bundle reader hardening rules are adapted from metaproc's safe_archive review (docs/project/research/research-2026-09-14-metaproc-code-review.md); the source review brief (research-2026-09-14-agent-tool-source-reviews.md) holds the evidence.
- Tables need no completion record, unlike streamed JSONL exports, because the manifest records each table's row count and digest.
- Table values follow the same portable value rules as the YAML artifacts: integers within ±2^53 and money as exact decimal strings.
