---
title: Rust CLI Engineering Baseline
description: Practice-by-practice comparison of the tbd Rust guidelines, fdu and flowmark-rs, with the repository, gate, release and dev-tooling baseline adopted for urollup.
date: 2026-09-13
author: Joshua Levy (github.com/jlevy) with LLM assistance
status: Complete for scaffolding; the decisions marked under Recommendations were confirmed on 2026-09-13 and 2026-09-14 as urollup design Decisions 2, 14, 23 and 27
---
# Research: Rust CLI Engineering Baseline

## Overview

urollup is a new Rust CLI with a local read-only web UI and an agent skill that must
also run in Claude Code Cloud sandboxes.
Before scaffolding, the project needs one stated engineering baseline: repository shape,
toolchain, lint floor, CLI process conventions, tests, CI gates, release channels,
supply-chain policy and dev-only tooling.
The goal is a baseline at least as strong as two existing repositories by the same
maintainer, [fdu](https://github.com/jlevy/fdu) and
[flowmark-rs](https://github.com/jlevy/flowmark-rs), and consistent with the tbd Rust
guidelines.
This brief compares the three sources practice by practice and recommends the
baseline that the [urollup design](../../urollup-design.md#82-engineering-conventions)
adopts.

## Questions to Answer

1. Which package shape, toolchain pin and lint floor should urollup start with?
2. Which stream, exit-status and terminal conventions fit its report contracts?
3. Which test layers and CI gates prove those contracts?
4. Which targets, release channels and verification steps serve local users and cloud
   sandboxes, including sandboxes with restricted network egress?
5. Where does the dev-only softschema toolchain sit, and how is schema drift gated?
6. What cool-off policy applies, and how are first-party packages exempted?

## Scope

Included: repository layout, Cargo configuration, lint and format policy, CLI process
conventions, testing layers, CI gates, release and distribution, supply chain, dev
tooling, agent skill packaging and agent-facing docs.

Excluded: accounting design, the bundle and summary formats, the exact contract check
commands, pricing, web security and performance targets.
The plan owned those when this brief was written.
*(Updated 2026-09-15: the [urollup design](../../urollup-design.md) now owns them,
except performance targets, which stay in the plan.)*

Sources, read on 2026-09-13:

- tbd guidelines `rust-project-setup`, `rust-cli-rules`, `rust-lint-format-rules`,
  `rust-testing-rules`, `rust-release-rules`, `rust-filesystem-rules`, `rust-rules`,
  `rust-code-review-rules`, `supply-chain-hardening`, `cli-agent-skill-patterns`,
  `golden-testing-guidelines` and `ci-and-gates-rules`
- fdu at commit `afbb2ee`: a 35k-line workspace of engine, CLI and PyO3 bindings,
  version 0.1.0, with a release rehearsal workflow but no publication yet
- flowmark-rs at commit `f1e9337`: a single published package, version 0.4.0, released
  to crates.io, PyPI, GitHub Releases and a Homebrew tap
- softschema checkout at `882008c` and PyPI release 0.8.1, uploaded 2026-09-11

## Findings

### How the Two Repositories Differ

fdu is the stronger engineering reference.
The lint floor in `rust-lint-format-rules` was measured against fdu, and fdu adds a
provenance-checking supply-chain gate, feature-boundary jobs, a drift-checked evidence
record and a single `make check` handoff gate.
It has not published a release.

flowmark-rs is the only one of the two with a working publication path: tag or dispatch
release orchestration, six-target static archives, crates.io and PyPI trusted
publishing, checksums and a Homebrew tap.
Its gates are weaker: an unpinned `stable` toolchain, an MSRV job that only compiles, a
non-blocking Markdown check, no local verify command and advisory ignores without
removal conditions.

The two repositories share the same gaps relative to the guidelines: neither uses
`insta` or `proptest`, neither runs a strict cross-target lint in CI, neither checks the
effective lint policy with a violation probe, neither formats TOML, and neither
publishes artifact attestations.

### Repository and Toolchain

| Practice | Guidelines | fdu | flowmark-rs | urollup baseline |
| --- | --- | --- | --- | --- |
| Package shape | Smallest shape; workspace when dependency sets or public APIs differ | Virtual workspace; `crates/fdu` depends on `crates/fdu-core` as an ordinary dependency, so the CLI cannot reach engine internals (`fdu/crates/fdu/Cargo.toml`) | One package, library plus two binaries behind a `cli` feature (`flowmark-rs/Cargo.toml`) | fdu: `crates/urollup-core` library and `crates/urollup` binary, because the HTTP server and web bundle are a different dependency set from accounting |
| Edition and MSRV | Edition 2024; declare `rust-version`; raising it is a compatibility decision | 2024, 1.85, resolver 3, shared `[workspace.package]` (`fdu/Cargo.toml`) | 2024, 1.85 | 2024, resolver 3, `rust-version = "1.85"` until a dependency forces a recorded raise |
| Toolchain pin | Exact `rust-toolchain.toml` with clippy and rustfmt; separate from MSRV | 1.97.1 in `fdu/rust-toolchain.toml` and every CI job | None; CI uses `stable` | fdu |
| Release profile | Deliberate, commented; symbol-bearing profiling profile | LTO, one codegen unit, strip, unwinding kept for PyO3, `profile.profiling`, optimized dev dependencies | LTO, one codegen unit, strip, `panic = "abort"` | fdu’s profile; keep unwinding so a panicking `serve` handler does not end the process |
| Line endings | Explicit `.gitattributes` | LF for JSON and goldens only | Global `* text=auto eol=lf` with reason (`flowmark-rs/.gitattributes`) | flowmark-rs: embedded skill text, JSONL fixtures and goldens must be byte-identical on Windows |
| Version stamp | One release identity | `build.rs` stamps revision and dirty marker; asserts `FDU_RELEASE_TAG` equals `v{version}` (`fdu/crates/fdu/build.rs`) | `build.rs` embeds tag, commits ahead and hash (`flowmark-rs/build.rs`) | fdu’s smaller stamp with the exact tag assertion |
| Verify command | One entry point, default `justfile`; another runner when bootstrap or convention justifies it | `make check`, about 30 prerequisites; `make fix` separate (`fdu/Makefile`) | Makefile formats only; no local gate | `make check` and `make fix` (see Recommendations) |

### Lints and Formatting

| Practice | Guidelines | fdu | flowmark-rs | urollup baseline |
| --- | --- | --- | --- | --- |
| Lint floor | Pedantic, `missing_docs`, `unsafe_code`, `warnings` and `unwrap_used` denied; every member opts in | Exactly the floor in `[workspace.lints]`; `fdu-py` restates it because PyO3 macros emit `unsafe impl` | Same, without `missing_docs` | fdu’s floor plus lints that cost nothing in new code (see Recommendations) |
| `clippy.toml` | `allow-unwrap-in-tests`, `allow-expect-in-tests` | Absent | Absent | Add |
| Unsafe | `deny`, not `forbid`; scoped `#[expect]` with a safety argument | One macOS reader behind `cfg` | `unsafe` SIGPIPE reset in `flowmark-rs/src/main.rs` | None expected |
| Formatting | rustfmt, taplo for TOML, flowmark for Markdown | rustfmt; flowmark-rs pinned in a uv group as a blocking check (`fdu/explorations/benchmarks/pyproject.toml`) | rustfmt; `uvx flowmark@0.7.3` with `continue-on-error: true` | fdu’s lockfile-pinned blocking check, plus `taplo fmt --check` |
| Platform-gated code | Strict cross-target clippy in CI; local runs may skip | `make cross-lint` skips missing targets; CI lints Linux only | None | Strict CI job for macOS and Windows targets |
| Proof the floor is live | `cargo metadata` lint-policy check and a violation probe | Manual | Manual | Automated in `make check` |

### CLI Process Conventions

| Practice | Guidelines | fdu | flowmark-rs | urollup baseline |
| --- | --- | --- | --- | --- |
| Parser | clap derive; help, version and usage errors are tested interfaces | clap derive; help text pinned by golden | clap derive | Same |
| Streams | Data to stdout, diagnostics to stderr, buffered, flush errors propagated | `run_with_io` takes separate output and diagnostic writers (`fdu/crates/fdu/src/cli.rs`) | `println!` and `eprintln!` | fdu’s injected writers, so stream behavior is unit-testable |
| Color and TTY | `IsTerminal`, `NO_COLOR`, `--color` | `--color auto\|always\|never`, `NO_COLOR`, `FORCE_COLOR`, never for machine formats, one style table | No color policy | fdu |
| Exit status | Return `ExitCode`; stable documented classes; partial is failure unless the contract says otherwise | `ExitCode`; 0 complete, 1 error, 2 usage error or partial scan unless `--allow-partial` (`fdu/crates/fdu/tests/cli_exit.rs`) | `ExitCode` in `main`, but 14 `std::process::exit` calls; 2 for usage and invalid UTF-8 | `ExitCode` only, one code per failure class (see Recommendations) |
| Broken pipe | Classify only at the stdout renderer, only after required work succeeded | Renderer classification with unit tests | Restores default SIGPIPE | fdu |
| Errors | `thiserror` in libraries, `anyhow` at the binary | Both, with rendered cause chains | Both | Same |
| Logging | Facade; libraries set no subscriber; verbosity never changes results | None; `FDU_COUNTERS=1` prints counters to stderr | None | `tracing` in core; the binary installs a stderr subscriber |
| Machine output | One documented schema per mode | `--format text\|json\|jsonl\|yaml`; JSON carries `"schema": "fdu.report/4"` | Text only | Versioned schema identifiers independent of the package version |
| Output files | Atomic publication through a named helper | `tempfile` staging | `NamedTempFile::persist` in `flowmark-rs/src/lib.rs` | One `write_atomic` boundary for reports and bundles |

### Testing

| Practice | Guidelines | fdu | flowmark-rs | urollup baseline |
| --- | --- | --- | --- | --- |
| Placement | Unit tests inline; integration tests in the member’s `tests/` | Same; binary tests use `CARGO_BIN_EXE_fdu` | Root `tests/` of the single package | Same as fdu |
| CLI sessions | tryscript for console goldens | Nine tryscript files run by `fdu/scripts/run-golden.mjs` against an explicit binary path; tryscript 0.2.1 locked in `package-lock.json`; portability and observability checks | Rust harness running a globally installed tryscript 0.1.7 (`flowmark-rs/tests/test_tryscript_golden.rs`) | fdu |
| Structured snapshots | `insta` by default | Hand-written `.golden` session files (`fdu/crates/fdu-core/src/opened/golden_support.rs`) | None | `insta` for ledger, reconciliation and report-data snapshots |
| Properties | `proptest` by default | Reference model and serial/parallel equivalence tests (`fdu/crates/fdu-core/tests/reference_model.rs`) | Fixed-seed generator to avoid a dependency (`flowmark-rs/tests/test_preservation_properties.rs`) | `proptest` for merge laws and traversal-order invariance, plus fdu-style equivalence tests |
| Parity ledger | Known divergences explicit | Python-surface deviations re-recorded by Linux CI | Known-divergence TOML that fails on unlisted and stale entries | flowmark-rs’s stale-entry rule for the ccusage and agentfdr feature matrix |
| Coverage | `cargo-llvm-cov` as discovery | None | llvm-cov job, non-blocking upload | Non-blocking coverage job |
| Benchmarks | Recorded evidence; timing checks only when attributable, such as a within-run comparison against a fixed baseline | Python harness under `explorations/benchmarks/` with its own tests in `make check`; measurement loop outside CI; generated reports drift-checked | Shell scripts and `benchmarks/REPORT.md` | fdu’s tested harness, plus CI jobs comparing each pull request with its merge base in one job (see CI Gates) |

### CI Gates

| Practice | Guidelines | fdu | flowmark-rs | urollup baseline |
| --- | --- | --- | --- | --- |
| Workflow authority | Read-only default, `persist-credentials: false`, actions pinned by SHA | All three (`fdu/.github/workflows/ci.yml`) | SHA pins; `ci.yml` has no `permissions` block | fdu |
| Ordering | Fail fastest first | Supply-chain job gates every other job | Independent jobs | fdu |
| MSRV | Compile and test | `cargo check`, then tests for the core crate | `cargo check` only | Compile and test the workspace |
| Feature boundaries | Job for each consumer feature set, with a dependency guard | Core tested with each feature set; `cargo tree` guard captured before `grep` (`fdu/Makefile` `lib-only`) | `--no-default-features` test | Core dependency guard: no clap, anyhow, HTTP or async runtime |
| Platforms | Test where behavior differs | Linux, macOS and Windows | Same | Same plus `ubuntu-24.04-arm` |
| Caches | Performance-only | No reusable caches written by PR jobs, enforced by policy | `Swatinem/rust-cache` | fdu |
| Semver | `cargo-semver-checks` for libraries promising compatibility | None | PR job with one documented allow | Add for `urollup-core` after its first publication |
| Generated files | Regenerate in CI and fail on difference | Ledger, report page, experiment schema | README, skill mirror | Contract schema, web bundle, skill mirrors |

### Release and Distribution

| Practice | Guidelines | fdu | flowmark-rs | urollup baseline |
| --- | --- | --- | --- | --- |
| Orchestration | One release identity; dry run; least privilege; idempotent reruns; testable release logic | `release.yml` rehearsal with a tested plan script and evidence job; publication deliberately absent | `release.yml` with dry-run dispatch, tested plan script, reusable crates and PyPI workflows that skip published versions | flowmark-rs’s flow with fdu’s protected `release` environment and evidence job |
| Archives | Native runners where smoke testing matters | None yet | `{x86_64,aarch64}` × `apple-darwin`, `pc-windows-msvc`, static `unknown-linux-musl`; aarch64 Linux cross-built and not run | Five targets, each built and run natively (see Recommendations) |
| crates.io | Trusted publishing; interdependent crates in one invocation | `cargo package --locked -p fdu-core -p fdu`, then installs the extracted crate | Trusted publishing via `rust-lang/crates-io-auth-action` | fdu’s packaging with flowmark-rs’s publishing |
| PyPI | Trusted publishing; smoke-test each wheel’s console command | abi3 extension wheels and a tested sdist | Binary wheels (`bindings = "bin"`) for manylinux 2_17, macOS and Windows | flowmark-rs’s binary wheel, without an sdist |
| Other channels | Explicit channel audiences | No Homebrew for 0.1.0 | Homebrew tap updated manually; README advertises `cargo binstall` without binstall metadata | Defer Homebrew, npm and binstall |
| Integrity | Checksums, provenance | `SHA256SUMS`, release manifest, CycloneDX SBOM check in wheels; attestations planned | `SHA256SUMS` on the GitHub Release | `SHA256SUMS` plus GitHub build-provenance attestations |

### Supply Chain and Dev Tooling

| Practice | Guidelines | fdu | flowmark-rs | urollup baseline |
| --- | --- | --- | --- | --- |
| Cool-off | 14-day floor; exceptions on record; no unpinned runners | `supply-chain-policy.json` with a tested validator for crate publication dates, npm integrity, PyPI hashes, action SHAs and toolchain manifests; first-party packages exempt by identity | Dependabot for Cargo and actions with no cooldown | fdu’s policy file and validator |
| uv | `UV_EXCLUDE_NEWER="14 days"` | Each `uv.toml` sets `exclude-newer = "14 days"` and `[exclude-newer-package] softschema = "2099-12-31"`; a preflight requires uv 0.12.1, since older uv cannot parse relative durations | `python/pyproject.toml` with ranged dependencies and no cool-off | Project `[tool.uv]` with exemptions and the same preflight *(Updated 2026-09-15: a root `uv.toml`, always passed as `uv --config-file uv.toml`)* |
| cargo-deny | Advisories, licenses, sources, bans; `unused-allowed-license = "allow"` | No ignores; commented policy (`fdu/deny.toml`) | Three advisory ignores without tracker or removal condition (`flowmark-rs/deny.toml`) | fdu; any ignore names a bead and a removal condition |
| npm | Scripts disabled, lockfile, audit | `.npmrc` `ignore-scripts` and `save-exact`; `npm ci --ignore-scripts`; `npm audit signatures` | `npm install -g tryscript@0.1.7` in CI | fdu |
| Schema tooling | Generated files have one owner and a drift check | `softschema==0.6.0` in a dev group; `make perf-schema-check` runs `softschema compile … --check` | None | softschema 0.8.1 with a drift gate (see Recommendations) |

### Agent Integration and Docs

| Practice | Guidelines | fdu | flowmark-rs | urollup baseline |
| --- | --- | --- | --- | --- |
| Skill | L1: local command first, exact-version fallback, never `latest` | `--skill` prints embedded `SKILL.md` with `uvx --from fdu==VERSION`; a unit test rejects `latest` | `--skill` and `--install-skill` for `.agents`, `.claude` and `AGENTS.md`, with a newer-format guard; `scripts/sync_skill_mirror.py --check` | fdu’s version-substituted text and test, flowmark-rs’s mirror drift check |
| `AGENTS.md` | Compact routes to skills and commands | Routes to architecture, gate, toolchain bootstrap and dependency policy (`fdu/AGENTS.md`) | Template placeholders | fdu’s shape |
| Docs set | README, license, changelog, security path, policies | README, CHANGELOG, SECURITY.md, SUPPLY-CHAIN-SECURITY.md, architecture docs | README, CHANGELOG, CONTRIBUTING, `docs/development.md`, `docs/publishing.md` runbook | Union of both |

## Key Insights

- **fdu’s gates plus flowmark-rs’s channels.** Neither repository alone is the target.
  urollup adopts fdu’s verification and supply-chain discipline and flowmark-rs’s proven
  publication path, then closes the gaps both share.
- **The shared gaps cost nothing in a new repository.** `insta`, `proptest`, strict
  cross-target lint, lint-policy proofs, taplo and attestations each carried migration
  cost in fdu or flowmark-rs; in urollup they are day-one configuration.
- **Embedded files must live inside the embedding crate.** `cargo package` ships only
  files under the package directory, so a compiled schema, web bundle or skill text read
  by `include_str!` from outside the crate builds in the repository and fails from
  crates.io. fdu’s `build.rs` comment records one variant of this trap; flowmark-rs keeps
  a bundled docs copy inside the package and checks it against the upstream file.
- **A shared exit code hides a failure class.** fdu returns 2 for both clap usage errors
  and partial scans, so automation cannot tell a bad invocation from missing coverage.
  The [design](../../urollup-design.md#65-exit-codes) separates runtime failure, invalid
  invocation, unmet coverage and exceeded thresholds, so urollup gives each its own
  code.
- **Pinned digests cannot come from the release they verify.** A `SHA256SUMS` downloaded
  beside an archive detects corruption, not a replaced release.
  fdu’s `supply-chain-policy.json` records reviewed per-asset digests for the GitHub CLI
  and uv; the urollup skill needs the same for its own archives.
  A binary cannot print digests of archives built after it, so those pins belong in the
  repository’s skill copy, written by a post-release step.
- **Static musl binaries trade portability for allocator risk.** They run in any Linux
  sandbox regardless of glibc, but musl’s default allocator can limit multithreaded
  parsing. Measure the release build before choosing a global allocator.

## Options Considered

### Option A: Two-Crate Workspace (fdu)

**Description:** `urollup-core` library and `urollup` binary in a virtual workspace.

**Pros:**
- The compiler prevents CLI or HTTP code from reaching core internals
- Core dependency tree excludes argument parsing, HTTP and async runtime crates
- Matches the plan’s split between a reusable core library and an executable

**Cons:**
- Two crates to publish in one invocation, in dependency order
- Version stamping and embedded assets need per-crate care

### Option B: Single Package with Feature Gates (flowmark-rs)

**Description:** One package, library plus binary, with `cli` and `serve` features.

**Pros:**
- One crate to publish; simpler first release

**Cons:**
- Boundary enforced by feature discipline rather than by the compiler
- Every CI feature combination must be tested to keep the library build honest

### Eliminated Options

- **cargo-dist:** Would generate release workflows and shell, npm and Homebrew
  installers. Neither sibling repository uses it, its shell installers are designed for
  `curl | sh`, and flowmark-rs’s tested release scripts already cover the chosen
  channels.
- **npm wrapper package:** No target audience needs Node to install urollup, and the
  wrapper adds a registry and a postinstall download path that the npm policy disables.

## Recommendations

Adopt Option A with the baseline below.
Items marked **Decision** need maintainer confirmation.

### Repository Layout

```text
urollup/
├── Cargo.toml                 # virtual workspace: package metadata, lints, profiles
├── Cargo.lock
├── rust-toolchain.toml  rustfmt.toml  clippy.toml  deny.toml  .gitattributes
├── Makefile                   # make check (verify) and make fix
├── crates/
│   ├── urollup-core/          # accounting library; embeds the compiled contract schema
│   └── urollup/               # binary: CLI, HTTP, embedded web bundle and skill text
├── contracts/                 # dev-only Pydantic models compiled by softschema
├── tests/golden/              # tryscript CLI sessions and fixtures
├── bench/
│   ├── generator/             # seeded corpus generator for bench-small and bench-1g
│   ├── harness/               # runs release binaries, writes one JSON record per run
│   ├── corpora/               # committed manifests; generated corpora are gitignored
│   └── results/               # reference-laptop records committed for each release
├── scripts/                   # tested gate and release programs
├── pyproject.toml  uv.toml  uv.lock   # dev-only uv project: softschema, flowmark-rs
├── package.json  package-lock.json  .npmrc   # dev-only Node tools
├── supply-chain-policy.json   SUPPLY-CHAIN-SECURITY.md  SECURITY.md  CHANGELOG.md
├── .agents/skills/  .claude/skills/
└── docs/
```

- Keep agent dialect adapters as modules of `urollup-core` until a real dependency or
  release boundary appears.
- Place every file Rust embeds inside the embedding crate: the compiled schema under
  `crates/urollup-core/`, and the built web bundle and skill text under
  `crates/urollup/`. A drift gate rebuilds each and fails on difference, so
  `cargo install --locked urollup` never needs Node, uv or Python.
- **Decision:** `make check` rather than the guideline’s default `justfile`. make is
  present on every CI runner and sandbox image, and fdu, flowmark-rs and softschema all
  use it. Recipes stay orchestration; decisions live in scripts with tests beside them.
- **Decision:** MIT license, matching fdu and flowmark-rs.

### Lint Floor

Declare once at the workspace root; every member sets `[lints] workspace = true`.

```toml
[workspace.lints.clippy]
pedantic = { level = "deny", priority = -1 }
missing_errors_doc = "allow"
missing_panics_doc = "allow"
module_name_repetitions = "allow"
must_use_candidate = "allow"
too_many_lines = "allow"
unwrap_used = "deny"
# Measured as low-cost additions in rust-lint-format-rules; free in new code.
let_underscore_future = "deny"
wildcard_enum_match_arm = "deny"

[workspace.lints.rust]
missing_docs = "deny"
unsafe_code = "deny"
warnings = "deny"
```

- Deny `clippy::panic` in `urollup-core`.
- **Decision:** Deny `clippy::arithmetic_side_effects` at module scope in token counter
  and money modules, which enforces the design’s checked arithmetic where it matters
  without applying a restriction lint to the whole workspace.
- Add `clippy.toml` with `allow-unwrap-in-tests` and `allow-expect-in-tests`.
- `rustfmt.toml`: `edition = "2024"`, `max_width = 100`, `use_small_heuristics = "Max"`
  (both repositories).
- Prove the floor in `make check`: a `cargo metadata` lint-policy check, a committed
  violation probe, and a strict cross-target clippy pass for `aarch64-apple-darwin` and
  `x86_64-pc-windows-msvc`.

### CLI Process Contract

- `main` only sets up the process, calls `run` with injected stdout and stderr writers,
  and returns `ExitCode`. No `std::process::exit`.
- stdout carries exactly the requested data format.
  Diagnostics, progress, warnings and logs go to stderr.
- Color and progress only on a terminal; `--color auto|always|never`; honor `NO_COLOR`;
  machine formats are never colored.
  No prompts and no pager.
- A closed stdout after complete work exits 0; any other write or flush error is a
  failure.
- `tracing` facade in `urollup-core` with no subscriber; the binary installs a stderr
  subscriber for `-v` and `serve` request logs.
- A JSON document is written only after the query completes.
  A streamed JSONL export ends with a completion record, so a consumer can detect a
  truncated stream. `--output` files and bundles are published atomically.
- Report, bundle, query and identity contracts carry their own version identifiers,
  independent of the package version.

| Exit | Meaning |
| --- | --- |
| 0 | Complete success, including non-strict results whose coverage fields report gaps |
| 1 | Runtime failure: unreadable requested source, I/O or write failure, capacity limit, internal error |
| 2 | Invalid invocation or request: usage error, invalid query, unsupported contract or identity version |
| 3 | A required coverage condition is unmet, such as a strict query or `--require-priced` |
| 4 | A `check` threshold is exceeded |
| 130 | Interrupted; no report or output file is published |

The first failing stage decides the code: request validation (2), execution (1),
coverage (3), then thresholds (4). **Decision:** this departs from fdu’s shared exit 2.

### Testing Layers

1. Unit tests inline, including stream and exit mapping through injected writers.
2. `insta` YAML snapshots of normalized ledgers and report data for small fixtures.
3. `proptest` for merge associativity, commutativity and idempotence, traversal-order
   invariance and counter reconciliation; fdu-style serial versus parallel and raw
   versus bundle equivalence tests.
4. tryscript CLI sessions under `tests/golden/`, invoked by explicit binary path, with
   tryscript locked in `package-lock.json` and a portability check that rejects machine
   paths in committed goldens.
5. Contract tests: golden outputs validate against the compiled schemas.
6. A feature-matrix ledger against pinned ccusage and agentfdr that fails on unlisted
   and stale divergences.
7. Packaged-artifact smoke tests on each release target.
8. Unit tests for the benchmark generator and harness in `make check`; the measurements
   themselves run in dedicated CI jobs and on the reference laptop.
9. Non-blocking coverage as discovery.

**Decision:** write the `bench/` generator and harness as Python standard-library
programs run through the root uv project, following fdu’s harness.
Python reads each child’s peak RSS through `os.wait4` without the `unsafe` block a Rust
harness would need, and uv is already a pinned dev toolchain.

### CI Gates

Jobs run with `permissions: contents: read`, `persist-credentials: false`, SHA-pinned
actions, the pinned toolchain, `--locked`, `RUSTFLAGS=-D warnings`, and no reusable
cache writes from pull requests.
Each maps to a `make` target with the same command.

1. Supply-chain provenance (all other jobs depend on it)
2. Format: `cargo fmt --check`, `taplo fmt --check`, flowmark `--auto --check`
3. Lint: workspace clippy with `--all-targets`, lint-policy check, violation probe,
   strict cross-target clippy
4. Test on Linux x86_64, Linux arm64, macOS and Windows
5. Core feature boundary and dependency guard
6. MSRV compile and test
7. Docs: `RUSTDOCFLAGS="-D warnings" cargo doc --locked --workspace --no-deps`
8. Dependency policy: `cargo deny --locked check`, `npm audit`, `npm audit signatures`
9. CLI goldens and portability check
10. Contracts: softschema drift check and golden-schema validation
11. Generated-file drift: web bundle and skill mirrors
12. Release-script, benchmark generator and harness unit tests
13. Non-blocking coverage; `cargo-semver-checks` for `urollup-core` after first
    publication
14. `bench-pr`: on every pull request, one `ubuntu-24.04` job builds the pull request
    and its merge base in release mode, generates `bench-small`, alternates their runs
    and applies the Testing Strategy regression policy
15. `bench-scheduled`: on a schedule and by manual dispatch, the same alternating
    comparison on `bench-1g`, between the head of `main` and the latest release tag
    (before the first release, the commit the previous scheduled run measured)

Benchmark jobs upload their JSON records as workflow artifacts and never commit.
Comparing both builds inside one job on one runner is what makes the result attributable
under `ci-and-gates-rules`; an absolute wall-time threshold on a shared runner would not
be. Run counts, corpus definitions and thresholds live in the plan’s Testing Strategy.

### Targets, Channels and Versioning

| Target | Runner | Artifacts | Primary users |
| --- | --- | --- | --- |
| `x86_64-unknown-linux-musl` | `ubuntu-24.04` | `.tar.gz` archive, manylinux x86_64 wheel | Cloud sandboxes, Linux hosts, CI |
| `aarch64-unknown-linux-musl` | `ubuntu-24.04-arm` | `.tar.gz` archive, manylinux aarch64 wheel | Arm64 sandboxes and hosts |
| `aarch64-apple-darwin` | `macos-15` | `.tar.gz` archive, wheel | Local macOS |
| `x86_64-apple-darwin` | `macos-15-intel` | `.tar.gz` archive, wheel | Local Intel macOS |
| `x86_64-pc-windows-msvc` | `windows-latest` | `.zip` archive, wheel | Local Windows |

- Linux archives are static musl builds, as in flowmark-rs, but built and smoke-tested
  on native runners. **Decision:** defer `aarch64-pc-windows-msvc`, which flowmark-rs
  ships.
- Channels, in dependency order: GitHub Release archives; crates.io `urollup-core` and
  `urollup` packaged in one invocation; PyPI binary wheels.
  All registry publishing uses trusted publishing from a protected `release`
  environment. **Decision:** defer Homebrew, npm and binstall.
  Confirm the `urollup` and `urollup-core` names are available on crates.io and PyPI
  before the first release.
  *(Checked 2026-09-13: both names were unregistered, though not reserved; see design
  [Decision 1](../../urollup-design.md#decision-1-product-name).)*
- Integrity: one `SHA256SUMS`, GitHub build-provenance attestations for every archive
  and wheel, the fdu evidence manifest, and wheel SBOM inspection.
  **Decision:** no separate GPG or minisign signature initially.
- Versioning: SemVer from `[workspace.package] version`; tag `vX.Y.Z` must equal it,
  asserted by `build.rs`; development builds report revision and dirty state.
  Before 1.0, a minor release may change CLI or JSON contracts, with contract version
  changes recorded in `CHANGELOG.md`.

### Sandbox Binary Acquisition

The skill resolves a binary in this order and records which path it used:

1. An installed `urollup` whose `--version` and supported contract versions are
   compatible.
2. The pinned GitHub Release archive for the detected target, verified against a SHA-256
   digest committed in the repository’s skill copy before extraction, then
   `urollup --version`. Where `gh` is available, `gh attestation verify` adds
   provenance.
3. `uv tool install urollup==X.Y.Z` from PyPI.
4. `cargo install --locked urollup@X.Y.Z` from crates.io, which also needs a Rust
   toolchain and build time.
5. When no registry or GitHub host is reachable: an environment setup step run with
   network access, or a verified archive supplied at a documented repository path,
   checked against the same digests.
6. Otherwise an explicit unsupported-environment result naming the hosts attempted.
   Never an unpinned runner, `latest`, a Git branch build, or numbers derived from the
   visible chat.

A post-release step, implemented as a tested script with a drift check, writes the
version and per-target digests into the repository skill copy.
The cloud smoke test records which hosts each supported environment actually reaches.

### Supply Chain and First-Party Exemptions

- Default: the tbd `supply-chain-hardening` 14-day cool-off for crates, npm packages,
  PyPI packages, GitHub Actions and toolchains.
  Cargo has no resolution-time gate, so the committed lockfile, `--locked` and fdu’s
  publication-date validator enforce it.
  If Dependabot is enabled, set its `cooldown` to 14 days.
- First-party packages are exempt from release age by identity, not version, and are
  recorded once under `firstParty` in `supply-chain-policy.json`: PyPI `softschema`,
  `flowmark-rs`, `flowmark` and `frontmatter-format`; npm `tryscript` and `get-tbd`. The
  exemption waives age only; lockfile hashes and integrity checks still apply.
- `SUPPLY-CHAIN-SECURITY.md` at the root, referenced from `AGENTS.md`.
- The user-level `~/.config/uv/uv.toml` uses a 7-day window with its own first-party
  list, but CI has no user configuration, so the project file carries both the window
  and the exemptions:

```toml
[project]
name = "urollup-dev"
version = "0.0.0"
requires-python = ">=3.12"

[dependency-groups]
# Also pytest for the contract tests, pinned to a release older than 14 days.
dev = ["softschema==0.8.1", "flowmark-rs==0.4.0"]

[tool.uv]
package = false
exclude-newer = "14 days"

[tool.uv.exclude-newer-package]
softschema = "2099-12-31"
flowmark-rs = "2099-12-31"
```

- *(Updated 2026-09-15: the cool-off and first-party exemptions now live in a root
  `uv.toml` that every command passes as `uv --config-file uv.toml`, so user-level uv
  configuration never changes resolution, and `pyproject.toml` keeps only
  `[tool.uv] package = false` beside the dev group above.)*
- Keep fdu’s `uv` version preflight so an old uv fails with a version message rather
  than a TOML date error.

### softschema Contract Toolchain

- **Authoring:** Pydantic contract models live in `contracts/` and run only through the
  root uv project: `uv --config-file uv.toml run --frozen softschema …`. softschema is a
  dev dependency, never a runtime or build dependency.
- **Output:** compiled schemas are committed under `crates/urollup-core/schemas/`, so
  `include_str!` works from a crates.io build.
  The plan decides how the core validates against them; any Rust validator crate goes
  through the same dependency review as other runtime dependencies.
  *(Decided 2026-09-15: typed serde structs with `deny_unknown_fields` validate every
  read and write, and the compiled JSON Schema runs only in `urollup validate` and
  tests, so the read path needs no validator crate; see
  [design §5.7](../../urollup-design.md#57-contract-authoring-and-validation).)*
- **Gate:** `make contracts-check` is part of `make check`, and a `contracts` CI job
  runs it with a SHA-pinned `setup-uv` at the pinned uv version and `uv sync --locked`.
  It fails when recompiling changes a committed schema and when a fixture or golden gets
  the wrong verdict. A committed stale-schema probe proves the gate fails.
  The plan owns the exact commands.
  *(Updated 2026-09-15: design §5.7 now holds them.)*
- **Gate program:** Put fixture loops and expected-failure checks in a tested script
  that `make contracts-check` calls, not in Makefile shell.
  An inline `! softschema validate "$f"` passes when uv cannot find softschema or a glob
  matches nothing, so the script must require a non-empty fixture list and distinguish a
  validation failure from a tool failure.
- **No Python at runtime:** no `build.rs` invokes uv or Python; release smoke tests run
  the archives in environments without the uv project.
- **Skill:**
  `softschema skill --install --scope project --agent portable --agent claude` wrote
  `.agents/skills/softschema/SKILL.md` and `.claude/skills/softschema/SKILL.md`.
  Regenerate them when the pin changes rather than editing them.
  Their zero-install fallback resolves `softschema@latest`, so `AGENTS.md` should route
  agents in this repository to the pinned
  `uv --config-file uv.toml run --frozen softschema`, as it now does.

## Next Steps

- [ ] Scaffold the repository to this baseline (the plan’s first Phase 1 task).
- [x] Confirm the decisions marked above.
  *(Confirmed 2026-09-13 and 2026-09-14 as design Decisions
  [2](../../urollup-design.md#decision-2-mit-license),
  [14](../../urollup-design.md#decision-14-exit-codes),
  [23](../../urollup-design.md#decision-23-engineering-baseline) and
  [27](../../urollup-design.md#decision-27-release-scope).)*
- [x] Check crates.io and PyPI name availability for `urollup` and `urollup-core`.
  *(Both were unregistered on 2026-09-13.)*
- [ ] Configure crates.io and PyPI trusted publishers and the protected `release`
  environment before the first release.
- [ ] Measure the musl release build on a representative corpus before choosing an
  allocator.
- [ ] Record reachable hosts for each cloud environment in the cloud smoke test.

## Methodology

Each fdu and flowmark-rs claim cites a file read at the stated commit.
Guideline claims come from the tbd guideline text loaded on 2026-09-13. The softschema
version and upload date come from the PyPI JSON API. Not verified here: default network
policy of Claude Code Cloud environments, musl allocator impact on urollup’s workload,
and crates.io or PyPI name availability.

## References

- [fdu at commit `afbb2ee`](https://github.com/jlevy/fdu/tree/afbb2eef01e94f37a4462549b0828ca8337a5f4c)
- [flowmark-rs at commit `f1e9337`](https://github.com/jlevy/flowmark-rs/tree/f1e9337e2d87ba614c231f0d17a2c181a3634117)
- [softschema](https://github.com/jlevy/softschema)
- [tryscript](https://github.com/jlevy/tryscript)
- [Supply Chain Hardening guidebook](https://github.com/jlevy/supply-chain-hardening)
- [uv settings reference](https://docs.astral.sh/uv/reference/settings/) (official docs)
- [crates.io trusted publishing](https://crates.io/docs/trusted-publishing) (official
  docs)
- [PyPI trusted publishers](https://docs.pypi.org/trusted-publishers/) (official docs)
- [maturin bindings](https://www.maturin.rs/bindings.html) (official docs)
- [actions/attest-build-provenance](https://github.com/actions/attest-build-provenance)
- [cargo-deny](https://github.com/EmbarkStudios/cargo-deny)
- [insta](https://insta.rs) and [proptest](https://github.com/proptest-rs/proptest)
- [Taplo](https://taplo.tamasfe.dev)

<!-- This document follows common-doc-guidelines.md.
See github.com/jlevy/practical-prose and review guidelines before editing.
-->
