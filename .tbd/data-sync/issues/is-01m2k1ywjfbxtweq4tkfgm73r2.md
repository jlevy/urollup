---
type: is
id: is-01m2k1ywjfbxtweq4tkfgm73r2
title: "PR #3 review R8: capture scope for manifest-declared roots undefined"
kind: bug
status: in_progress
priority: 2
version: 2
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels: []
dependencies: []
parent_id: is-01m2k1xg6q5hna1tf7v1sd9x4e
created_at: 2026-09-15T17:31:13.869Z
updated_at: 2026-09-15T17:34:39.342Z
---
PR #3 finding R8 (Low). docs/urollup-design.md:500-502 and confirmed Decision 9 cover default discovery and --source only; a source manifest declares roots and artifacts (:355-356) and falls in neither. Needs a maintainer decision because Decision 9 is confirmed. Recommendation: manifest-declared roots count as default discovery; manifest-declared artifacts follow --source.
