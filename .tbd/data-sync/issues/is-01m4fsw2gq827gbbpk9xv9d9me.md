---
type: is
id: is-01m4fsw2gq827gbbpk9xv9d9me
title: "PR #18 A6: reading a fallback file leaves no trace in the manifest"
kind: bug
status: open
priority: 3
version: 1
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels: []
dependencies: []
parent_id: is-01m4fsve6qyh1655009mcb1vk3
created_at: 2026-10-09T07:43:33.398Z
updated_at: 2026-10-09T07:43:33.398Z
---
Low. Review A https://github.com/jlevy/urollup/pull/18#pullrequestreview-5467179374. reader.rs:335-362 (called at 264-268). Coordinator decision 3: opening any file other than the discovered primary records a non-affecting SourceChange naming what was read.
