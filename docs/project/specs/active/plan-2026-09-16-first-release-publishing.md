---
title: "urollup 0.1.0 Publishing and Distribution"
description: End-to-end plan for building, validating, publishing, and verifying the first urollup release through GitHub Releases, crates.io, and PyPI binary wheels for uvx.
author: Joshua Levy with LLM assistance
date: 2026-09-16
status: Active; Phase 1 release machinery implemented in PR #36, awaiting review; publication not started
---
# Feature: urollup 0.1.0 Publishing and Distribution

## Overview

urollup 0.1.0 is one Rust product released from one reviewed commit under one version.
It reaches users through three complementary channels:

1. GitHub Releases provides the primary prebuilt archives, checksums, release notes, and
   provenance attestations.
2. crates.io publishes `urollup-core` for Rust consumers and `urollup` for users who
   prefer `cargo install` and accept a local source build.
3. PyPI publishes the compiled `urollup` executable in Maturin `bin` wheels.
   This gives agents and people exact-version, isolated execution with `uvx` and
   persistent installation with `uv tool install`, without adding Python logic to
   urollup.

The same Cargo workspace version and git tag identify all three channels.
Build jobs have no publishing authority; packaged artifacts pass native smoke tests
before a protected `release` environment grants short-lived publishing credentials.
A release run may resume an identical partial publication, but it never replaces
different bytes under an existing version.

The first public alpha is `0.1.0`, published after milestone 0.1 passes its local
real-log acceptance gate.
Later Phase 1 milestones become subsequent pre-1.0 releases; they do not delay or expand
the first alpha.

This focused plan owns the release machinery and the first-release runbook.
The [main implementation plan](plan-2026-09-13-urollup-cli-and-web.md) owns product
scope, milestones, and acceptance of urollup itself.
The
[engineering baseline](../../research/research-2026-09-13-rust-cli-engineering-baseline.md)
owns the broader repository standards.

## Current Readiness and Critical Path

This section is the project’s single record of release readiness.
The README, `AGENTS.md`, the product plan and the governing review link here rather than
restating blockers, so a status change is one edit.

As of 2026-10-11 UTC, readiness is divided into four gates so a locally testable alpha
is not confused with a publishable release:

| Gate | Exit condition | Current state |
| --- | --- | --- |
| Automated 0.1 product | The uncached Claude Code and Codex adapters, exact session selection, `report`, `daily`, `sessions`, JSON and table output, terminal-aware color, stderr-only interactive progress, plain machine streams, sanitized fixture parity, goldens and repository gates pass | Implementation stack merged at main `4c55617`; all 15 jobs passed in [CI run 35492922611](https://github.com/jlevy/urollup/actions/runs/35492922611). The CLI and local-QA corrections `uro-oz6w` and `uro-qg1a` merged in PRs #25 and #24 on 2026-10-10. Every [correctness fix](#open-correctness-fixes) the 2026-10-10 real-history rerun found has merged: PR #32 on 2026-10-10 and PRs #33, #34 and #19 on 2026-10-11. The residual shapes listed there stay on `main`, each with a bead. Process-wide admission has its design, ledger and cost model (PRs #27–#29, `uro-6pi8` slices 1–3) but is not yet wired into ingestion |
| Local alpha acceptance | The privacy-tested local aggregate mode and pinned-ccusage diff run against consented logs; G1 passes; unobserved Claude record shapes and remaining maintainer decisions are resolved or explicitly deferred | G1 (`uro-d36a`) and full-history QA (`uro-ky6c`) remain open. G1 waits on a real-history rerun now that the [correctness fixes](#open-correctness-fixes) have merged (`uro-mxv0`), and on process-wide safety and scale evidence (`uro-zrr0`). A successful exploratory live run does not close these gates |
| Packaging rehearsal | Every archive, wheel and Cargo package is built and validated through the credential-free release path, with the complete manifest and no external writes | Machinery implemented in [PR #36](https://github.com/jlevy/urollup/pull/36) on 2026-10-13 ([Phase 1 notes](#phase-1-implementation-notes)); its dispatch rehearsals on the pull-request branch, linked from the PR, built, natively validated and assembled all ten artifacts and both Cargo packages with no external writes. Still open: the maintainer’s environment and publisher setup ([runbook](../../../publishing.md#maintainer-setup-before-the-first-tag)) and a rehearsal of the accepted 0.1.0 commit on `main` |
| Publication | The accepted 0.1 commit is merged, release documentation is final, protected publishers are configured, `v0.1.0` is approved, and every registry-backed installation probe passes | Blocked by local acceptance and the packaging rehearsal |

The local alpha can be installed and exercised before packaging machinery exists.
Publication begins only after both the product-acceptance and packaging-rehearsal gates
pass; neither gate is evidence for the other.
The
[memory and scale policy](plan-2026-09-16-scalable-ingestion.md#accepted-scale-and-memory-policy-2026-09-27)
the maintainer approved on 2026-09-27 retired the representative 512 MiB/10-second
blockers. `uro-zrr0` still gates G1 on process-wide admission safety (`uro-6pi8`),
density-based scale evidence (`uro-z1h1`) and representative evidence (`uro-erqo`).
Retiring those thresholds does not close the release gate.
Hybrid spill is a follow-up.
The [whole-history analysis workflow](plan-2026-09-20-usage-analysis-workflow.md) is G5
across milestones 0.2–0.5. Cache-hit request metrics, tool/time reports and list-price
valuation must not be advertised as completed 0.1 features.

### Open Correctness Fixes

This list records the correctness defects found on `main` and their state as of the date
above. PRs #16 and #18 merged on 2026-10-09, PRs #26 and #32 on 2026-10-10, and PRs #33,
#34 and #19 on 2026-10-11. A local real-history rerun on 2026-10-10 found the migrated
Guardian and subagent defect (`uro-jqc3`) and the copied Claude subagent defect
(`uro-wvgc`), and review of the `uro-jqc3` fix found the related migrated-fork defect
(`uro-p9ua`). Codex and Claude totals stay provisional until the comparison is rerun on
`main` with these fixes (`uro-mxv0`).

- **Codex paginated-fork double count (`uro-kpbp`):** a paginated child rollout that
  copies its parent’s history before an explicit boundary counts that copied prefix
  again. In a synthetic case, a parent with 100 tokens and a child that copies that
  100-token prefix and adds 20 tokens reports 220 instead of 120, with
  `coverage.complete: true` and no diagnostic.
  Fixed in [PR #16](https://github.com/jlevy/urollup/pull/16), merged 2026-10-09.
- **Codex counter-only usage beside usage records (`uro-h2sf`, fixed for root rollouts
  in PR #26, merged 2026-10-10):** any `token_usage_record` in a Codex rollout made
  every cumulative `token_count` in it count nothing.
  A session that Codex before 0.153 started and a later release resumed loses its
  pre-upgrade usage, and a rollout that an older release appended counter-only turns to
  loses those turns, with `coverage.complete: true` and no diagnostic.
  In the `codex-rollout/mixed-counter-direct` fixture the history reports 90,000 instead
  of 139,700 tokens. [PR #26](https://github.com/jlevy/urollup/pull/26), merged
  2026-10-10, fixes root rollouts (no parent, fork origin or history boundary), which
  hold most of this usage.
  Forks and subagents keep the gap, and a nested all-legacy fork or subagent counts its
  intermediate thread’s usage twice (`uro-r8si`, present on `main`).
- **Source reading and Codex usage (fixed in
  [PR #18](https://github.com/jlevy/urollup/pull/18), merged 2026-10-09):** `.jsonl.gz`
  sources are silently excluded (`uro-nc34`); a source deleted or compressed during a
  run aborts the command (`uro-2abk`); a corrupt, truncated or unreadable source lowers
  totals while coverage stays complete (`uro-1h5s`); Codex subagent requests report an
  `unknown` model and effort (`uro-5nkv`); and a lowered Codex cumulative total is
  charged to one request (`uro-v1c9`).
- **Codex Guardian and subagent usage excluded as copied history (`uro-jqc3`, fixed in
  PR #32, merged 2026-10-10):** Codex’s legacy-to-paginated rollout migration sets a
  child’s `subagent_history_start_ordinal` past the file’s last line, so every line,
  including the child’s own `token_usage_record` lines, falls before the boundary and is
  excluded as the parent’s copied history, with `coverage.complete: true` and no
  diagnostic. The rerun found this in many migrated Guardian-review and subagent rollouts
  ([research](../../research/research-2026-10-10-codex-paginated-subagent-boundary.md)).
  In the `codex-rollout/guardian-migrated-boundary` fixture `main` reports 9,600 instead
  of 21,190 tokens. [PR #32](https://github.com/jlevy/urollup/pull/32), merged
  2026-10-10, owns a usage record by the thread it names, decides a migrated
  counter-only child’s unassigned lines by the turns its parent root recorded and by the
  parent’s counter totals, and counts a `compacted` record that is the only record of
  its response. Only a root with neither lineage nor an old-format subagent’s nested
  `thread_spawn` parent (`uro-3b12`) decides a child’s turns.
  Until the boundary, or a line naming the child, a later turn or total the parent
  recorded, or a line naming another thread, voids an inferred start.
  What it still cannot place is a `codex-history-boundary-unverified` coverage gap, so
  on real history whole-history Codex coverage stays partial where a migrated child’s
  parent is not a discovered root, its turns have no IDs, a bounded migration dropped
  responses, or its first own step does not continue the copied total.
  One shape still counts twice: a subagent spawned in a turn that a migrated root’s
  rollback planning dropped (`uro-eh0d`, also on `main` for unmigrated children); a
  local real-history check found none.
  A real-history rerun after the fix still gates G1 (`uro-d36a`), milestone 0.1
  (`uro-n8h5`) and the core CLI delivery slice (`uro-aakb`).
- **Migrated legacy Codex user forks count their parent’s prefix again (`uro-p9ua`,
  fixed in PR #33, merged 2026-10-11):** the same migration drops a legacy fork’s copied
  `session_meta` and gives it no boundary, so a migrated counter-only user fork counts
  its parent’s copied counters as its own.
  In a synthetic case the fork reports 340 instead of 180, with
  `coverage.complete: true`; in the `codex-rollout/migrated-user-fork` fixture `main`
  reports 61,100 instead of 37,350 tokens.
  [PR #33](https://github.com/jlevy/urollup/pull/33) judges a counter-only paginated
  rollout with a parent link, and no boundary, `history_base`, subagent `thread_source`
  or `source`, seeded first counter or other thread’s header, as if its boundary lay
  past its last line. PR #32’s turn, total and contradiction rules then decide its lines
  from the first. A fork whose parent is absent or has lineage, or whose turns have no
  IDs, reports its undecided steps as a `codex-history-boundary-unverified` gap, never a
  count. Its round 2 also keeps an open inferred start’s lines the child’s through a
  settings event without a thread ID, for migrated subagents too.
  Known shapes that remain
  ([research](../../research/research-2026-10-10-codex-paginated-subagent-boundary.md#residual-risks)):
  - The `uro-eh0d` rollback double count also reaches migrated user forks: a fork taken
    before its parent rolled back a turn the fork copied (synthetic: 280 against a real
    220), or a fork whose parent rolled back its first turn and is absent, holds usage
    records or has lineage, so that no counter-only root supplies the totals the seeded
    check compares and the fork counts its copy of that first counter as its own (350
    against 190 when a parent with usage records is present).
    A local real-history check found no migrated user fork with own totals inside a
    parent rollback window.
  - A native counter-only fork written before 0.153 still counts the parent total Codex
    seeded it with, as on `main` (`uro-p12c`).
  - A fork’s own step that repeats a parent total in every count is dropped as the
    parent’s copy, as PR #32’s rule does for a migrated subagent (`uro-uwv0`).
- **Copied Claude subagent transcripts make their responses ambiguous (`uro-wvgc`, fixed
  in PR #34, merged 2026-10-11):** a resumed or forked Claude Code session can hold a
  byte-identical copy of an earlier session’s subagent transcript in its own
  `subagents/` directory, whose lines still name the earlier session.
  `main` takes a subagent’s thread from the directory, so the original and the copy both
  prove ownership of the same responses: whenever the copy’s `src-` ID sorts first, each
  response lands on Claude Code’s unowned row with a `conflicting-owners` diagnostic,
  and every copy adds an empty subagent thread.
  Totals are unchanged.
  A local real-history check on 2026-10-10 found a handful of such copies causing every
  ambiguous Claude request.
  In the `claude-project/resumed-subagent-copy` fixture `main` reports 2 of 7 requests
  as ambiguous and 5 threads instead of 4.
  [PR #34](https://github.com/jlevy/urollup/pull/34) identifies a subagent thread by the
  session its lines name and the agent ID, so a copy is the original subagent and still
  counts once after Claude Code deletes the original.
  Exact session selection reads a copy whenever the session its lines name is selected,
  by native or `thr-` ID, even after Claude Code deletes that session’s own files or
  when they lie in another project directory or root.
  Two residual shapes remain: across project directories and roots, the session
  transcript’s path and the copy’s own selectors still miss each other (`uro-4v0g`), and
  once a session’s transcript is deleted another fork’s replay can still decide a spawn
  (`uro-x146`). A real-history rerun must confirm the fix (`uro-mxv0`).
- **Codex cache writes counted twice, and inconsistent usage aborting runs (`uro-381f`,
  fixed in PR #19, merged 2026-10-11):** Codex normalization subtracted only cache reads
  from inclusive input, so every cache-write token was also counted as ordinary input.
  That writes sit inside `input_tokens` is an inference from Codex’s Responses mapping,
  and no observed record has had cache writes
  ([incident](../../../formats/accounting-incidents.md#codex-cache-write-double-counting)).
  One Codex usage object whose input was below its cache reads stopped every
  whole-history command for both agents.
  Such usage, now also input below reads plus writes, is excluded with one
  `codex-inclusive-input-underflow` diagnostic and an `InconsistentUsage` coverage gap
  per rollout, and the run continues.

## Goals

- Publish urollup 0.1.0 from one immutable commit and version through GitHub Releases,
  crates.io, and PyPI.
- Make the fastest agent path an exact-version command such as
  `uvx --isolated urollup@0.1.0 --version`, while keeping the installed program a Rust
  executable with no Python runtime logic.
- Build every channel artifact once per release run, validate the packaged artifact on
  its target platform, and publish those validated bytes where the registry permits.
- Keep the release workflow least-privilege, review-gated, reproducible, independently
  retryable by channel, and explicit about partial state.
- Provide a rehearsable path that exercises every release step except external
  publication before the tag is created.
- Leave enough evidence to identify the commit, toolchain, target, inputs, artifact
  digests, and completed channels after the workflow logs expire.

## Non-Goals

- Importable Python bindings.
  A later, separately versioned extension over `urollup-core` is tracked independently;
  the first PyPI package is a binary distribution only.
- A Python downloader, Python wrapper around the CLI, or Python implementation code.
- Homebrew, npm, Nix, `cargo-binstall`, self-update, installer scripts, Linux packages,
  containers, or Windows arm64. These add permanent channel or target obligations and
  wait for demonstrated demand.
- An sdist on PyPI. Source installation is already served by crates.io, and an sdist is
  not published until its isolated build path is supported and tested.
- GPG or minisign signatures.
  GitHub provenance attestations and `SHA256SUMS` are the first-release integrity paths.
- Atomic visibility across independent registries.
  No system can make GitHub, crates.io, and PyPI publish in one transaction; the
  workflow instead detects, records, and safely resumes partial publication.

## Background

Rust’s first-party install path is `cargo install`, which builds the crate locally and
therefore requires a Rust toolchain and compile time.
`cargo-binstall` can consume prebuilt release assets, but it is a separate tool users
must install first. Direct GitHub downloads are universal but require target selection,
extraction, checksum verification, and `PATH` setup.

`uvx` provides a simpler agent-facing contract: it resolves an exact package version
into an isolated cached environment and runs its command.
Maturin’s `bindings = "bin"` mode packages a Rust executable as a wheel script placed on
that environment’s `PATH`. The wheel is a distribution envelope; urollup remains a
native executable and does not import or embed Python.

The channels serve different audiences rather than duplicating one another:

| Channel | Audience | Installation contract |
| --- | --- | --- |
| PyPI `urollup` | Agents, Python-tool users, and users who already have uv | `uvx --isolated urollup@0.1.0 …` or `uv tool install urollup==0.1.0` downloads a wheel and runs its Rust executable |
| GitHub Releases | Sandboxes, CI, and users who want an auditable prebuilt binary without a package manager | Download the target archive, verify `SHA256SUMS` and optionally the GitHub attestation, extract, and run |
| crates.io `urollup` | Rust users with a toolchain | `cargo install --locked urollup@0.1.0` builds from the published source package |
| crates.io `urollup-core` | Rust applications embedding the accounting engine | Add the exact compatible crate version as a Cargo dependency |

The root `pyproject.toml` remains development-only and continues to declare
`urollup-dev` with `package = false`. PyPI distribution metadata lives in
`crates/urollup/pyproject.toml` beside the binary crate, points Maturin at the adjacent
Cargo manifest, and derives the release version from Cargo.
Publishing must not turn Python into a Rust build or runtime dependency.

## Design

### Release Identity and Invariants

The release unit is the urollup product: the CLI, core crate, archives, and wheels share
version `0.1.0` and tag `v0.1.0`.

- `[workspace.package].version` is the version source.
- The release plan requires `v${workspace_version}` and refuses any mismatch among the
  tag, Cargo packages, PyPI metadata, archive names, release manifest, and
  `urollup --version`.
- Every artifact comes from the tagged commit with the committed `Cargo.lock` and pinned
  release tools. No floating action tag, container tag, package version, or
  runner-installed packager is a release input.
- A dispatch rehearsal accepts an explicit commit and version but has no upload
  credentials and cannot publish.
  Publication is tag-triggered only.
- A concurrency key per version prevents two publishing runs for the same release.
- A required artifact or platform failure stops publication; there is no silent partial
  target matrix.

Before 1.0, a minor version may change the CLI or JSON contract as documented in the
main plan. Report, bundle, query, and identity contract versions remain independent and
are recorded separately in `CHANGELOG.md`.

### Artifact Matrix

GitHub archives optimize for direct, portable binaries.
PyPI wheels optimize for the platform tags uv and Python installers select.
These are separate build products even when they target the same CPU.

| Host | GitHub archive | PyPI wheel | Native validation environment |
| --- | --- | --- | --- |
| Linux x86_64 | `x86_64-unknown-linux-musl` static binary in `.tar.gz` | `manylinux_2_17_x86_64` Maturin `bin` wheel | x86_64 Ubuntu runner; wheel built in an audited manylinux environment |
| Linux arm64 | `aarch64-unknown-linux-musl` static binary in `.tar.gz` | `manylinux_2_17_aarch64` Maturin `bin` wheel | arm64 Ubuntu runner; wheel built in an audited manylinux environment |
| macOS Apple silicon | `aarch64-apple-darwin` binary in `.tar.gz`, macOS 11.0 minimum | macOS arm64 Maturin `bin` wheel, macOS 11.0 minimum | Apple-silicon macOS runner |
| macOS Intel | `x86_64-apple-darwin` binary in `.tar.gz`, macOS 11.0 minimum | macOS x86_64 Maturin `bin` wheel, macOS 11.0 minimum | Intel macOS runner |
| Windows x86_64 | `x86_64-pc-windows-msvc` executable in `.zip` | Windows x86_64 Maturin `bin` wheel | x86_64 Windows runner |

The release implementation sets and verifies `MACOSX_DEPLOYMENT_TARGET=11.0` for both
macOS architectures rather than inheriting a runner default.
Linux wheels are genuine manylinux builds; a musl executable is not relabeled as
manylinux. First-release musl users use the static GitHub archive.
Musllinux wheels can be added later if uvx-on-Alpine demand justifies another tested
matrix.

Every GitHub archive contains only:

- `urollup` or `urollup.exe`
- `README.md`
- `LICENSE`

Names follow `urollup-0.1.0-<rust-target>.<archive-extension>`. The release also carries
all built wheels, `SHA256SUMS`, `release-manifest.json`, and, after all post-publish
probes pass, `release-evidence.json`. The immutable manifest records the release
version, tag, commit, Rust toolchain, target or wheel platform, enabled features,
artifact size, SHA-256 digest, and producing workflow run.
The evidence record names each channel, the published files or packages it verified, and
the result of each clean installation probe.
Artifact identity and mutable workflow progress are not conflated in one file.

### PyPI Binary Package

The PyPI distribution and command are both named `urollup`.

- Maturin uses `bindings = "bin"` and the `crates/urollup/Cargo.toml` manifest.
- The wheel exposes exactly one console executable, `urollup`.
- Wheel tags are Python-ABI-independent and platform-specific.
  The metadata does not impose a narrower Python floor than the wheel tooling requires
  because the executable does not load the Python ABI.
- Wheels contain no Python wrapper, extension module, downloaded-on-first-run binary,
  source distribution, test fixture, or development dependency.
- The version is derived from the Cargo manifest and checked against the release plan;
  it is not duplicated as an independently edited literal.
- PyPI publishes only `urollup` in 0.1.0. A possible future `urollup-core` Python
  distribution remains unclaimed until it contains real, tested bindings; no placeholder
  package is published to reserve a name.

The wheel build uses an exact, reviewed Maturin version that has cleared the
repository’s 14-day dependency cool-off.
The root uv development lock remains separate from release package metadata.

### crates.io Packages

The release pins Cargo 1.90 or newer and uses its native workspace support for the
interdependent crates:

```bash
cargo package --locked --workspace
cargo publish --locked --workspace
```

Cargo verifies the full selected set and publishes `urollup-core` before its dependent
`urollup` automatically.
Repeated `-p` selectors are reserved for a rehearsed partial release; their command-line
order is not treated as a sequencing contract.
Workspace uploads are dependency-ordered but not atomic, so the partial-publication
recovery path remains required.
`cargo publish` creates, verifies and uploads its `.crate` in one supported operation;
the credential-free `cargo package` rehearsal proves the same packaging rules but is not
described as promotion of that earlier tarball.

Before publishing, the release gate:

- reviews `cargo package --list` for both crates;
- packages both crates together with `--locked`;
- extracts each `.crate` into an empty directory and builds or tests it outside the
  workspace;
- installs the extracted `urollup` package into a temporary root with a temporary Cargo
  patch that resolves `urollup-core` from its extracted sibling, then runs the packaged
  executable’s version and representative report smoke tests; and
- confirms package metadata, README, license, repository, minimum Rust version, and
  default features.

The first crates.io release uses one token with the shortest practical expiry, only the
`publish-new` endpoint scope, and exact future crate-name scopes for `urollup-core` and
`urollup`, because trusted publishing cannot be configured until each crate exists.
Immediately after 0.1.0, both crates get the exact repository, `release.yml` workflow
and `release` environment trusted publisher; trusted-publishing-only mode is enabled,
and the bootstrap token is revoked and removed.

### Workflow and Permissions

`.github/workflows/release.yml` has two entry paths:

- `workflow_dispatch` performs a complete rehearsal for an explicit commit and version,
  skipping only registry and GitHub Release writes.
- a `v*` tag performs the same plan, gate, build, and validation work before enabling
  publish jobs.

The workflow is thin orchestration over tested repository scripts and `make` targets.
Identity parsing, target mapping, archive construction, package-content checks, checksum
and manifest generation, registry-state comparison, and smoke-test selection do not live
as untested inline shell.

The workflow uses these authority boundaries:

1. The default workflow permission is `contents: read`; checkout never persists
   credentials.
2. Plan, gate, build, and validation jobs have no write or OIDC permission.
3. Publish jobs reference a protected `release` environment restricted to version tags
   and a required maintainer approval.
4. Only the PyPI, later crates.io, and attestation jobs receive `id-token: write`.
5. Only the GitHub Release and attestation jobs receive the narrowly required
   `contents: write` or `attestations: write` permission, respectively.
6. The first-release crates.io token exists only as a protected environment secret and
   is removed after trusted publishers are configured.

All third-party actions are pinned to reviewed commit SHAs.
Release tools and build images are exact inputs covered by the supply-chain policy.

### Build, Validate, and Publish Flow

The workflow stages and promotes artifacts in this order:

1. **Plan.** Resolve the immutable commit, version, tag, channel set, target matrix, and
   prior publication state into a machine-readable release plan.
2. **Gate.** Run `make check` on the release commit, verify a clean tracked tree, and
   verify that the commit is reachable from `main` and already exists on the remote.
3. **Package.** Build each archive and wheel once with locked inputs.
   Build jobs upload their artifacts to the workflow; they cannot publish them.
4. **Validate.** On each native host, download the packaged artifact into an empty
   environment, verify its contents and manifest entry, run `urollup --version`, and run
   a JSON report against a committed synthetic fixture using `--source` and
   `--no-default-sources` so no real user logs are read.
5. **Assemble.** Generate `SHA256SUMS`, the release manifest, and draft release notes
   from the validated artifact set.
   Attest the archives, wheels, checksum file, and manifest from the tagged workflow.
6. **Approve.** Enter the protected `release` environment only after every required
   build and validation job passes.
7. **Stage GitHub Release.** Create a draft for the tag and attach the already validated
   archives, wheels, checksums, manifest, and notes.
   Existing assets are accepted only when their digests match.
8. **Publish crates.io.** Run the pinned Cargo 1.90-or-newer workspace publication,
   which orders `urollup-core` before `urollup`, with `--no-verify`: the token-free
   `crates` job already verified the full set, and the publish job proves this commit
   packages to those exact digests, without the token, before it uploads (see the
   [Phase 1 notes](#phase-1-implementation-notes)). On a rerun, accept an existing
   version only when its registry checksum matches the planned package; treat a publish
   polling timeout as unknown until the registry is queried.
9. **Publish PyPI.** Upload the already validated wheels through the pending trusted
   publisher. On a rerun, compare every expected filename and PyPI hash before skipping.
10. **Publish GitHub Release.** Make the staged release public only after both
    registries verify.
11. **Verify.** Run clean, registry-backed installation probes, write and attest
    `release-evidence.json`, and attach it to the release.
    The workflow reports success only when all required channels and probes succeed.

Registry publication cannot be rolled back as a transaction.
Staging GitHub as a draft keeps the release announcement private until both registries
succeed. If a registry publish succeeds and a later channel fails, the run summary
records the exact completed channel and leaves the GitHub release as a draft until an
identical rerun finishes.
The final evidence record is published only when every required channel and probe is
complete.

### Release Notes and User Documentation

`CHANGELOG.md` and the GitHub release describe the user-visible state of 0.1.0, not the
sequence of development commits.
Because this is the first release, notes summarize features, supported providers and
commands, privacy behavior, supported platforms, installation commands, and known
limitations. Defects introduced and fixed before 0.1.0 are not listed as shipped fixes.

Before tagging, `README.md` documents these exact paths:

```bash
# Run an exact version without a persistent installation.
uvx --isolated urollup@0.1.0 --version

# Install the prebuilt executable persistently through uv.
uv tool install urollup==0.1.0

# Build and install from crates.io with the committed dependency resolution.
cargo install --locked urollup@0.1.0
```

The README also links the GitHub archive verification steps and states that the PyPI
package contains a Rust binary rather than a Python reimplementation.

### Failure and Recovery Contract

- **Before any channel publishes:** fix the release machinery or candidate, create a new
  commit, rerun the rehearsal, and move the tag only if it has never been published or
  consumed. Once any channel publishes, the tag and version are immutable.
- **One channel published:** preserve the failed run and artifacts, diagnose the exact
  channel, and rerun the same version.
  Identical existing files are skipped; missing files are published; any checksum
  conflict stops the run.
- **Published artifact is defective:** yank the crates.io and PyPI versions where
  appropriate, mark the GitHub release with a prominent notice, preserve the artifacts
  and evidence, and publish a corrected patch version.
  Never upload replacement bytes as 0.1.0.
- **Credential or workflow compromise:** disable the release environment, revoke the
  crates.io bootstrap token or registry publisher, remove GitHub environment access,
  identify affected workflow runs and commits from the manifests and attestations, and
  publish a security notice when users may have consumed an affected artifact.
- **Unsupported target only:** a required target failure blocks the release.
  It is not removed from the matrix during the run; changing supported targets requires
  a reviewed plan and documentation change.

## Implementation Plan

### Phase 1: Build and Rehearse the Release System

- [x] Add tested release-plan and artifact-validation scripts for version identity,
  target mapping, archive naming and contents, registry-state comparison, checksums,
  manifests, and installed-artifact smoke tests.
- [x] Add local `make` targets for a single-host release rehearsal and for the full
  no-publish validation path used by CI.
- [x] Add `crates/urollup/pyproject.toml` for a Maturin `bin` wheel without changing the
  root development-only `pyproject.toml`; pin Maturin under the supply-chain policy.
- [x] Build the five GitHub archive targets and five PyPI wheel targets, enforcing the
  macOS 11.0 deployment floor and confirming the manylinux 2.17 tag on both Linux
  wheels.
- [x] Smoke-test every archive and wheel on a matching native host, including
  `--version` and an explicit-source JSON report over a synthetic fixture.
- [x] Pin Cargo 1.90 or newer; package both Cargo crates with
  `cargo package --locked --workspace`, inspect their contents, build outside the
  workspace, and smoke-test an install from the packaged source.
- [x] Add `release.yml` with dispatch rehearsal and tag publication paths, SHA-pinned
  actions, read-only defaults, per-job permissions, version concurrency, artifact
  retention, and the protected `release` environment boundary.
- [x] Prove the release gates fail for a tag/version mismatch, missing target, malformed
  archive, missing or wrong wheel script payload or `RECORD` entry, empty artifact set,
  checksum conflict, and partial registry state.
- [ ] Create the PyPI pending trusted publisher for project `urollup`, workflow
  `release.yml`, and environment `release`; configure the GitHub environment and prepare
  a shortest-expiry crates.io token with only `publish-new` and exact `urollup-core` and
  `urollup` crate scopes, without exposing it to build jobs.
- [ ] Run a full dispatch rehearsal from the intended 0.1.0 commit and retain its
  release plan, artifacts, smoke-test results, checksums, and manifest for review.

The pending setup item is the maintainer’s; agents do not configure credentials,
publishers, environments or tags.
The rehearsal item waits for the accepted 0.1.0 commit on `main`; the PR’s own rehearsal
ran on its branch head with `allow-unmerged`.

### Phase 1 Implementation Notes

These choices were made while implementing Phase 1 and are open to maintainer review.
The [publishing runbook](../../../publishing.md) describes the resulting procedure.

- **Tag-checked SemVer lives in the release plan, not a build script.** `uro-co0m`
  proposed a `build.rs` tag check.
  `scripts/release-identity.mjs` instead refuses any mismatch among the tag, the
  workspace version, the `urollup-core` requirement, the PyPI metadata (which must
  derive its version from Cargo), the wheel `METADATA`, the artifact names and
  `urollup --version`, so the published crates carry no build script that every
  `cargo install` would run.
  Prereleases are limited to `alpha.N`, `beta.N` and `rc.N`, which have one PEP 440
  spelling, and build metadata is refused.
- **A dispatch names its commit as an assertion.** The commit input must be the head of
  the dispatched ref, so the release machinery that runs is the candidate’s own; the
  `allow-unmerged` input lets a pull-request branch rehearse, and such a run records
  that its commit is not releasable.
- **The gate runs beside the builds.** `make check` runs in parallel with the read-only
  build matrix; `assemble`, and so every write, waits for both.
- **Static musl archives** compile zstd’s C sources with Ubuntu noble `musl-tools`
  1.2.4-2, installed at that exact version and verified by the supply-chain gate.
- **Genuine manylinux wheels** are built inside the PyPA `manylinux2014` images pinned
  by digest, with the host’s pinned Rust toolchain mounted read-only, the locked Maturin
  1.15.0, Cargo offline against prefetched locked crates, and `--auditwheel check`.
  Validation reads each executable’s symbol versions and refuses anything above
  `GLIBC_2.17` or outside the manylinux2014 library policy.
- **Windows links the C runtime statically** (`+crt-static`) and asks the linker for a
  reproducible header (`/Brepro`); validation refuses a dynamic CRT import.
- **Archives are written by `scripts/release-archive.mjs`**, not the runner’s tar or
  zip, so their bytes depend only on their contents and the commit time; Cargo-home
  paths are remapped out of the executables.
  Maturin’s default SBOM is disabled because it embeds the build path, a random serial
  number and the build time.
- **PyPI uploads use the pinned uv** (`uv publish --trusted-publishing always
  --check-url`) rather than another publishing action.
- **Every publish job enters the `release` environment separately**, so each needs its
  own approval; attestation runs before approval, as the build-flow order requires.
- **The first release’s notes come from `CHANGELOG.md`**: a tag publication refuses a
  section still headed Unreleased, and the first release refuses a `Fixed` section.
- **The crates.io rehearsal found a packaging defect**: an unanchored `README.md`
  include pattern packaged every fixture README into `urollup-core`. The include
  patterns are now anchored.
  The `urollup` package no longer ships `tests/cli_process.rs`: its fixtures live in
  `urollup-core`’s `tests/`, which no package ships, so the test could only fail there.
  Both crates’ unit tests read those fixtures too, so the extracted packages are built
  and installed, not tested; the repository runs every test.
- **Concurrency is per version and per path.** A tag publication holds
  `release-publish-<tag>`; rehearsals use `release-rehearsal-v<version>`, because GitHub
  keeps one pending run per group and would otherwise let a later rehearsal cancel a
  pending publication.
- **A rehearsal stops on unknown registry state**, as a publication does, so it always
  shows the exact channel actions; GitHub drafts are visible only to the write token, so
  `stage-github` repeats the GitHub decision with drafts included.
- **Evidence is attached after the release is public**, as the plan orders, so GitHub’s
  immutable releases must stay off until evidence is published another way.
- **No build code runs beside a publishing credential.** Cargo hands its environment to
  every build script and proc macro, so `cargo publish` verification with the token set
  would expose it to the whole dependency tree (review A1). The workflow passes the
  secret as `RELEASE_CRATES_TOKEN`, never under Cargo’s name; `publish-crates` removes
  it from its own environment, runs `cargo package --no-verify` without it to prove the
  rehearsed digests, and hands it only to `cargo publish --no-verify`. A real-Cargo test
  shows `--no-verify` runs no build script and that verification does, and a gate probe
  proves `make release-check` fails if the publish command verifies.
  The workflow gate allows the secret in exactly one place, `RELEASE_CRATES_TOKEN` on
  the publish step, and refuses it in a job-level env, an action’s env or input, under
  another name or in another job (review C2). The Phase 2 trusted-publishing change
  keeps this rule.
- **A tag run checks its own preconditions.** `plan` peels the remote tag explicitly
  rather than trusting `GITHUB_SHA` and requires an annotated tag naming the checked-out
  commit. It verifies that exact tag object (the one `ls-remote` reported, on the release
  commit, made under the release tag’s own name) and its SSH signature against
  `.github/release-allowed-signers`, so a signed tag made for something else cannot be
  re-pointed as a release (review B1). After the first comparison every check names the
  object ID, never the tag name, so a stray ref such as `refs/<tag>` cannot shadow it
  (review C1). These checks cannot tell an older tag the maintainer signed for the same
  name and version from the one just pushed, so the plan summary prints the tag object
  and its time, and the approver compares them with the tag they signed (review C6). It
  refuses to start unless the `release` environment (a required reviewer, a `v*`
  tag-only deployment policy, no administrator bypass where GitHub reports it) and an
  active tag ruleset covering `refs/tags/v*`, with nothing excluded and no visible
  bypass actor, blocking updates and deletions exist.
  The job’s read-only token reads these settings, with an anonymous read as the fallback
  if GitHub refuses the token; the summary records which credential served each read,
  and a spent API quota is reported as a rate limit to rerun (review C7). GitHub hides a
  ruleset’s bypass list from non-administrators, so the maintainer confirms it at setup,
  and the runbook asks for two rulesets because one bypass list covers every rule in a
  ruleset (review B2). Whether the job token reads an existing environment has not run
  yet: the rehearsal before the first tag must show the job token serving that read, and
  if it shows the fallback, the maintainer decides before tagging (reviews B7 and C7). A
  rehearsal reports every setting without refusing.
  Every write then re-peels the tag and stops if it moved.
- **The maintainer checks a tag before pushing it.** `make release-verify-tag` runs the
  same binding and signature checks against the local tag, the tagged commit’s
  allowed-signers file and workspace version, and `main`, because a pushed release tag
  cannot be moved (review B6). It prints the tag object and its time, and reads the tag
  rulesets’ bypass lists with the maintainer’s own `gh` login, refusing when the only
  ruleset that blocks updates and deletions has a bypass actor (review C9).
- **The rehearsal exercises the publish path’s checks on real bytes** (review B8):
  `assemble` loads the finished bundle with the publish jobs’ own loader.
  The `crates` job packages with the first release’s publish selection, derived from the
  publish job’s own decision and command functions (`fullPublicationCommand`: an
  all-absent crates.io gives `--workspace`), and requires the verified bytes (review
  C4). `publish-crates` packages with exactly the selection it uploads, `--workspace` or
  `-p urollup` for a resumed publication, and refuses unless Cargo packaged exactly
  those crates and each matches its digest; an empty or unreadable selection is refused
  (review C5). Whether `cargo package -p urollup` reproduces the bytes once
  `urollup-core` is on crates.io can only run in that partial state; the digest check
  refuses the upload if it does not.
- **Every archive and wheel member carries the commit time**, which the build and the
  native validation both check (review B5). Validation fails rather than skip the check
  when `SOURCE_DATE_EPOCH` is missing, and records the time it checked against (review
  C8).
- **Signed tags are a technical control on who can release.** The approval boundary
  cannot tell the maintainer from an agent holding the maintainer’s credentials, so
  AGENTS.md reserves tags, approvals and release settings to the maintainer, and a tag
  must carry an SSH signature from a key in the committed allowed-signers file.
- **GitHub API assumptions**, both failing closed if wrong: an uploaded release asset
  reports its SHA-256 `digest` once GitHub has processed it (`stage-github` re-reads for
  a while before calling a missing digest a conflict), and the tag peeling above
  replaces any assumption about `GITHUB_SHA` for annotated tags.
- **Attestations are verified against this release only**: `--signer-workflow`,
  `--source-ref`, `--source-digest` and `--deny-self-hosted-runners` in the probes, the
  notes and the runbook.
  The `.crate` files are attested with the archives and wheels.
- **Release files come from named artifacts.** `assemble`, `evidence` and
  `attach-evidence` download only the artifacts they need, by name or pattern, into
  separate directories and read fixed paths; a bundle must agree with itself (manifest,
  `SHA256SUMS`, files, `.crate` files and plan) before anything is promoted; and an
  existing draft’s name, notes, prerelease flag and target are reset to the plan.
- **The gate job keeps its read token for all of `make check`.** Only the supply-chain
  step needs it, but `make check` runs that step itself and would fall back to anonymous
  GitHub API limits without it; splitting `make check` would duplicate its target list
  in a second place that can drift, for a token that can only read a public repository.
  The probes, by contrast, give their read token only to `gh attestation verify`.
- **Wheels are stamped with the commit time on every platform**: the manylinux container
  now receives `SOURCE_DATE_EPOCH`, as the macOS and Windows builds do.
- **A musl and glibc benchmark** runs in every rehearsal as an informational job, as
  evidence for the global-allocator decision `uro-uj8w` asks for; it does not gate.

### Phase 2: Publish and Verify 0.1.0

- [ ] Confirm every 0.1 milestone acceptance gate required by the main plan, run
  `make check`, and confirm the release commit is clean, pushed, and on `main`.
- [ ] Finalize Cargo metadata, `README.md`, `CHANGELOG.md`, release notes, known
  limitations, and the publishing and incident runbook.
- [ ] Create and push the signed, annotated tag `v0.1.0` on the reviewed release commit;
  review the workflow’s plan and validated artifacts before approving the `release`
  environment.
- [ ] Publish and verify crates.io, PyPI, and GitHub Release through the workflow
  without rebuilding validated channel artifacts.
- [ ] On clean target environments, download and verify a GitHub archive, run
  exact-version `uvx`, install with `uv tool install`, and install with
  `cargo install --locked`; each path must run `--version` and the representative
  fixture report.
- [ ] Configure trusted publishing and trusted-publishing-only mode for both crates.io
  crates, revoke the first-publish token, and verify the PyPI publisher is no longer
  pending.
- [ ] Update the repository’s agent skill fallback to the published version and reviewed
  per-target GitHub archive digests, then run its cloud or sandbox acquisition smoke
  test.
- [ ] Attach `release-evidence.json` to the release record and close the linked release
  beads only after every required channel and post-publish probe passes.

## Testing Strategy

### Tested Release Logic

Repository tests cover the logic outside GitHub Actions:

- valid and invalid SemVer tags, prerelease tags, and tag/Cargo/version-output drift;
- exact target-to-runner, archive, wheel-platform, and executable-name mapping;
- deterministic archive membership and rejection of extra, missing, or nested files;
- release manifest completeness, unique artifact names, digests, and non-empty output;
- registry states: absent, identical complete, identical partial, conflicting, and
  unavailable;
- rerun planning and the rule that a checksum conflict never counts as success;
- first-release notes that contain no fictitious fixes from unreleased development; and
- success and known-failure probes for every new release gate.

### Packaged-Artifact Tests

Each native matrix job starts from the packaged artifact, not `target/release/urollup`:

- extract the archive or install the wheel into an empty temporary location;
- for a wheel, inspect its script payload and `RECORD` before installation;
- keep the source tree and any previously installed `urollup` off `PATH`;
- verify the expected executable appears on `PATH` and resolves to the binary installed
  from that archive or wheel;
- assert `urollup --version` reports exactly `0.1.0` and associate that installed
  artifact with the release commit through its verified manifest entry;
- run `urollup report --source <synthetic-fixture> --no-default-sources --format json
  --timezone UTC` and compare its reconciled result with the committed expectation;
- verify stdout, stderr, and exit status; and
- inspect dynamic dependencies to enforce the documented libc and macOS floors.

Wheel tests use a local file with no index access before publication, including both the
ephemeral and persistent uv paths.
Post-publication tests repeat them against PyPI:

```bash
uv tool run --isolated --no-index --from ./dist/urollup-0.1.0-*.whl urollup --version
uvx --isolated urollup@0.1.0 --version
uv tool install urollup==0.1.0
```

The test harness uses isolated uv tool and binary directories so a developer’s existing
installation or cache cannot make a missing wheel executable appear to pass.

### Rehearsal Acceptance

The no-publish workflow is accepted only when it:

- runs the same plan, gate, package, smoke, checksum, manifest, notes, and attestation
  preparation as the tag path;
- produces the complete required matrix and fails when any entry is absent;
- proves build jobs lack registry, OIDC, attestation, and contents-write authority;
- exposes the exact channel actions it would take, including prior-version handling;
- retains downloadable artifacts long enough for human inspection; and
- performs no GitHub Release or registry mutation.

### Post-Publish Validation

The release is complete only when:

- all expected GitHub assets download, match `SHA256SUMS`, and pass
  `gh attestation verify` against `jlevy/urollup`;
- crates.io reports `urollup-core` and `urollup` 0.1.0 with the expected checksums and
  ownership, and docs.rs can build `urollup-core`;
- PyPI reports exactly the expected wheel filenames and hashes, with no sdist;
- exact-version `uvx`, persistent `uv tool install`, direct archive execution, and
  `cargo install --locked` all pass their representative command;
- GitHub release notes and `CHANGELOG.md` identify the same version, supported targets,
  install commands, contracts, and limitations; and
- `release-evidence.json` records every channel as complete and the bootstrap crates.io
  token has been revoked.

## Rollout Plan

1. Land the release machinery without credentials and make its rehearsal a required
   pre-release gate.
2. Configure the protected GitHub environment and registry publishers through their
   administrative interfaces.
   Record only non-secret identifiers in the repository.
3. Rehearse the exact 0.1.0 candidate and review the packaged outputs.
4. Merge the candidate to `main`, tag its immutable commit, and let the tag-triggered
   workflow rebuild, validate, await approval, publish, and verify.
5. Keep the GitHub release in draft state until both registries publish and verify.
6. After public release verification, configure crates.io trusted publishing, revoke the
   bootstrap token, update agent-facing pinned fallbacks, and retain the evidence
   record.

No release announcement or global success result is emitted while any required channel
or installation probe is incomplete.

## Resolved Decisions

- The first public alpha is `0.1.0` and follows milestone 0.1 acceptance.
  Later product milestones do not accumulate into this release.
- macOS 11.0 is the minimum for both Intel and Apple-silicon archives and wheels, and
  the final binaries are inspected to enforce it.
- Cargo 1.90 or newer owns dependency-ordered workspace packaging and publication.
- The PyPI convenience package remains a native Maturin `bin` wheel.
  Future importable PyO3 bindings are a separate product surface and release decision.

## References

- [Main urollup implementation plan](plan-2026-09-13-urollup-cli-and-web.md)
- [urollup design, Decision 27](../../../urollup-design.md#decision-27-release-scope)
- [Rust CLI engineering baseline](../../research/research-2026-09-13-rust-cli-engineering-baseline.md)
- [tbd pull request 302: aligned CLI, packaging, and Rust guidance](https://github.com/jlevy/tbd/pull/302)
- [Cargo `install`](https://doc.rust-lang.org/cargo/commands/cargo-install.html)
- [Cargo `package`](https://doc.rust-lang.org/cargo/commands/cargo-package.html)
- [Cargo `publish`](https://doc.rust-lang.org/cargo/commands/cargo-publish.html)
- [Rust 1.90 release notes](https://blog.rust-lang.org/2025/09/18/Rust-1.90.0/)
- [uv tool execution and installation](https://docs.astral.sh/uv/concepts/tools/)
- [Maturin `bin` bindings](https://www.maturin.rs/bindings.html#bin)
- [Maturin distribution and wheel compatibility](https://www.maturin.rs/distribution.html)
- [crates.io trusted publishing](https://crates.io/docs/trusted-publishing)
- [PyPI pending trusted publishers](https://docs.pypi.org/trusted-publishers/creating-a-project-through-oidc/)
- [GitHub deployment environments](https://docs.github.com/en/actions/reference/workflows-and-actions/deployments-and-environments)
- [GitHub artifact attestations](https://docs.github.com/en/actions/concepts/security/artifact-attestations)

<!-- This document follows common-doc-guidelines.md.
See github.com/jlevy/practical-prose and review guidelines before editing.
-->
