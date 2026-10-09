---
type: is
id: is-01m4fsw0p83d9m7346ecj8zgwh
title: "PR #18 A1: a file compressed or deleted after its scan is reported as lost data"
kind: bug
status: closed
priority: 2
version: 2
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels: []
dependencies: []
parent_id: is-01m4fsve6qyh1655009mcb1vk3
created_at: 2026-10-09T07:43:31.527Z
updated_at: 2026-10-09T08:21:26.574Z
closed_at: 2026-10-09T08:21:26.567Z
close_reason: "Fixed in 189e466 (PR #18 A1): Vanished now means no file at open; a path gone after the scan is the non-affecting RemovedAfterScan, and the open file is checked instead of the path. Tests: a_source_removed_after_a_complete_scan_is_reported_but_not_a_loss (delete and compress-then-remove), a_change_to_the_open_file_is_still_a_loss_when_its_path_is_gone."
resolution: null
duplicate_of: null
---
Medium. Review A https://github.com/jlevy/urollup/pull/18#pullrequestreview-5467179374. crates/urollup-core/src/sources/reader.rs:727-730, manifest.rs:269-277, adapters.rs:68-95. detect_changes records Vanished after a complete scan and affects_snapshot counts it as a loss. Coordinator decision 2: a post-scan vanish is a non-affecting change; a source with no file at open time stays affecting.
