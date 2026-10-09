---
type: is
id: is-01m4fw072xqes7we83h4jhnxtw
title: Vet non-regular files at the Claude sidecar open and at discovered in-root link targets
kind: bug
status: open
priority: 3
version: 1
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels: []
dependencies: []
created_at: 2026-10-09T08:20:46.299Z
updated_at: 2026-10-09T08:20:46.299Z
---
Follow-up from PR #18 review C (https://github.com/jlevy/urollup/pull/18#pullrequestreview-5467177624), C1 and its Surface reviewed note. Both predate PR #18. (1) read_subagent_meta_with_limit (crates/urollup-core/src/adapters/claude_project.rs, File::open of agent-x.meta.json) follows symbolic links, including ones leaving the declared roots, and blocks on a FIFO. (2) Discovery's link branch (crates/urollup-core/src/sources/roots.rs, walk: record_file for any in-root link target that is not a directory) records an in-root link to a FIFO named *.jsonl, whose open then blocks the worker. PR #18 added sources::reader::open_regular_file (symlink_metadata must report a regular file, and the open file must match it) for undiscovered fallback files; apply the same check to these two paths. Residual for all three: a regular file swapped for a FIFO between the check and the open still blocks; closing that needs O_NONBLOCK, which needs a libc dependency decision.
