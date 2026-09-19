# urollup

urollup (usage rollup) is a Rust CLI for trustworthy token, request-size and usage
reports from coding-agent session logs.
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

## Common Workflows

Inside a recognized agent session, report the current session:

```bash
urollup report --current
```

List every discovered session, roll usage up by local calendar day, or break the full
corpus down by project and model:

```bash
urollup sessions --all
urollup daily --all
urollup report --all --group-by project,model
```

Select one exact session and its descendants, emit JSON in a fixed timezone for a script
or agent, or read only explicitly named inputs:

```bash
urollup report --session <session-id-or-log-path> --scope descendants
urollup daily --all --timezone UTC --format json
urollup report --all --source <log-root-or-file> --no-default-sources
```

The CLI discovers the standard Claude Code and Codex log locations unless
`--no-default-sources` is set.
Whole-history reports have no input-size limit.
`UROLLUP_JOBS` sets how many threads decode logs, and `UROLLUP_STATS=1` prints phase
timings, the worker count and row counts to stderr.
Run `urollup <command> --help` for exact-session selection, descendant scope, timezone,
grouping and output options.

Version 0.1 does not yet provide date-range filters, weekly or monthly commands, price
and cost calculation, compact or responsive tables, or status-line and block views.
Those ccusage-style workflows remain planned rather than implied by the current CLI.

For the 0.1.0 terminal contract, automatic color is limited to interactive human-facing
output and can always be disabled with `--color never` or `NO_COLOR`; `--color always`
and `FORCE_COLOR` support an explicitly forced human presentation.
Interactive human-facing table commands show a transient scan indicator by default;
progress is written to stderr and `--no-progress` always disables it.
Redirected, piped and machine-readable workflows stay plain and noninteractive.

## Status

Milestone 0.1’s accounting and report implementation and fixture-backed ccusage
reconciliation are complete, as are terminal-aware color, interactive progress and the
privacy-tested local acceptance tools.
A bounded, consented real-log corpus passed the command and memory-watchdog checks.
Whole-history `sessions`, `daily` and `report --all` runs now complete on large local
corpora: sources decode on parallel workers into compact rows, so memory follows the
number of usage records rather than log bytes.
The temporary 512 MiB input guard is gone; a compact-row ceiling (default 25% of
physical RAM, or 2 GiB when RAM cannot be read) exits with a capacity error instead of
growing without bound.
`--max-ram` and `--max-rows` set the budget.
The
[scalable-ingestion plan](docs/project/specs/active/plan-2026-09-16-scalable-ingestion.md)
records dated measurements and the remaining memory work.
Whole-history results still need acceptance under the
[full-history QA playbook](tests/qa/full-history-rollup.qa.md).
Recorded maintainer decisions and independent implementation review also remain before
the milestone is accepted, after which packaging implementation and rehearsal can begin.

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
| [Milestone 0.1 local QA playbook](tests/qa/milestone-0.1-local-acceptance.qa.md) | Consent, terminal, bounded-corpus, privacy and cleanup checks for manual acceptance |
| [2026-09-16 milestone 0.1 QA report](docs/project/qa/qa-report-2026-09-16-milestone-0.1.md) | Privacy-safe evidence, aggregate results and the large-corpus limitation that scalable ingestion later removed |
| [Scalable-ingestion plan](docs/project/specs/active/plan-2026-09-16-scalable-ingestion.md) | Whole-history ingestion design, semantic changes and dated measurements |

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
