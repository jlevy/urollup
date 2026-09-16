---
type: is
id: is-01m2kszhw7aa0nhn3561fqps86
title: Add a single-file archive layer over the bundle folder (Later)
kind: feature
status: open
priority: 3
version: 1
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - later
dependencies: []
parent_id: is-01m2kszagjf5mxhe33418nswt5
created_at: 2026-09-16T00:31:01.508Z
updated_at: 2026-09-16T00:31:01.508Z
---
Later, not scheduled: a single-file archive layer over the *.urollup/ bundle folder, for transport. Design §5.4, §10.2 and Decision 16.

Acceptance (when taken up):
- The folder stays the canonical form, browsable and readable with zstdcat and jq; the archive is a transport wrapper only.
- Readers apply the same manifest-listed-files, path-safety, digest and decompression-bound rules as the folder reader.
