# Project Instructions for AI Agents

This file provides instructions and context for AI coding agents working on this
project.

<!-- BEGIN TBD INTEGRATION format=f08 surface=agents-md -->
## tbd

This repository uses **tbd** for git-native issue tracking (beads), spec-driven
planning, and on-demand engineering guidelines.
As the agent, you operate tbd on the user’s behalf: translate their requests into tbd
actions rather than telling them to run commands.

- Run `tbd prime` to load current project state and the full tbd workflow.
- Run `tbd skill` for the complete reusable tbd skill instructions.
- Run `tbd shortcut --list` and `tbd guidelines --list` for on-demand resources.
- Track all work as beads: `tbd create`, `tbd ready`, `tbd close`, and `tbd sync`.

<!-- END TBD INTEGRATION -->

## Development Tools

Python tooling is development-only and pinned in `pyproject.toml`, `uv.toml` and
`uv.lock`; it is never a runtime or build dependency of the Rust crates.

- **softschema** authors and checks the urollup summary contract.
  Run it as `uv --config-file uv.toml run --frozen softschema …` so the pinned version
  is used. In this repository, never use the softschema skill’s `softschema@latest`
  fallback (`uvx` or `npx`); the skill in `.agents/skills/softschema/` is otherwise
  current.
- **flowmark** (flowmark-rs) formats Markdown:
  `uv --config-file uv.toml run --frozen flowmark --auto <file>`.
- Always pass `--config-file uv.toml` to uv, so user-level uv configuration never
  changes resolution and `uv.lock` stays identical on every machine and in CI.
- Dependencies follow a 14-day release cool-off (`exclude-newer` in `uv.toml`);
  first-party packages such as softschema are exempt.

## Project Status

urollup is in planning; there is no product code yet, only one exploration (a Rust log
throughput spike) under `explorations/log-throughput/`. It will be a Rust CLI and local
read-only web UI that produces usage rollups (tokens, cost, request sizes, tools) from
coding-agent session logs, per session or aggregated across sessions.

The design lives in `docs/urollup-design.md`, the entry point for goals, layers,
decisions and open questions.
Planning docs live under `docs/project/`:

- `specs/active/`: the plan spec, with phases, milestones, testing and rollout.
- `research/`: background research briefs.

The planning epic is `uro-lpow`. Run `tbd list --specs` to see beads grouped by linked
spec.

## Build & Test

No build exists yet.
The Rust workspace, `make check` gate and CI are defined in the design doc’s “Workspace
and Crate Structure” and “Engineering Conventions” sections, and scaffolding them is the
first Phase 1 bead.

```bash
uv --config-file uv.toml sync --locked                   # install pinned dev tooling
uv --config-file uv.toml run --frozen softschema --help  # schema contract tooling
```

## Conventions & Patterns

- Track all work as tbd beads; link implementation beads to the plan spec with `--spec`.
- Markdown docs follow `tbd guidelines common-doc-guidelines`; format them with
  `uv --config-file uv.toml run --frozen flowmark --auto <file>`.
- Specs never include time estimates, and phases stay as few as possible.
- Never copy private session content (prompts, paths, IDs, values) from local agent logs
  into docs, fixtures or tests; fixtures are synthetic or sanitized.
