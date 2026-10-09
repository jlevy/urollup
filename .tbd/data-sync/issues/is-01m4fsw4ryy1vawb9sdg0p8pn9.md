---
type: is
id: is-01m4fsw4ryy1vawb9sdg0p8pn9
title: "PR #18 B2: source-incomplete diagnostics for sources without a src- ID collapse and undercount"
kind: bug
status: open
priority: 2
version: 1
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels: []
dependencies: []
parent_id: is-01m4fsve6qyh1655009mcb1vk3
created_at: 2026-10-09T07:43:35.709Z
updated_at: 2026-10-09T07:43:35.709Z
---
Medium. Review B https://github.com/jlevy/urollup/pull/18#pullrequestreview-5467213761. adapters.rs:77-92, ledger/diagnostics.rs:155-156. compact deduplicates identical diagnostics. Coordinator decision 4: aggregate per adapter with with_occurrences(n); report count equals losing sources.
