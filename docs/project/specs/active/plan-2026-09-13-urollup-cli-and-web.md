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
decisions, and [glossary](../../../urollup-design.md#93-glossary).
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
| Phase 1: accounting core and useful uncached CLI (milestones 0.1–0.5) | Claude Code and Codex adapters for persistent and captured-stream dialects, the reconciled ledger, accounting, prices, summaries, bundles, the capture store and the Phase 1 CLI | [§2](../../../urollup-design.md#2-sources-and-capture-layer) without Pi dialects or cache reads; [§3](../../../urollup-design.md#3-ledger-and-identity-layer) with observed purpose only; [§4.1](../../../urollup-design.md#41-measure-contracts)–[§4.3](../../../urollup-design.md#43-time-grouping-and-percentiles) and [§4.5](../../../urollup-design.md#45-price-table), plus provider limit observations; [§5](../../../urollup-design.md#5-artifact-layer) without database input; [§6.1](../../../urollup-design.md#61-workflows-and-session-selection)–[§6.6](../../../urollup-design.md#66-report-content-and-examples) without Phase 2 commands; [§8.1](../../../urollup-design.md#81-workspace-and-crate-structure)–[§8.2](../../../urollup-design.md#82-engineering-conventions) and the [uncached engine](../../../urollup-design.md#uncached-engine) |
| Phase 2: web UI, workflow reports and broader evidence | Capture cache reads, `serve`, the reporting skill, `compare`, `check`, the `windows` report, Pi adapters, and configured purpose with annotation sets | [§2.5](../../../urollup-design.md#25-capture-store-and-cache) cache reads; [§7](../../../urollup-design.md#7-serving-layer-optional); [§6.7](../../../urollup-design.md#67-reporting-skill-and-cloud-workflow); [§6.3](../../../urollup-design.md#63-commands) Phase 2 commands; [§4.4](../../../urollup-design.md#44-usage-windows); [§2.1](../../../urollup-design.md#21-dialects-and-discovery) and [§6.2](../../../urollup-design.md#62-current-session-detection) for Pi; [§3.5](../../../urollup-design.md#35-purpose-and-annotations) |
| Phase 3: idempotent persistent cache | The ledger and query cache, and urollup’s own store as input | [Ledger and query cache](../../../urollup-design.md#ledger-and-query-cache-later); [§5.1](../../../urollup-design.md#51-portable-inputs-and-artifacts) database input |

Items the design marks Later without a phase, such as the
[account registry](../../../urollup-design.md#46-accounts-and-plans-later), are listed
in the design’s
[future enhancements](../../../urollup-design.md#92-future-enhancements).
Queued review decisions get phase items only once confirmed
([§10.2](../../../urollup-design.md#102-queued-review-decisions)).

## Implementation Plan

### Phase 1: Accounting core and useful uncached CLI

Phase 1 ships as five milestones, each a usable and tested pre-1.0 release.
The capture store lands only after the uncached engine is the correctness reference.

#### Milestone 0.1: Uncached Claude Code and Codex reports

- [ ] Scaffold a minimal repository to the engineering baseline: workspace, toolchain
  pin, lint and format configuration, supply-chain policy, `make check` and `make fix`,
  the npm dev project for tryscript, and the CI jobs those gates need; prove each gate
  fails on a committed violation
  ([§8.1](../../../urollup-design.md#81-workspace-and-crate-structure),
  [§8.2](../../../urollup-design.md#82-engineering-conventions)).
- [ ] Freeze public sanitized fixtures, including the research brief’s
  [double-counting cases](../../research/research-2026-09-13-portable-agent-usage.md#synthetic-double-counting-example)
  ([§1.2](../../../urollup-design.md#12-why-urollup-exists),
  [§3.4](../../../urollup-design.md#34-dialect-reconciliation-rules)).
- [ ] Implement analytical identities, the normalized ledger, reconciliation, ownership
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
  diagnostics, `--capture-idle`, default-discovery scope and `--capture`, atomic
  replacement entries in the platform data directory, retained reads for deleted
  sources, retained versions linked by thread for rewritten sources, runs without
  capture when the store is unwritable, `--no-capture`, `capture status` and
  `capture prune`, with equivalence goldens against the uncached engine
  ([§2.4](../../../urollup-design.md#24-capture-and-export-strip-policies),
  [§2.5](../../../urollup-design.md#25-capture-store-and-cache)).
- [ ] Measure and record capture-write throughput at zstd level 3 before capture ships
  on by default ([performance targets](#performance-targets)).

#### Milestone 0.4: Prices

- [ ] Add the reviewed price table and its `PriceTable` contract, `--prices` and
  config-directory overrides, staleness diagnostics, `--require-priced` and golden
  repricing tests ([§4.5](../../../urollup-design.md#45-price-table)).

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
  disagreements from source records rather than treating either as an oracle
  ([§1.5](../../../urollup-design.md#15-non-goals),
  [existing implementations](../../research/research-2026-09-13-portable-agent-usage.md#existing-implementations)).

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
  and imported multi-account and cloud-export fixtures, individually
  ([§2.1](../../../urollup-design.md#21-dialects-and-discovery),
  [§6.2](../../../urollup-design.md#62-current-session-detection)).
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
  idle threshold, uncaptured `--source` logs, an unwritable store, retained and
  rewritten sources, and identical results with the store disabled.
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
  [§9.1](../../../urollup-design.md#91-design-decisions).
- **Candidate:** 9 proposed decisions already reflected in the design, pending
  maintainer confirmation, in
  [§10.1](../../../urollup-design.md#101-candidate-decisions).
- **Queued:** 11 review decisions raised by the squares and metaproc code reviews and
  the PR #3 design review, walked through one at a time in beads `uro-gxen` and
  `uro-6y0j`, in [§10.2](../../../urollup-design.md#102-queued-review-decisions).
- **Open questions** on cloud export formats, pricing bases and billing exports are in
  [§10.3](../../../urollup-design.md#103-open-questions).

## References

- [urollup design specification](../../../urollup-design.md)
- [Portable research brief](../../research/research-2026-09-13-portable-agent-usage.md)
- [Rust CLI engineering baseline](../../research/research-2026-09-13-rust-cli-engineering-baseline.md)
- [squares code review](../../research/research-2026-09-14-squares-code-review.md) and
  [metaproc and qm review](../../research/research-2026-09-14-metaproc-code-review.md)
- [Log throughput spike](../../../../explorations/log-throughput/README.md)
- [fdu](https://github.com/jlevy/fdu) and
  [flowmark-rs](https://github.com/jlevy/flowmark-rs), the reference Rust repositories
  for the baseline
- [Agentfdr](https://github.com/kamihork/agentfdr)
- [ccusage](https://github.com/ccusage/ccusage)

<!-- This document follows common-doc-guidelines.md.
See github.com/jlevy/practical-prose and review guidelines before editing.
-->
