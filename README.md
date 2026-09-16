# urollup

urollup (usage rollup) is a planned Rust CLI and local read-only web UI for coding-agent
usage. It reads Claude Code, Codex and Pi session logs and reports tokens, cost, request
sizes and tool activity for the current session, a selection of sessions, or every
session on disk, without double counting copied or overlapping history.
Its portable output is a softschema usage summary: one session and an aggregate of many
sessions use the same format, and summaries merge into larger summaries without double
counting.

**Status:** milestone 0.1 in progress.
The uncached engine discovers Claude Code and Codex logs and produces deterministic
`report`, `daily` and `sessions` output as terminal tables or JSON. Milestone 0.1’s
ccusage reconciliation harness remains in progress.

## Planning Docs

| Doc | Purpose |
| --- | --- |
| [Design specification](docs/urollup-design.md) | Entry point: goals, sources and capture, ledger and identities, accounting, summary and bundle formats, CLI, web UI, open and confirmed decisions, glossary and flag index |
| [Plan spec](docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md) | Phases and milestones, testing strategy and performance targets, rollout |
| [Portable agent usage research](docs/project/research/research-2026-09-13-portable-agent-usage.md) | Existing tools, log dialects, common workflows, accounting pitfalls, merge rationale |
| [Rust CLI engineering baseline](docs/project/research/research-2026-09-13-rust-cli-engineering-baseline.md) | Project setup practices drawn from tbd guidelines, fdu and flowmark-rs |
| [squares code review](docs/project/research/research-2026-09-14-squares-code-review.md) | Reusable Codex and Claude parsing, time measures and tests from squares, and its integration needs |
| [metaproc code review](docs/project/research/research-2026-09-14-metaproc-code-review.md) | Captured agent streams, harness pitfalls, accounts and quotas from metaproc and its qm harness |
| [Agent tool source reviews](docs/project/research/research-2026-09-14-agent-tool-source-reviews.md) | Detailed evidence from the Codex, ccusage, Pi, agentfdr and Anthropic plugin source reviews: dialect facts, reuse tables, pitfalls and recommendation status |

The portable research brief also records a throughput spike on real local logs; its
prototype is kept in
[explorations/log-throughput](explorations/log-throughput/README.md).

## Development

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

See [AGENTS.md](AGENTS.md) for toolchain setup and contributor conventions,
[SUPPLY-CHAIN-SECURITY.md](SUPPLY-CHAIN-SECURITY.md) before changing dependencies,
[SECURITY.md](SECURITY.md) to report a vulnerability, and [PROVENANCE.md](PROVENANCE.md)
for code ported from other repositories.

<!-- This document follows common-doc-guidelines.md.
See github.com/jlevy/practical-prose and review guidelines before editing.
-->
