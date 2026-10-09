---
type: is
id: is-01m4fsw5frfxqb0d35khexv4r6
title: "PR #18 C1: the vanished-source fallback opens files discovery refused or never saw"
kind: bug
status: closed
priority: 2
version: 2
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels: []
dependencies: []
parent_id: is-01m4fsve6qyh1655009mcb1vk3
created_at: 2026-10-09T07:43:36.439Z
updated_at: 2026-10-09T08:21:36.111Z
closed_at: 2026-10-09T08:21:36.109Z
close_reason: "Fixed in 6e8827f (PR #18 C1): an undiscovered sibling is opened only when symlink_metadata reports a regular file and the open file matches it. Test: an_undiscovered_file_that_is_not_a_regular_file_is_never_read (outside link, link to another source, FIFO). Pre-existing sidecar and discovery link paths tracked as uro-zd3m."
resolution: null
duplicate_of: null
---
Medium. Review C https://github.com/jlevy/urollup/pull/18#pullrequestreview-5467177624. reader.rs:346-360 (open_source), 366-370 (sibling). Follows links outside roots, reads duplicates, blocks on FIFOs. Coordinator decision 3: an undiscovered sibling is opened only when symlink_metadata says regular file; never block on a FIFO.
