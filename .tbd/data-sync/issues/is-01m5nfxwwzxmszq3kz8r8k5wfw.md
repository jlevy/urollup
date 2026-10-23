---
type: is
id: is-01m5nfxwwzxmszq3kz8r8k5wfw
title: Count a Claude transcript whose file name is not its sessionId
kind: bug
status: open
priority: 2
version: 1
spec_path: docs/project/specs/active/plan-2026-10-23-alpha-stabilization.md
labels:
  - correctness
  - alpha
  - milestone-0.1
dependencies: []
parent_id: is-01m5nfwqwjgaf1665db3fhvaan
created_at: 2026-10-23T23:00:55.838Z
updated_at: 2026-10-23T23:00:55.838Z
---
claude_project.rs:369-377 and 1591, with the session taken from the file stem (1839-1854): every main-file record whose sessionId differs from the stem is treated as a resumed replay and becomes a copy. A renamed, exported or archived transcript passed with --source therefore reports 0 requests with complete: true and a misleading claude-nested-copy-without-original diagnostic (synthetic: 2 requests and 4,520 tokens under the UUID name, 0 under another name). The maintainer's default corpus has only UUID-named transcripts, so this hits --source archives. Fix: treat a foreign-sessionId record as a replay only when the named session's uuid or message owner exists; otherwise count it as original with a diagnostic, or take the session from the lines when none names the stem. Related: uro-xpd0.
