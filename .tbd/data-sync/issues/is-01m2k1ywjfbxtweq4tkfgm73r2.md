---
type: is
id: is-01m2k1ywjfbxtweq4tkfgm73r2
title: "PR #3 review R8: capture scope for manifest-declared roots undefined"
kind: bug
status: closed
priority: 2
version: 5
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels: []
dependencies: []
parent_id: is-01m2ew7x8t5sh3h5997e0rz0t2
created_at: 2026-09-15T17:31:13.869Z
updated_at: 2026-09-15T19:58:31.087Z
closed_at: 2026-09-15T19:58:31.086Z
close_reason: "Confirmed by maintainer 2026-09-15: manifest roots count as default discovery (captured by default); individual manifest artifacts follow --source (captured only with --capture). Recorded in design Decision 9, §2.5 and Source Manifest; removed from §10.2."
resolution: null
duplicate_of: null
---
PR #3 finding R8 (Low). docs/urollup-design.md:500-502 and confirmed Decision 9 cover default discovery and --source only; a source manifest declares roots and artifacts (:355-356) and falls in neither. Needs a maintainer decision because Decision 9 is confirmed. Recommendation: manifest-declared roots count as default discovery; manifest-declared artifacts follow --source.
