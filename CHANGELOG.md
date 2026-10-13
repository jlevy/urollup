# Changelog

This file describes each urollup release as users receive it.
Versions follow [Semantic Versioning](https://semver.org/); before 1.0, a minor release
may change the CLI or JSON contracts.
The release workflow publishes the section for a version as its GitHub release notes and
refuses a tag while that section still says Unreleased
([publishing runbook](docs/publishing.md)).

## 0.1.0 - Unreleased

Draft for the first public alpha.
The
[publishing plan](docs/project/specs/active/plan-2026-09-16-first-release-publishing.md)
finalizes this section before `v0.1.0` is tagged.

urollup 0.1.0 reports token usage from Claude Code and Codex session logs, reconciling
copied, resumed and overlapping history so that each request counts once.

### Commands

- `report`: totals, coverage, request sizes, separate breakdowns by project, account,
  model and effort, and diagnostics, for the current session (`--current`), named
  sessions (`--session`, with `--scope descendants`) or every discovered session
  (`--all`).
- `daily`: a calendar rollup by day in an IANA timezone.
- `sessions`: one row per session.
- Terminal tables or JSON (`--format json`); color and an interactive progress indicator
  only on terminals, and plain machine-readable streams otherwise.
- Explicit inputs with `--source` and `--no-default-sources`, and an ingest budget with
  `--max-ram` and `--max-rows`.

### Privacy

urollup reads local agent log files and prints its reports; 0.1.0 contains no network
client or server code.
Reports name projects and sessions from the logs, so review them before sharing.

### Platforms

| Platform | GitHub archive | PyPI wheel |
| --- | --- | --- |
| Linux x86_64 | Static musl binary | manylinux_2_17 |
| Linux arm64 | Static musl binary | manylinux_2_17 |
| macOS Apple silicon | macOS 11.0 or newer | macOS 11.0 or newer |
| macOS Intel | macOS 11.0 or newer | macOS 11.0 or newer |
| Windows x86_64 | Static C runtime | win_amd64 |

### Contract Versions

These versions are independent of the package version.

| Contract | Version in 0.1.0 |
| --- | --- |
| Report JSON (`schema_version`) | 1 |
| Analytical identity (`req-v1-`, `thr-v1-` IDs) | 1 |
| Query | Carried in each report’s `query` object under report schema 1 |
| Bundle | Not shipped in 0.1.0 |

### Known Limitations

- Codex and Claude Code totals stay provisional until the real-history comparison is
  rerun on the release candidate; the readiness record lists the open checks.
- No date-range filters, weekly or monthly commands, price or cost calculation, compact
  tables or status-line views.
- Multiple `report --group-by` dimensions produce separate breakdowns, not combined
  groupings.
- Cache-hit request counts, tool counts, usage durations and list-price estimates are
  not reported.
- Each command rereads the logs; there is no cache.
- The ingest budget bounds retained observation rows, not whole-process memory.

<!-- This document follows common-doc-guidelines.md.
See github.com/jlevy/practical-prose and review guidelines before editing.
-->
