# urollup

urollup (usage rollup) is a planned Rust CLI and local read-only web UI for coding-agent
usage. It reads Claude Code, Codex and Pi session logs and reports tokens, cost, request
sizes and tool activity for the current session, a selection of sessions, or every
session on disk, without double counting copied or overlapping history.
Its portable output is a softschema usage summary: one session and an aggregate of many
sessions use the same format, and summaries merge into larger summaries without double
counting.

**Status:** planning.
There is no code yet; the design is under review.

## Planning Docs

| Doc | Purpose |
| --- | --- |
| [Plan spec](docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md) | Entry point: goals, workflows, design summary, phases, testing, rollout, decisions to confirm |
| [Data contracts](docs/project/architecture/arch-2026-09-13-urollup-data-contracts.md) | Ledger, analytical identities, accounting measures, summary and bundle formats |
| [Portable agent usage research](docs/project/research/research-2026-09-13-portable-agent-usage.md) | Existing tools, log dialects, common workflows, accounting pitfalls, merge rationale |
| [Rust CLI engineering baseline](docs/project/research/research-2026-09-13-rust-cli-engineering-baseline.md) | Project setup practices drawn from tbd guidelines, fdu and flowmark-rs |

## Development

Work is tracked as [tbd](https://github.com/jlevy/tbd) beads (prefix `uro`).
Development-only Python tooling is pinned with [uv](https://docs.astral.sh/uv/):

```bash
uv sync --locked                   # install pinned dev tools
uv run --frozen softschema --help  # summary contract tooling
tbd ready                          # beads ready to work on
```

See [AGENTS.md](AGENTS.md) for agent and contributor conventions.

<!-- This document follows common-doc-guidelines.md.
See github.com/jlevy/practical-prose and review guidelines before editing.
-->
