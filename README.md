# urollup

urollup (usage rollup) is a Rust CLI for trustworthy token, cost, request-size and tool
activity reports from coding-agent session logs.
Its accounting model reconciles copied, resumed and overlapping history before producing
per-session or aggregate results.

The milestone 0.1 CLI reads Claude Code and Codex logs and provides `report`, `daily`
and `sessions` commands with terminal-table and JSON output.
Pi and Gemini CLI adapters, portable mergeable summaries, capture and a local read-only
web UI are planned in later milestones.

## Install for Development

From a checkout, install the current binary into Cargo’s executable directory:

```bash
cargo install --locked --path crates/urollup
```

Public installation through GitHub Releases, crates.io, PyPI and `uvx` begins with
version 0.1.0. The
[publishing plan](docs/project/specs/active/plan-2026-09-16-first-release-publishing.md)
defines the target matrix and release gates; those public packages are not published
yet.

## Use

```bash
urollup report --current       # report the calling agent session
urollup sessions               # list every discovered session
urollup daily --format json    # aggregate discovered usage by day as JSON
```

The CLI discovers the standard Claude Code and Codex log locations.
Use `--source` to add another root or artifact, and `--no-default-sources` to read only
explicitly named sources.
Run `urollup <command> --help` for selection, scope, grouping and output options.

For the 0.1.0 terminal contract, automatic color is limited to interactive human-facing
output and can always be disabled with `--color never` or `NO_COLOR`; `--color always`
and `FORCE_COLOR` support an explicitly forced human presentation.
Noticeably long operations show progress by default only during interactive human use;
progress is written to stderr and `--no-progress` always disables it.
Redirected, piped and machine-readable workflows stay plain and noninteractive.

## Status

Milestone 0.1’s accounting and report implementation and fixture-backed ccusage
reconciliation are complete, as are terminal-aware color, interactive progress and the
privacy-tested local acceptance tools.
The consented real-log run, recorded maintainer decisions and independent implementation
review remain before the milestone is accepted.
Packaging implementation and rehearsal follow that acceptance gate.

## Documentation

| Doc | Purpose |
| --- | --- |
| [Design specification](docs/urollup-design.md) | Goals, layers, interfaces, decisions, glossary and CLI flag index |
| [Implementation plan](docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md) | Product phases, milestones, tests, performance targets and rollout |
| [0.1.0 publishing plan](docs/project/specs/active/plan-2026-09-16-first-release-publishing.md) | Release channels, target matrix, artifact validation and publication runbook |
| [Portable agent usage research](docs/project/research/research-2026-09-13-portable-agent-usage.md) | Existing tools, log dialects, workflows, accounting risks and merge rationale |
| [Rust CLI engineering baseline](docs/project/research/research-2026-09-13-rust-cli-engineering-baseline.md) | Repository, gate, release and development-tooling practices |
| [Agent tool source reviews](docs/project/research/research-2026-09-14-agent-tool-source-reviews.md) | Detailed Codex, ccusage, Pi, agentfdr and Anthropic plugin evidence |
| [squares code review](docs/project/research/research-2026-09-14-squares-code-review.md) | Reusable parsing, time measures and tests from squares |
| [metaproc and qm review](docs/project/research/research-2026-09-14-metaproc-code-review.md) | Captured streams, harness pitfalls, accounts and quotas |
| [Golden testing audit](docs/project/research/research-2026-09-15-golden-testing-audit.md) | Golden-harness risks, fixes and the end-to-end fixture strategy |

The portable research brief also records a throughput spike on real local logs; its
prototype is kept in
[explorations/log-throughput](explorations/log-throughput/README.md).

## Develop

The workspace has two crates: `crates/urollup-core`, the accounting library, and
`crates/urollup`, the executable.
Rust 1.98.0 is pinned in `rust-toolchain.toml`, and the minimum supported Rust version
is 1.85. Work is tracked as [tbd](https://github.com/jlevy/tbd) beads (prefix `uro`).
Development-only tools are pinned with [uv](https://docs.astral.sh/uv/) (softschema,
flowmark) and npm (tryscript, taplo).

```bash
make build   # debug build
make test    # Rust tests and CLI goldens
make check   # the handoff gate CI also runs
make fix     # format Rust, TOML and Markdown
tbd ready    # beads ready to work on
```

The maintainer-only acceptance targets never run in CI and refuse to read default agent
logs without explicit consent.
They write allowlisted aggregates under ignored `target/` directories:

```bash
make e2e-local CONSENT_LOCAL_LOGS=1
make parity-local CONSENT_LOCAL_LOGS=1
```

See [AGENTS.md](AGENTS.md) for toolchain setup and contributor conventions,
[SUPPLY-CHAIN-SECURITY.md](SUPPLY-CHAIN-SECURITY.md) before changing dependencies,
[SECURITY.md](SECURITY.md) to report a vulnerability, and [PROVENANCE.md](PROVENANCE.md)
for code ported from other repositories.
urollup is licensed under the [MIT License](LICENSE).

<!-- This document follows common-doc-guidelines.md.
See github.com/jlevy/practical-prose and review guidelines before editing.
-->
