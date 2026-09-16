---
title: "urollup: Rust Agent Usage CLI and Rollup Web UI"
description: Implementation plan for urollup, the Rust agent usage CLI and rollup web UI, covering phases and milestones, the testing strategy with performance targets, and rollout for the design in docs/urollup-design.md.
author: Joshua Levy with LLM assistance
date: 2026-09-13
status: Draft
---
# Feature: urollup, a Rust Agent Usage CLI and Rollup Web UI

## Overview

urollup is a standalone Rust executable that reads Claude Code, Codex and Pi session
logs and produces trustworthy token, cost and usage rollups through a CLI and a local
read-only web UI, with portable usage summaries that merge without double counting.

The [urollup design specification](../../../urollup-design.md) is the source of truth
for the design: its [goals](../../../urollup-design.md#13-design-goals),
[non-goals](../../../urollup-design.md#15-non-goals), layers, confirmed and candidate
decisions, [glossary](../../../urollup-design.md#103-glossary) and
[flag index](../../../urollup-design.md#104-flag-index).
This plan covers how that design is built and accepted: the phases and milestones, the
testing strategy with its performance targets, and the rollout.
Checklist items link to the design sections they implement rather than repeating their
rules.

Background research lives in the
[portable research brief](../../research/research-2026-09-13-portable-agent-usage.md),
the
[Rust CLI engineering baseline](../../research/research-2026-09-13-rust-cli-engineering-baseline.md),
the [squares code review](../../research/research-2026-09-14-squares-code-review.md) and
the
[metaproc and qm review](../../research/research-2026-09-14-metaproc-code-review.md).
None of these documents contains private session data.

## Scope

| Phase | Delivers | Design sections implemented |
| --- | --- | --- |
| Phase 1: accounting core and useful uncached CLI (milestones 0.1–0.5) | Claude Code and Codex adapters for persistent and captured-stream dialects, the reconciled ledger, accounting, prices, summaries, bundles, the capture store and the Phase 1 CLI | [§2](../../../urollup-design.md#2-sources-and-capture-layer) without Pi dialects or cache reads; [§3](../../../urollup-design.md#3-ledger-and-identity-layer) with observed purpose only; [§4.1](../../../urollup-design.md#41-measure-contracts)–[§4.3](../../../urollup-design.md#43-time-grouping-and-percentiles) and [§4.5](../../../urollup-design.md#45-price-table), plus provider limit observations; [§5](../../../urollup-design.md#5-artifact-layer) without database input; [§6.1](../../../urollup-design.md#61-workflows-and-session-selection)–[§6.6](../../../urollup-design.md#66-report-content-and-examples) without Phase 2 commands, and the [§6.8](../../../urollup-design.md#68-status-line) session segment if confirmed; [§8.1](../../../urollup-design.md#81-workspace-and-crate-structure)–[§8.2](../../../urollup-design.md#82-engineering-conventions) and the [uncached engine](../../../urollup-design.md#uncached-engine) |
| Phase 2: web UI, workflow reports and broader evidence | Capture cache reads, `serve`, the reporting skill, `compare`, `check`, the `windows` report, Pi adapters, and configured purpose with annotation sets | [§2.5](../../../urollup-design.md#25-capture-store-and-cache) cache reads; [§7](../../../urollup-design.md#7-serving-layer-optional); [§6.7](../../../urollup-design.md#67-reporting-skill-and-cloud-workflow); [§6.3](../../../urollup-design.md#63-commands) Phase 2 commands; [§4.4](../../../urollup-design.md#44-usage-windows); [§2.1](../../../urollup-design.md#21-dialects-and-discovery) and [§6.2](../../../urollup-design.md#62-current-session-detection) for Pi; [§3.5](../../../urollup-design.md#35-purpose-and-annotations); the [§6.8](../../../urollup-design.md#68-status-line) today segment if confirmed |
| Phase 3: idempotent persistent cache | The ledger and query cache, and urollup’s own store as input | [Ledger and query cache](../../../urollup-design.md#ledger-and-query-cache-later); [§5.1](../../../urollup-design.md#51-portable-inputs-and-artifacts) database input |

Items the design marks Later without a phase, such as the
[account registry](../../../urollup-design.md#46-accounts-and-plans-later), are listed
in the design’s
[future enhancements](../../../urollup-design.md#102-future-enhancements).
Queued review decisions get phase items only once confirmed
([§9.2](../../../urollup-design.md#92-queued-review-decisions)).

## Implementation Plan

### Phase 1: Accounting core and useful uncached CLI

Phase 1 ships as five milestones, each a usable and tested pre-1.0 release.
The capture store lands only after the uncached engine is the correctness reference.

#### Milestone 0.1: Uncached Claude Code and Codex reports

- [x] Scaffold a minimal repository to the engineering baseline: workspace, toolchain
  pin, lint and format configuration, supply-chain policy, `make check` and `make fix`,
  the npm dev project for tryscript, and the CI jobs those gates need; prove each gate
  fails on a committed violation
  ([§8.1](../../../urollup-design.md#81-workspace-and-crate-structure),
  [§8.2](../../../urollup-design.md#82-engineering-conventions)).
- [ ] Freeze public sanitized fixtures, including the research brief’s
  [double-counting cases](../../research/research-2026-09-13-portable-agent-usage.md#synthetic-double-counting-example)
  ([§1.2](../../../urollup-design.md#12-why-urollup-exists),
  [§3.4](../../../urollup-design.md#34-dialect-reconciliation-rules)).
- [x] Implement analytical identities, the normalized ledger, reconciliation, ownership
  status and coverage, with collision detection and re-derivation from stored keys
  ([§3.1](../../../urollup-design.md#31-entities),
  [§3.3](../../../urollup-design.md#33-reconciliation),
  [§3.4](../../../urollup-design.md#34-dialect-reconciliation-rules),
  [§3.6](../../../urollup-design.md#36-analytical-identities),
  [§4.2](../../../urollup-design.md#42-ownership-and-totals)).
- [ ] Implement the `claude-project` and `codex-rollout` adapters with default
  discovery, override variables, snapshot manifests, source links and provider limit
  observations; document unsupported fields and the agent versions each fixture covers
  ([§2.1](../../../urollup-design.md#21-dialects-and-discovery),
  [§2.2](../../../urollup-design.md#22-snapshot-boundary),
  [§4.4](../../../urollup-design.md#44-usage-windows)).
- [ ] Implement `--current` environment detection, `--session`, `--all`, and the
  discovery index and hierarchy crawler behind `--scope`; a detected Pi session exits 2
  with an unsupported-dialect diagnostic
  ([§6.1](../../../urollup-design.md#61-workflows-and-session-selection),
  [§6.2](../../../urollup-design.md#62-current-session-detection),
  [§3.2](../../../urollup-design.md#32-relationships-and-the-discovery-index)).
- [ ] Add `report`, `daily` and `sessions` in table and JSON formats, with project,
  account, model and effort grouping, request sizes, deterministic output and CLI
  goldens; list-price estimates wait for 0.4
  ([§6.3](../../../urollup-design.md#63-commands),
  [§6.4](../../../urollup-design.md#64-queries-output-formats-and-streams),
  [§6.5](../../../urollup-design.md#65-exit-codes),
  [§4.1](../../../urollup-design.md#41-measure-contracts),
  [§4.3](../../../urollup-design.md#43-time-grouping-and-percentiles)).
- [ ] Add the [ccusage reconciliation harness](#ccusage-reconciliation-harness) for
  token totals: pinned ccusage, per-day and per-session token comparison on the
  `claude-project` and `codex-rollout` fixtures in CI, the explained-differences ledger,
  and the privacy-tested local aggregate diff script
  ([§10.6](../../../urollup-design.md#106-ccusage-use-case-coverage),
  [ccusage feature inventory](../../research/research-2026-09-13-portable-agent-usage.md#ccusage-feature-inventory)).

#### Milestone 0.2: Contracts, summaries, bundles and merge

- [ ] Add pytest to the uv project, and author the `UsageSummary`, `BundleManifest`,
  `SourceManifest` and table record contracts with fixtures,
  `scripts/check_contracts.py` and `make contracts-check`; implement typed serde
  validators and tests that they agree with softschema on every fixture
  ([§5.6](../../../urollup-design.md#56-versioning-and-compatibility),
  [§5.7](../../../urollup-design.md#57-contract-authoring-and-validation)).
- [ ] Implement `export` and summary and bundle readers and writers with the export
  allow-list, redaction profiles and `--sources-file`; mixed raw, summary and bundle
  input; and overlap-safe `merge`, `validate` and `schema`, before any totals-only
  output
  ([§5.1](../../../urollup-design.md#51-portable-inputs-and-artifacts)–[§5.5](../../../urollup-design.md#55-redaction),
  [§2.4](../../../urollup-design.md#24-capture-and-export-strip-policies),
  [source manifest](../../../urollup-design.md#source-manifest)).

#### Milestone 0.3: Capture store

- [ ] Implement the durable capture store: the capture strip policy with unknown-key
  diagnostics, `--capture-idle`, default-discovery scope including source manifest
  roots, `--capture` for `--source` logs and manifest artifacts, atomic replacement
  entries in the platform data directory, retained reads for deleted sources, retained
  versions linked by thread for rewritten sources, runs without capture when the store
  is unwritable, `--no-capture`, `capture status` and `capture prune`, with equivalence
  goldens against the uncached engine
  ([§2.4](../../../urollup-design.md#24-capture-and-export-strip-policies),
  [§2.5](../../../urollup-design.md#25-capture-store-and-cache)).
- [ ] Measure and record capture-write throughput at zstd level 3 before capture ships
  on by default ([performance targets](#performance-targets)).

#### Milestone 0.4: Prices

- [ ] Add the reviewed price table and its `PriceTable` contract, `--prices` and
  config-directory overrides, staleness diagnostics, `--require-priced` and golden
  repricing tests ([§4.5](../../../urollup-design.md#45-price-table)).
- [ ] Extend the [ccusage reconciliation harness](#ccusage-reconciliation-harness) to
  costs: `--mode calculate` runs under shared rates, per-model cost rows, pricing ledger
  entries, and an informational comparison with the bundled table
  ([§4.5](../../../urollup-design.md#45-price-table),
  [ccusage feature inventory](../../research/research-2026-09-13-portable-agent-usage.md#ccusage-feature-inventory)).

#### Milestone 0.5: Full Phase 1 surface, benchmarks and parity

- [ ] Port metaproc’s log-processing code into the Rust adapters with provenance: format
  detection from record types, `claude-stream` and `codex-exec` captured-stream parsing,
  gzip and harness log-rewrite handling, the cross-agent tool taxonomy and its
  bug-derived test cases, behind a library-shaped adapter API
  ([§2.6](../../../urollup-design.md#26-harness-captures),
  [§8.1](../../../urollup-design.md#81-workspace-and-crate-structure)).
- [ ] Complete selection with `--hook-input`, `--sessions-from`, `--latest`, `--agent`,
  `--project`, `--cwd` and `--whole-sessions`
  ([§6.1](../../../urollup-design.md#61-workflows-and-session-selection),
  [§6.2](../../../urollup-design.md#62-current-session-detection)).
- [ ] Add `sources`, `weekly`, `monthly`, `requests`, `tools` and `tree`, observed
  purpose and tool grouping, JSONL, CSV and Markdown output, query files and `--strict`
  ([§6.3](../../../urollup-design.md#63-commands),
  [§6.4](../../../urollup-design.md#64-queries-output-formats-and-streams),
  [§6.6](../../../urollup-design.md#66-report-content-and-examples),
  [§3.5](../../../urollup-design.md#35-purpose-and-annotations)).
- [ ] Add the remaining CI jobs and `AGENTS.md` routes; flowmark-rs is already pinned in
  the uv project ([§8.2](../../../urollup-design.md#82-engineering-conventions)).
- [ ] Build the benchmark generator and harness in `bench/` with its CI jobs, record the
  first reference-laptop results with capture writes recorded separately, and measure
  summary size with the request index on a consented representative corpus
  ([performance targets](#performance-targets),
  [Decision 17](../../../urollup-design.md#decision-17-request-index-on-by-default)).
- [ ] Establish the feature matrix against pinned ccusage and agentfdr, explaining
  disagreements from source records rather than treating either as an oracle: extend the
  [ccusage reconciliation harness](#ccusage-reconciliation-harness) to every shared use
  case and record measured status in the design’s coverage table
  ([§1.5](../../../urollup-design.md#15-non-goals),
  [§10.6](../../../urollup-design.md#106-ccusage-use-case-coverage),
  [existing implementations](../../research/research-2026-09-13-portable-agent-usage.md#existing-implementations),
  [ccusage feature inventory](../../research/research-2026-09-13-portable-agent-usage.md#ccusage-feature-inventory)).
- [ ] If the candidates are confirmed, add the `statusline` session segment and the
  report presentation options: responsive and `--compact` tables, `--color`,
  `--no-cost`, `--last` and the pooled cache-read share
  ([§6.8](../../../urollup-design.md#68-status-line),
  [statusline command](../../../urollup-design.md#statusline-command),
  [report presentation](../../../urollup-design.md#report-presentation)).

### Phase 2: Web UI, workflow reports and broader evidence

- [ ] Add the capture cache read path: prefix checks, append segments, `--no-cache`,
  `--rebuild-cache` and `--verify-cache`, with cached-versus-uncached golden equivalence
  ([§2.5](../../../urollup-design.md#25-capture-store-and-cache)).
- [ ] Add `serve`: the read-only HTTP API and embedded UI over the same snapshots and
  query engine, with its security controls and coverage badges
  ([§7](../../../urollup-design.md#7-serving-layer-optional)).
- [ ] Add the CLI-backed reporting skill, `compare` and `check`
  ([§6.7](../../../urollup-design.md#67-reporting-skill-and-cloud-workflow),
  [§6.3](../../../urollup-design.md#63-commands)).
- [ ] Add the `windows` report over recorded provider limit observations, with Claude
  window lengths configured and labeled, and local totals labeled partial
  ([§4.4](../../../urollup-design.md#44-usage-windows)).
- [ ] Run the cloud smoke test covering log visibility, reachable hosts, the binary
  acquisition path, execution, artifact retrieval, local merge and overlapping
  re-export; document unsupported environments
  ([§6.7](../../../urollup-design.md#67-reporting-skill-and-cloud-workflow)).
- [ ] Validate the `pi-session` and `pi-events` adapters with Pi `--current` detection,
  and imported multi-account and cloud-export fixtures, individually; add `ccusage pi`
  cases to the [ccusage reconciliation harness](#ccusage-reconciliation-harness)
  ([§2.1](../../../urollup-design.md#21-dialects-and-discovery),
  [§6.2](../../../urollup-design.md#62-current-session-detection)).
- [ ] If the [statusline command](../../../urollup-design.md#statusline-command) is
  confirmed, add its today segment once capture cache reads meet its latency gate
  ([§6.8](../../../urollup-design.md#68-status-line)).
- [ ] Add configured purpose rules and `--annotation-set` imports with provenance;
  semantic review stays downstream of accounting
  ([§3.5](../../../urollup-design.md#35-purpose-and-annotations)).

### Phase 3: Idempotent persistent cache

- [ ] Choose storage after benchmarks expose access patterns; transactional embedded
  storage is the design direction, not a dependency decision made here
  ([ledger and query cache](../../../urollup-design.md#ledger-and-query-cache-later)).
- [ ] Handle append, replacement, deletion and late usage updates, with independently
  versioned pricing
  ([ledger and query cache](../../../urollup-design.md#ledger-and-query-cache-later)).
- [ ] Accept a read-only snapshot of urollup’s own store as input, tested for
  equivalence with raw, summary and bundle input
  ([§5.1](../../../urollup-design.md#51-portable-inputs-and-artifacts)).
- [ ] Prove cached and uncached equivalence, crash recovery, invalidation and concurrent
  reads
  ([ledger and query cache](../../../urollup-design.md#ledger-and-query-cache-later)).

## Testing Strategy

Each area tests the rules in these design sections:

| Area | Design sections |
| --- | --- |
| Accounting | [§3.3](../../../urollup-design.md#33-reconciliation), [§3.4](../../../urollup-design.md#34-dialect-reconciliation-rules), [§4.1](../../../urollup-design.md#41-measure-contracts) |
| Ground truth | [§4.1](../../../urollup-design.md#41-measure-contracts), [§4.2](../../../urollup-design.md#42-ownership-and-totals) |
| Identities | [§3.6](../../../urollup-design.md#36-analytical-identities) |
| Merge | [§5.3](../../../urollup-design.md#53-exact-aggregation), [§5.4](../../../urollup-design.md#54-observation-bundles) |
| Contracts | [§5.6](../../../urollup-design.md#56-versioning-and-compatibility), [§5.7](../../../urollup-design.md#57-contract-authoring-and-validation) |
| Capture and privacy | [§2.4](../../../urollup-design.md#24-capture-and-export-strip-policies), [§2.5](../../../urollup-design.md#25-capture-store-and-cache), [§5.5](../../../urollup-design.md#55-redaction) |
| Surfaces | [§6.2](../../../urollup-design.md#62-current-session-detection), [§6.4](../../../urollup-design.md#64-queries-output-formats-and-streams), [§6.5](../../../urollup-design.md#65-exit-codes), [§7.2](../../../urollup-design.md#72-web-ui) |
| Web security | [§7.3](../../../urollup-design.md#73-security-controls) |
| Parity | [§1.5](../../../urollup-design.md#15-non-goals), [§10.6](../../../urollup-design.md#106-ccusage-use-case-coverage) |

- **Accounting:** golden fixtures and conservation and property tests cover streaming
  updates, synthetic messages, repeated imports, request IDs spanning files, forked
  history, modern Codex ownership, ordinal gaps, nested tool calls, counter resets,
  timezone and DST boundaries, model and effort switches, unknown prices, malformed and
  oversized lines, and partial tails.
  Dialect cases cover `codex exec` totals across a resume and beside their rollout;
  repeated, `info: null`, compaction-estimate and context-window-full Codex
  `token_count` events; copied `token_usage_record` lines and legacy and paginated
  subagent prefixes; Claude block records disagreeing on `output_tokens`, advisor
  iterations, and `progress`, `/btw` and `uuid` replays; Pi fork, clone, export and
  nested usage copies; `claude-stream` limit records without timestamps and cumulative
  `total_cost_usd`; nested null fields, non-millisecond timestamps, and a path briefly
  absent during a rewrite.
  Conflicting sources yield deterministic diagnostics, never whichever record a worker
  finished first.
- **Ground truth:** before 1.0, one consented corpus is checked against a provider usage
  export (Anthropic Console or OpenAI usage), at least for totals, with differences
  explained from source records.
- **Identities:** a session copied under different roots and hosts, merged in any order,
  keeps identical IDs; stored keys re-derive IDs under another identity version, and a
  redacted component makes that exit 2; an injected digest collision raises an
  identity-collision error; a gateway reusing message IDs across sessions yields
  ambiguous keys, not merges; an archived or compressed Codex rollout keeps its `src-`
  ID, and a rewrite that changes a source’s first record gets a new `src-` ID whose
  retained predecessor still counts dropped usage once.
- **Merge:** `proptest` checks traversal-order invariance and merge associativity,
  commutativity and idempotence, and one selection yields identical report data from raw
  logs, merged summaries and merged bundles, including bundles exported on two machines
  under the default `paths` profile and grouped by account, project and model.
  Unresolved-overlap cases (diverged exports, overlapping windows, an older usage
  revision in the larger extent, differing ownership evidence, missing indexes, mixed
  identity versions) never add usage, make `--strict` exit 3, and resolve when bundles
  merge. Partial candidate selections appear only as `possible`.
- **Contracts:** golden summaries and manifests pass `softschema validate` and
  `softschema repair --check`; the serde validators, the compiled JSON Schema validator
  and softschema agree on every fixture; and readers reject portable-value violations,
  unsafe paths, links and files whose digests disagree with the manifest.
- **Capture and privacy:** a leak fixture whose record carries text under an unknown key
  yields no plaintext in the summary, bundle or records table, while the capture store
  keeps the value and reports the unknown key.
  Validation diagnostics never echo values from redacted or stripped fields.
  Default `paths` exports contain no absolute path, working directory or path-shaped
  locator, and `names` or `native-ids` without a key exits 2. Capture goldens cover the
  idle threshold, captured manifest roots, uncaptured `--source` logs and manifest
  artifacts, an unwritable store, retained and rewritten sources, and identical results
  with the store disabled.
- **Surfaces:** CLI and HTTP return identical report data for one query and snapshot,
  and Markdown, CSV and the UI derive from it.
  CLI goldens cover exit codes, JSONL completion records, and `--current` with nested
  agents, concurrent sessions in one directory, worktrees, forks, archived rollouts,
  orphaned subagents, Codex hook input from a subagent and on `SubagentStop`, and
  unsaved Codex and Pi sessions.
  Browser tests exercise filters, exports and badges, and hostile log text and quoted
  arguments are never executed.
- **Web security:** raw HTTP requests with an attacker hostname, wrong port, missing
  `Host`, foreign or `null` `Origin`, cross-site `Sec-Fetch-Site` or `OPTIONS` get 403
  and no data; missing, malformed or wrong tokens get 401; no response has
  `Access-Control-Allow-*`. Tests check required headers, per-launch tokens, and that
  the token appears only in the stdout URL line and redirect file.
  Evidence tests cover unknown IDs, out-of-extent or overflowing offsets, path-like and
  percent-encoded IDs, changed files and the byte cap, and a browser test confirms a
  second local origin cannot read responses and evidence HTML renders inert.
- **Parity:** the [ccusage reconciliation harness](#ccusage-reconciliation-harness) runs
  pinned ccusage and urollup on the same inputs and fails on any difference that the
  explained-differences ledger does not cite.

### ccusage reconciliation harness

urollup must roll up usage at least as effectively as ccusage, so a side-by-side harness
runs a pinned ccusage and urollup on the same inputs and compares their results.
ccusage is a comparator, not an oracle: every known disagreement is a ledger entry that
cites its source, and any other difference fails.
The research brief’s
[ccusage feature inventory](../../research/research-2026-09-13-portable-agent-usage.md#ccusage-feature-inventory)
gives the release, command paths and output fields compared, and the design’s
[ccusage use-case coverage](../../../urollup-design.md#106-ccusage-use-case-coverage)
lists the use cases the harness measures.

- **Layout:** the harness lives in `tests/parity/`. Its Python programs are unit-tested
  standard-library scripts run through `uv --config-file uv.toml run --frozen`, like the
  benchmark harness, and read TOML with `tomllib`. `make parity` runs the fixture cases
  and `make parity-local` the local diff.
  - `ccusage/`: an npm project that pins ccusage, with its lockfile
  - `cases.toml`: each case’s fixture set, urollup command, ccusage command path, join
    keys, urollup scope and compared metrics
  - `ledger.toml`: the explained-differences ledger
  - `compare.py`: the comparator
  - `local_diff.py`: the local aggregate diff
  - `test_*.py`: standard-library `unittest` tests, which pytest also collects,
    including the privacy sentinel test
- **Pinned ccusage:**
  - ccusage 20.0.20, published on 2026-08-15 and past the 14-day cool-off, is an exact
    `devDependencies` entry in `tests/parity/ccusage/package.json`, a separate npm
    project, so only parity jobs install it.
  - Its committed lockfile records the integrity of the launcher and of each native
    platform package. The platform packages are exact-version optional dependencies with
    no install scripts, so `npm ci --ignore-scripts` followed by `npm audit signatures`
    installs a runnable binary, and the supply-chain validator checks this lockfile like
    the root npm project’s.
  - The comparator runs the platform binary from `node_modules/@ccusage/` directly and
    requires `--version` to print the pinned version.
  - Runs never use `npx`, `bunx`, `pnpm dlx`, `nix run` or `@latest`. If no platform
    package fits a runner, the fallback is a `cargo build --locked` of the release tag’s
    commit, with the same version check.
  - Updating the pin is a pull request that moves to the latest release past the
    cool-off, reruns every case and retires the ledger entries that stop matching.
- **Isolated, deterministic runs:**
  - Both tools read a temporary copy of the same fixture roots.
    `HOME`, `XDG_CONFIG_HOME` and `XDG_DATA_HOME` point at an empty directory, and
    `CLAUDE_CONFIG_DIR` and `CODEX_HOME` point at the copy.
    The working directory has no `.ccusage/` folder, so neither tool sees other logs or
    configuration.
  - urollup discovers the same roots through those native variables, and from milestone
    0.3 it runs with `--no-capture`.
  - ccusage always runs with `--offline` and `--json`, and costs use `--mode calculate`.
    `NO_COLOR=1` and `LOG_LEVEL=0` are set.
  - Every case sets the same `--timezone` on both sides: `UTC`, plus
    `America/Los_Angeles` for DST fixtures.
    Weekly cases set the week start explicitly on both sides.
  - Interval cases map ccusage’s inclusive `--until` date to urollup’s half-open bound.
  - Cases use per-agent command paths such as `ccusage claude daily` and
    `ccusage codex session`. The unified `ccusage daily` is a separate case.
  - The report records each command line, tool version and exit code, and an expected
    ccusage failure is itself a ledger entry.
- **Comparison:**
  - **Normalized rows:** the comparator normalizes both JSON outputs into rows keyed by
    case, agent, period or top-level session, and model.
    Each row carries uncached input, output, cache writes, cache reads and total tokens,
    plus reasoning tokens for Codex.
  - **Field mapping:** fields map per agent: ccusage `inputTokens` is uncached input for
    Claude Code and Codex, and Codex `cacheCreationTokens` is always 0 at 20.0.20.
  - **Session join:** sessions join on native session ID, taken from ccusage’s
    `sessionId` or from the thread ID in a Codex `sessionFile`. Each case declares the
    urollup `--scope` that matches the ccusage path, so a difference in subagent
    attribution becomes a ledger entry, not a tolerance.
  - **Model rows:** per-model rows compare ccusage `modelBreakdowns`, or Codex `models`,
    with urollup `--group-by model`.
  - **One-sided rows:** a row present in only one tool is a difference.
  - **Tokens:** token counts compare exactly on fixtures, and every nonzero delta needs
    a ledger entry that states that exact delta.
  - **Shared rates:** from milestone 0.4, costs compare under shared rates.
    A generator writes a `urollup:PriceTable/v1` override for the fixture models from
    the LiteLLM snapshot that ccusage 20.0.20 embeds (commit `1a183ef`), and urollup
    reads it with `--prices`, so rate sources cannot differ.
  - **Cost tolerance:** ccusage’s `f64` amounts must match urollup’s exact decimals
    within USD 0.000001 per row.
  - **Bundled-table run:** a second run with urollup’s bundled table reports rate-source
    differences for information and never fails.
- **Explained-differences ledger:**
  - **Entry fields:** each entry in `ledger.toml` has an ID and the case, key and metric
    patterns it explains.
    It also records the expected delta, which is exact for fixtures and a sign and bound
    for the local corpus, and its cause: `ccusage-bug`, `dedupe`, `semantics`, `pricing`
    or `unsupported`. Every entry cites the research brief or a pinned ccusage source
    line, links the urollup design section, and states a retirement condition, such as
    the ccusage commit that fixes it.
  - **Failures:** the comparator fails when a difference matches no entry, when an entry
    matches no difference, when two entries match one difference, or when an entry lacks
    a citation. A stale entry fails too, so a ccusage fix retires its entry at the next
    pin update.
  - **Seed entries:** the ledger starts from the brief’s documented behaviors:

| Cause | ccusage 20.0.20 behavior | Source | First case |
| --- | --- | --- | --- |
| `ccusage-bug` | A Claude Code record whose line holds a nested null field loses its usage | [Existing implementations](../../research/research-2026-09-13-portable-agent-usage.md#existing-implementations) | 0.1 |
| `ccusage-bug` | `daily` double-counts a sidechain replay that precedes its parent; fixed on `main` in `a4b8420` | [Existing implementations](../../research/research-2026-09-13-portable-agent-usage.md#existing-implementations) | 0.1 |
| `dedupe` | Codex `daily`, `session` and `--since` runs deduplicate with different keys | [Existing implementations](../../research/research-2026-09-13-portable-agent-usage.md#existing-implementations) | 0.1 |
| `ccusage-bug` | Timestamps without 0 or 3 fractional digits are skipped for Claude Code and abort a Codex report | [Existing implementations](../../research/research-2026-09-13-portable-agent-usage.md#existing-implementations) | 0.1 |
| `unsupported` | `.jsonl.zst` rollouts, `token_usage_record` and `subagent_history_start_ordinal` are not read | [Existing implementations](../../research/research-2026-09-13-portable-agent-usage.md#existing-implementations) | 0.1 |
| `semantics` | Codex cache creation tokens are always 0; fixed on `main` in `15b3bef` | [ccusage feature inventory](../../research/research-2026-09-13-portable-agent-usage.md#ccusage-feature-inventory) | 0.1 |
| `semantics` | Session reports filter by last-activity date and drop zero-token sessions; `<synthetic>` and advisor models are listed differently | [ccusage feature inventory](../../research/research-2026-09-13-portable-agent-usage.md#ccusage-feature-inventory) | 0.1 |
| `pricing` | Fuzzy model matching, marginal per-category long-context rates, unpriced tokens as cost 0, and Codex tiers from `config.toml` | [Existing implementations](../../research/research-2026-09-13-portable-agent-usage.md#existing-implementations) | 0.4 |
| `ccusage-bug` | Pi fork replays count twice; fixed on `main` in `809eeb6` | [Existing implementations](../../research/research-2026-09-13-portable-agent-usage.md#existing-implementations) | Phase 2 |

- **Excluded from comparison:** `blocks` is never compared
  ([Decision 10](../../../urollup-design.md#decision-10-recorded-usage-windows)), and
  neither is `--mode auto` or `display`, which mix recorded `costUSD` with calculated
  cost. Other agents’ usage appears only in local unified runs, as `unsupported` entries.
- **Local corpus:**
  - **Runs:** the maintainer runs `local_diff.py` with `make parity-local`, by hand or
    from a local scheduler, never in CI. It runs both tools on the default roots with
    the same timezone and per-agent ccusage paths.
    The interval ends at the start of the current local day, so sessions still being
    written cannot move totals between the two runs, and urollup runs with
    `--no-capture`, so usage retained from deleted logs is not counted on one side only.
  - **Output:** rows are joined in memory, and the script writes only aggregates: tool
    versions, platform, timezone and interval, per-metric totals for each tool, per-day
    and per-model deltas, counts of matched and one-sided sessions, a histogram of
    per-session relative deltas, and the delta each ledger entry explains with the
    unexplained residual.
  - **Explained amounts:** until milestone 0.5, local entries give a sign and bound.
    Once `requests` exists, each local ledger entry names the urollup diagnostic or
    request property that identifies the affected requests, such as a nested null field
    or a compressed rollout, and the script sums those requests’ tokens in memory.
  - **Privacy:** the script never writes or prints content, paths, project names, or
    session, request or thread IDs.
    A model name appears only when it matches urollup’s price table or the pinned
    LiteLLM snapshot, and other models count as `other`. A CI test runs the script on a
    sentinel fixture whose paths, project names, IDs, prompt text and custom model names
    carry unique markers, and fails if any marker reaches stdout, stderr or the output
    file.
  - **Records:** after review, the maintainer commits one record per urollup release to
    `bench/results/parity/`, beside the benchmark records, since both are
    reference-machine aggregates of the consented corpus.
    An unexplained residual above 0.1% of a metric’s daily total, a proposed threshold
    revised from recorded results, gets a bead.
- **CI results:** CI uploads each fixture run’s per-case report as a job artifact;
  fixture results are regenerated rather than committed.

The harness grows with the milestones:

| Milestone | Cases | Compared |
| --- | --- | --- |
| 0.1 | `ccusage claude` and `ccusage codex` `daily` and `session`, with and without `--since`, plus unified `daily`, on the `claude-project` and `codex-rollout` fixtures; the local diff | Token categories per day and per session |
| 0.4 | The same paths with `--mode calculate` under shared rates, plus `--breakdown` | Costs per day, session and model, and the informational bundled-table run |
| 0.5 | `weekly` and `monthly`, `--instances` and `--project` against project grouping and filters, DST and interval-boundary fixtures, unified reports, and every other [§10.6](../../../urollup-design.md#106-ccusage-use-case-coverage) row with a ccusage counterpart; request-attributed explained amounts in the local diff | The full feature matrix, recorded in §10.6 |
| Phase 2 | `ccusage pi` paths with the Pi adapters, and `statusline` if confirmed | Pi tokens and costs, and status line session cost |

### Performance targets

Performance gates are proposed targets, not measured claims; correctness comes first,
and targets change here only with recorded results.
Gated commands write no capture entries
([Decision 26](../../../urollup-design.md#decision-26-benchmarks),
[§8.3](../../../urollup-design.md#83-execution-and-performance)).

| Target | Corpus | Measured commands | Proposed gate |
| --- | --- | --- | --- |
| Small CLI report | `bench-small`, about 64 MiB | `daily` and `report --session`, end to end | Median wall time under 1 s |
| Standard rollups | `bench-1g`, 1 GiB | `daily`, `monthly --group-by account,model,effort` and `sessions` | Median under 10 s; peak RSS under 512 MiB |
| Resident queries | `bench-1g` loaded by `urollup serve` | Fixed UI query set over HTTP after the snapshot is ready | p95 server latency under 200 ms per query |
| Status line, if the candidate is confirmed | A `bench-small` session with subagents | `statusline` from hook input, session segment, end to end | p95 wall time under 250 ms, within Claude Code’s 300 ms debounce |
| Capture writes | `bench-small` and `bench-1g` | `sessions --capture --capture-idle 0`, writing zstd level 3 entries into an empty store | Recorded throughput and peak RSS, not gated |

- **Corpora:** a seeded generator expands sanitized fixture templates and writes a
  manifest of bytes, files, sessions, requests and dialect mix.
  Bytes split evenly between `claude-project` and `codex-rollout`, with streamed
  updates, repeated blocks, subagent and forked threads, cumulative counters, partial
  tails, unpriced models and one oversized record; `bench-1g` scales distinct sessions,
  since copies reconcile away.
  The consented corpus recalibrates the mix, recording only aggregate size, mix and
  timings.
- **Machines:** absolute gates run on a reference Apple silicon laptop with at least 10
  cores, 16 GiB RAM and an internal SSD, on AC power and otherwise idle; a replacement
  reruns the previous release first.
  `bench-pr` builds each pull request and its merge base on one `ubuntu-24.04` runner
  ([4 vCPUs, 16 GB](https://docs.github.com/en/actions/reference/runners/github-hosted-runners))
  and alternates their runs on `bench-small`; `bench-scheduled` compares `main` with the
  latest release tag on `bench-1g`.
- **Harness:** release binaries run as subprocesses, 3 warmup and 10 measured runs per
  command on a warm filesystem cache (cold runs recorded, not gated), with peak RSS from
  `getrusage`; resident queries use 5 warmup and 50 measured requests.
  Each run writes a JSON record of build, machine, corpus, command, run counts, cache
  state, timings, peak RSS and throughput; reference-laptop records are committed under
  `bench/results/` for each release.
- **Regressions:** a pull request that raises median wall time or peak RSS on
  `bench-small` by more than 10% merges only with a fix or accepted justification; a
  scheduled `bench-1g` regression over 10% gets a bead resolved before the next release;
  and a release missing a reference-laptop gate ships only with the target revised from
  recorded results. Request exports and exact percentiles declare separate memory costs.

## Rollout Plan

urollup is a standalone product with no dependency on a consuming repository.
It first runs in shadow mode against retained logs and existing reports, which stay in
use until supported reports are validated.
Binaries with the embedded UI are published only after fixture, packaged-binary,
feature-matrix and Testing Strategy release checks pass, and dependencies are pinned
under the supply-chain policy as crates and frontend tools are chosen.

Release mechanics follow the baseline’s
[targets, channels and versioning](../../research/research-2026-09-13-rust-cli-engineering-baseline.md#targets-channels-and-versioning),
within the confirmed
[release scope](../../../urollup-design.md#decision-27-release-scope):

- **Targets:** static musl Linux x86_64 and arm64, macOS arm64 and x86_64, and Windows
  x86_64, each built and smoke-tested on a native runner.
  The musl build is benchmarked before choosing a global allocator; Windows arm64 waits
  for demand.
- **Channels:** one `release.yml` publishes from a protected `release` environment to
  GitHub Release archives with `SHA256SUMS` (the channel the cloud skill pins),
  crates.io `urollup-core` and `urollup` in one invocation, and PyPI binary wheels.
  Registries use trusted publishing, except a short-lived scoped token for the first
  crates.io upload. A dispatch dry run skips only upload, and reruns skip identical
  artifacts and fail on different bytes under one version.
  Homebrew, npm and cargo-binstall wait for demand.
  `urollup` and `urollup-core` were unregistered on crates.io and PyPI on 2026-09-13.
- **Verification:** `--locked` builds from the tagged commit, with a build-provenance
  attestation for every archive and wheel; no GPG or minisign signature.
- **Versioning:** SemVer from `[workspace.package] version`, checked against the tag by
  `build.rs`. Before 1.0, a minor release may change CLI or JSON contracts; report,
  bundle, query and identity contracts carry their own versions, recorded in
  `CHANGELOG.md` ([§5.6](../../../urollup-design.md#56-versioning-and-compatibility)).

## Decisions

The design doc records every decision and open question:

- **Confirmed:** 27 decisions, each with its choice, rationale, tradeoffs and date, in
  [§10.1](../../../urollup-design.md#101-design-decisions).
- **Candidate:** 14 proposed decisions already reflected in the design, pending
  maintainer confirmation, in [§9.1](../../../urollup-design.md#91-candidate-decisions),
  including five from the 2026-09-15
  [ccusage use-case coverage](../../../urollup-design.md#106-ccusage-use-case-coverage)
  review.
- **Queued:** 10 review decisions raised by the squares and metaproc code reviews,
  walked through one at a time in bead `uro-gxen`, in
  [§9.2](../../../urollup-design.md#92-queued-review-decisions).
- **Open:** 3 questions on cloud export formats, pricing bases and billing exports, in
  [§9.3](../../../urollup-design.md#93-open-questions).

## References

- [urollup design specification](../../../urollup-design.md)
- [Portable research brief](../../research/research-2026-09-13-portable-agent-usage.md),
  including its
  [ccusage feature inventory](../../research/research-2026-09-13-portable-agent-usage.md#ccusage-feature-inventory)
- [Rust CLI engineering baseline](../../research/research-2026-09-13-rust-cli-engineering-baseline.md)
- [squares code review](../../research/research-2026-09-14-squares-code-review.md) and
  [metaproc and qm review](../../research/research-2026-09-14-metaproc-code-review.md)
- [Agent tool source reviews](../../research/research-2026-09-14-agent-tool-source-reviews.md)
- [Log throughput spike](../../../../explorations/log-throughput/README.md)
- [fdu](https://github.com/jlevy/fdu) and
  [flowmark-rs](https://github.com/jlevy/flowmark-rs), the reference Rust repositories
  for the baseline
- [Agentfdr](https://github.com/kamihork/agentfdr)
- [ccusage](https://github.com/ccusage/ccusage)

<!-- This document follows common-doc-guidelines.md.
See github.com/jlevy/practical-prose and review guidelines before editing.
-->
