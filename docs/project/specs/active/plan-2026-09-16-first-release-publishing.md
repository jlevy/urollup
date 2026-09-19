---
title: "urollup 0.1.0 Publishing and Distribution"
description: End-to-end plan for building, validating, publishing, and verifying the first urollup release through GitHub Releases, crates.io, and PyPI binary wheels for uvx.
author: Joshua Levy with LLM assistance
date: 2026-09-16
status: Active; plan finalized, release implementation not started
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

As of 2026-09-19, readiness is divided into four gates so a locally testable alpha is
not confused with a publishable release:

| Gate | Exit condition | Current state |
| --- | --- | --- |
| Automated 0.1 product | The uncached Claude Code and Codex adapters, exact session selection, `report`, `daily`, `sessions`, JSON and table output, terminal-aware color, stderr-only interactive progress, plain machine streams, sanitized fixture parity, goldens and repository gates pass | Implemented on stacked PRs #4 and #8, both CI-green. Whole-history ingestion continues on the unpushed `scalable-ingestion` branch (`uro-n1cp`) |
| Local alpha acceptance | The privacy-tested local aggregate mode and pinned-ccusage diff run against consented logs; G1 passes; unobserved Claude record shapes and remaining maintainer decisions are resolved or explicitly deferred | The opt-in local tools and privacy sentinels are implemented; G1 (`uro-d36a`) and full-history QA (`uro-ky6c`) wait on the 512 MiB ingestion target |
| Packaging rehearsal | Every archive, wheel and Cargo package is built and validated through the credential-free release path, with the complete manifest and no external writes | Not started; this plan defines the implementation and acceptance contract |
| Publication | The accepted 0.1 commit is merged, release documentation is final, protected publishers are configured, `v0.1.0` is approved, and every registry-backed installation probe passes | Blocked by local acceptance and the packaging rehearsal |

The local alpha can be installed and exercised before packaging machinery exists.
Publication begins only after both the product-acceptance and packaging-rehearsal gates
pass; neither gate is evidence for the other.

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
   which verifies the full set and orders `urollup-core` before `urollup`. On a rerun,
   accept an existing version only when its registry checksum matches the planned
   package; treat a publish polling timeout as unknown until the registry is queried.
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

- [ ] Add tested release-plan and artifact-validation scripts for version identity,
  target mapping, archive naming and contents, registry-state comparison, checksums,
  manifests, and installed-artifact smoke tests.
- [ ] Add local `make` targets for a single-host release rehearsal and for the full
  no-publish validation path used by CI.
- [ ] Add `crates/urollup/pyproject.toml` for a Maturin `bin` wheel without changing the
  root development-only `pyproject.toml`; pin Maturin under the supply-chain policy.
- [ ] Build the five GitHub archive targets and five PyPI wheel targets, enforcing the
  macOS 11.0 deployment floor and confirming the manylinux 2.17 tag on both Linux
  wheels.
- [ ] Smoke-test every archive and wheel on a matching native host, including
  `--version` and an explicit-source JSON report over a synthetic fixture.
- [ ] Pin Cargo 1.90 or newer; package both Cargo crates with
  `cargo package --locked --workspace`, inspect their contents, build outside the
  workspace, and smoke-test an install from the packaged source.
- [ ] Add `release.yml` with dispatch rehearsal and tag publication paths, SHA-pinned
  actions, read-only defaults, per-job permissions, version concurrency, artifact
  retention, and the protected `release` environment boundary.
- [ ] Prove the release gates fail for a tag/version mismatch, missing target, malformed
  archive, missing or wrong wheel script payload or `RECORD` entry, empty artifact set,
  checksum conflict, and partial registry state.
- [ ] Create the PyPI pending trusted publisher for project `urollup`, workflow
  `release.yml`, and environment `release`; configure the GitHub environment and prepare
  a shortest-expiry crates.io token with only `publish-new` and exact `urollup-core` and
  `urollup` crate scopes, without exposing it to build jobs.
- [ ] Run a full dispatch rehearsal from the intended 0.1.0 commit and retain its
  release plan, artifacts, smoke-test results, checksums, and manifest for review.

### Phase 2: Publish and Verify 0.1.0

- [ ] Confirm every 0.1 milestone acceptance gate required by the main plan, run
  `make check`, and confirm the release commit is clean, pushed, and on `main`.
- [ ] Finalize Cargo metadata, `README.md`, `CHANGELOG.md`, release notes, known
  limitations, and the publishing and incident runbook.
- [ ] Create and push annotated tag `v0.1.0` on the reviewed release commit; review the
  workflow’s plan and validated artifacts before approving the `release` environment.
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
