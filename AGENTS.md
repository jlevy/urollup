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

Node tooling is also development-only: `package.json` and `package-lock.json` pin
tryscript (CLI goldens) and `@taplo/cli` (TOML formatting), and `.npmrc` disables
lifecycle scripts. Install it with `npm ci --ignore-scripts`; make targets do this on
demand.

## Project Status

urollup is in Phase 1, milestone 0.1. The Cargo workspace implements the accounting
core, Claude Code and Codex adapters, exact session selection, and uncached `report`,
`daily` and `sessions` commands in terminal-table and JSON formats.
Terminal-aware color and interactive stderr progress are implemented, the fixture-backed
ccusage reconciliation harness runs in CI, and privacy-tested local aggregate tools are
available behind explicit consent.
The remaining milestone 0.1 work is the consented real-log acceptance run, recorded
maintainer decisions and independent implementation review.
There is one exploration (a Rust log throughput spike) under
`explorations/log-throughput/`. urollup will be a Rust CLI and local read-only web UI
that produces usage rollups (tokens, cost, request sizes, tools) from coding-agent
session logs, per session or aggregated across sessions.

The design lives in `docs/urollup-design.md`, the entry point for goals, layers,
decisions, open questions and the CLI flag index.
Planning docs live under `docs/project/`:

- `specs/active/`: the plan spec, with phases, milestones, testing and rollout.
- `research/`: background research briefs.

The planning epic is `uro-lpow`. Run `tbd list --specs` to see beads grouped by linked
spec.

## Build & Test

```bash
make build   # debug build of the workspace
make test    # Rust and QA-tool tests, CLI goldens and result checks
make check   # handoff gate: everything CI enforces, fastest first
make fix     # format Rust (rustfmt), TOML (taplo) and Markdown (flowmark)
```

`make check` is the required handoff gate; if it passes, CI should.
It runs the toolchain and uv preflights, the supply-chain gate, the lint-policy check,
rustfmt, taplo and flowmark checks, `uv lock --check`, clippy and tests with and without
default features, rustdoc, the dependency guard, the MSRV build and tests, cargo-deny,
npm audit, and `make gate-proofs`, which proves each gate fails on its committed
violation in `tests/gate-probes/`. Each gate’s decision logic lives in a tested script
under `scripts/`; `make help` lists the targets, and `.github/workflows/ci.yml` runs the
same targets.

A fresh machine needs these before `make check`, at the versions the Makefile pins
(`make toolchain` and `make uv-version` say which is missing):

```bash
rustup toolchain install 1.98.0 --profile minimal --component clippy,rustfmt  # rust-toolchain.toml
rustup toolchain install 1.85.0 --profile minimal                             # MSRV
cargo install cargo-deny --locked --version 0.20.2                            # make audit
uv --config-file uv.toml sync --locked                                        # softschema, flowmark
npm ci --ignore-scripts                                                       # tryscript, taplo
```

- **CLI goldens and result checks:** sessions in `tests/golden/**/*.tryscript.md` run
  against `target/debug/urollup` through `$UROLLUP_BIN` in a hermetic environment that
  can reach no real log, and `make e2e-results` checks each fixture case’s reconciled
  totals against its `expected.json`. After an intentional output change run
  `make golden-update` and read the diff; `--update` writes what it saw, and
  `make golden-lint` rejects machine-specific paths and non-hermetic sessions.
  [tests/golden/README.md](tests/golden/README.md) has the session rules, the
  fixture-to-golden mapping and the results contract; never point a golden or a check at
  a real `~/.claude` or `~/.codex`.
- **Serving boundary:** the `serve` feature of `crates/urollup` is default-on and empty
  until Phase 2. HTTP, async-runtime and web-asset crates may enter only as `optional`
  dependencies behind it; `make dependency-guard` fails if one reaches `urollup-core` or
  a `--no-default-features` build.
- **Dependencies:** follow [SUPPLY-CHAIN-SECURITY.md](SUPPLY-CHAIN-SECURITY.md) before
  adding or upgrading any crate, npm package, PyPI package, action or toolchain.
- **Ported code:** record every file adapted from another repository in
  [PROVENANCE.md](PROVENANCE.md) with its source path and commit.

## Conventions & Patterns

- Track all work as tbd beads; link implementation beads to the plan spec with `--spec`.
- Markdown docs follow `tbd guidelines common-doc-guidelines`; format them with
  `uv --config-file uv.toml run --frozen flowmark --auto <file>`.
- Specs never include time estimates, and phases stay as few as possible.
- Never copy private session content (prompts, paths, IDs, values) from local agent logs
  into docs, fixtures or tests; fixtures are synthetic or sanitized.
