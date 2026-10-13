---
type: is
id: is-01m4v37ada1mje6amtv0kr3dds
title: "PR #36 A10: read tokens passed to build-time code in gate and probe (Low)"
kind: bug
status: closed
priority: 2
version: 2
spec_path: docs/project/specs/active/plan-2026-09-16-first-release-publishing.md
labels: []
dependencies: []
parent_id: is-01m4v36rse18sa71kmtyf6qzw0
created_at: 2026-10-13T16:58:37.865Z
updated_at: 2026-10-13T18:17:46.155Z
closed_at: 2026-10-13T18:17:46.154Z
close_reason: "Declined for the gate job (read-only token, make check would need a drifting second target list); probe token now reaches only gh in 2b8d6bc. Dispositions: https://github.com/jlevy/urollup/pull/36#issuecomment-6152184645"
resolution: canceled
duplicate_of: null
---
Review A finding A10 (https://github.com/jlevy/urollup/pull/36#pullrequestreview-5507095043), PR #36. References: .github/workflows/release.yml:134-137,474-478
