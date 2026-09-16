---
type: is
id: is-01m2kt20pjyjyamr8aq40qe02x
title: Add --sources-file and the source manifest contract
kind: task
status: closed
priority: 2
version: 4
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - phase-1
  - milestone-0.2
dependencies:
  - type: blocks
    target: is-01m2kt2762jcwn6nbhdq0tssnk
  - type: blocks
    target: is-01m2kt30t72y6mnfy39nfa9x52
parent_id: is-01m2ke45tasy262pas37jxwss5
created_at: 2026-09-16T00:32:22.224Z
updated_at: 2026-09-16T03:01:59.079Z
closed_at: 2026-09-16T03:01:59.077Z
close_reason: Created twice when the bead-mapping run was interrupted and rerun; uro-rdfn is the original.
resolution: duplicate
duplicate_of: is-01m2ksw2b7m94av1rbjhnpx1e8
---
Milestone 0.2: read a source manifest from --sources-file or the platform config directory. Design §2.1 (Source Manifest, Projects and Accounts) and Decision 9.

Acceptance:
- urollup:SourceManifest/v1 YAML declares roots and artifacts, dialect hints, source environment, project mappings, and each known account's stable identifier with an optional display alias.
- Read from --sources-file, else sources.yaml in $XDG_CONFIG_HOME/urollup/ (default ~/.config/urollup/) on Linux, ~/Library/Application Support/urollup/ on macOS and %APPDATA%\urollup\ on Windows; price overrides use the same directory.
- Roots a manifest declares are part of default discovery, so the capture store preserves them like other default roots, while individual artifacts it lists are treated like --source inputs (Decision 9).
- Project mapping maps worktrees to one logical project while keeping the original recorded cwd; otherwise project is the git top-level basename when recorded, else the recorded cwd basename, and never decoded from an encoded project directory name.
- Precedence tests: the urollup override variable wins, then the native variable, then defaults; a missing root named by a flag or variable exits 1 and a missing default root is skipped; --no-default-sources removes defaults and variables.
