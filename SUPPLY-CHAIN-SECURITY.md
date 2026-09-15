# Supply-Chain Security

This repository delays new executable dependencies for at least 14 days, installs from
committed lockfiles, disables npm lifecycle scripts, pins GitHub Actions to reviewed
commits, and treats missing provenance as a hard failure.
The governing policy is the tbd `supply-chain-hardening` guideline and the
[Supply Chain Hardening guidebook](https://github.com/jlevy/supply-chain-hardening).

## The Gate

`make supply-chain` tests and runs `scripts/check-supply-chain.mjs`, a zero-dependency
validator driven by [supply-chain-policy.json](supply-chain-policy.json).
CI runs it first, and every other job waits for it.
It checks against the authoritative services:

- **Cargo:** every `Cargo.lock` registry package’s checksum, yank status and crates.io
  publication date.
- **npm:** every `package-lock.json` package’s integrity, tarball URL and publication
  date.
- **PyPI:** every `uv.lock` artifact’s SHA-256 hash and upload time.
- **GitHub Actions:** every `uses:` pins a 40-character commit that GitHub reports as
  verified, committed at least 14 days ago.
- **Toolchains and bootstrap tools:** the Rust channel manifests, the Node.js
  `SHASUMS256.txt`, and the GitHub release assets for uv, gh and cargo-deny match their
  pinned digests, and each file that names them uses the pinned version.
- **Workflow trust:** every workflow grants only `contents: read` at the top level;
  pull-request jobs grant no write permission, set `persist-credentials: false`, write
  no reusable cache, and run `npm ci --ignore-scripts`.
- **Scripts:** any shell script under `.claude`, `.codex` or `scripts` that downloads or
  executes packages is inventoried in the policy; tbd hook copies stay identical.

GitHub API checks use `GITHUB_TOKEN` or `GH_TOKEN` when provided and otherwise reuse the
local GitHub CLI credential without invoking a shell or printing it.
CI passes only its read-only workflow token.

Two other gates cover dependencies: `make audit` runs cargo-deny (advisories, licenses,
bans and sources, configured in [deny.toml](deny.toml)), and `make npm-audit` runs
`npm audit` and `npm audit signatures`. Every `deny.toml` ignore must name the tbd bead
that tracks it and the condition for removing it.

## Changing Dependencies

Dependency changes must be narrow and intentional:

1. Confirm the dependency is necessary and inspect its source and ownership.
2. Select a version that has been public for at least 14 full days.
   Cargo has no resolution-time cool-off, so after adding a crate run
   `cargo update -p <crate> --precise <version>` for any locked crate the gate reports
   as too new; for npm, resolve with `npm install --before=<date 14 days ago>`; uv
   applies `exclude-newer = "14 days"` from `uv.toml`.
3. Regenerate only the relevant lockfile and review the source and lock diffs.
4. Run `make supply-chain`, `make audit`, `make npm-audit` and `make check`.
5. Commit every changed lockfile with the manifest change.

Exceptions require a specific version, reason, prior maintainer approval, expiration and
follow-up in `supply-chain-policy.json`. An exception can waive release age only; it can
never waive missing or mismatched checksums, integrity, timestamps or source provenance.
The gate fails on an expired exception even when nothing uses it.
Agents do not approve new exceptions.

## First-Party Packages

Packages published from github.com/jlevy, the same authorship as this repository, are
exempt from release age by identity rather than by version, under `firstParty` in the
policy: npm `get-tbd` and `tryscript`, and PyPI `softschema` and `flowmark-rs`.
`uv.toml` carries the matching `exclude-newer-package` entries for the PyPI packages.
The cool-off exists so that a compromised upstream release is noticed by somebody else
before this repository takes it, which does not apply to a package this project’s own
authors publish. A `firstParty` entry carries no version and waives nothing but age:
integrity, hashes and publication-time provenance are verified on every run.

## Bootstrap Scripts

- `.claude/scripts/ensure-gh-cli.sh` and its `.codex` copy install gh 2.92.0 and verify
  each archive against a pinned SHA-256 digest.
- The tbd session and closing hooks fall back to `npx get-tbd@<version>`, where the
  version is `tbd_fallback_version` in `.tbd/config.yml`, checked against the policy.
- `scripts/install-cargo-deny.sh` installs cargo-deny 0.20.2 for CI and verifies the
  archive digest before extraction.
  Local development installs it with
  `cargo install cargo-deny --locked --version 0.20.2`.

## Reviewed Versions (Milestone 0.1 Scaffold)

Checked on 2026-09-15, when the cool-off cutoff was 2026-09-01. The gate re-verifies all
of these on every run; this record explains the choices.

| Item | Version | Published | Note |
| --- | --- | --- | --- |
| Rust toolchain (`rust-toolchain.toml`) | 1.98.0 | Channel manifest 2026-08-20 | Development and CI pin |
| Rust MSRV (`rust-version`) | 1.85.0 | Channel manifest 2025-02-20 | Builds and tests with no raise needed |
| clap | 4.6.6 | 2026-08-06 | 4.6.7 (2026-09-14) held back |
| clap_builder | 4.6.6 | 2026-08-06 |  |
| clap_derive | 4.6.4 | 2026-07-21 | Required exactly by clap 4.6.6 |
| clap_lex | 1.1.0 | 2026-03-12 | 1.1.1 (2026-09-14) held back |
| syn | 3.0.4 | 2026-08-24 | 3.0.5 (2026-09-04) held back |
| proc-macro2 | 1.0.107 | 2026-07-19 |  |
| quote | 1.0.47 | 2026-07-19 |  |
| anstyle | 1.0.14 | 2026-03-13 |  |
| unicode-ident | 1.0.24 | 2026-02-16 |  |
| heck | 0.5.0 | 2024-03-12 |  |
| tryscript (npm) | 0.2.1 | 2026-08-22 | First-party |
| @taplo/cli (npm) | 0.7.0 | 2024-02-01 | Bundles taplo 0.9.0; no dependencies |
| Other npm packages | 59 locked | Newest is fastq 1.20.3, 2026-08-29 | Resolved with `--before=2026-09-01` |
| get-tbd (npm bootstrap) | 0.8.1 | 2026-08-26 | First-party |
| softschema (PyPI) | 0.8.1 | 2026-09-11 | First-party, pinned before this scaffold |
| flowmark-rs (PyPI) | 0.4.0 | 2026-09-04 | First-party, pinned before this scaffold |
| Node.js (CI) | 24.19.0 | 2026-08-03 | Matches local development |
| uv (CI and `UV_MIN_VERSION`) | 0.12.1 | 2026-07-31 | Oldest release that parses relative `exclude-newer` |
| cargo-deny | 0.20.2 | 2026-07-09 | Same release as fdu’s CI action image |
| gh (bootstrap) | 2.92.0 | 2026-04-28 |  |
| actions/checkout | `d23441a` (v6.1.0) | 2026-07-16 |  |
| actions/setup-node | `8207627` (v7.0.0) | 2026-07-14 |  |
| dtolnay/rust-toolchain | `2c7215f` | 2026-07-16 | Toolchain version given as an input |
| astral-sh/setup-uv | `37802ad` (v7) | 2026-03-16 |  |

## Reviewed Versions (Milestone 0.1 Core: Ledger and Snapshot Reading)

Checked on 2026-09-15 against the crates.io API, when the cool-off cutoff was
2026-09-01, for beads `uro-spce` and `uro-26dh`. Every crate below builds on the 1.85
MSRV. `urollup-core` gained no CLI, HTTP or async-runtime crate; `make dependency-guard`
checks that.

| Crate | Version | Published | Why, and what was held back |
| --- | --- | --- | --- |
| serde (derive) | 1.0.229 | 2026-07-18 | Typed lenient record decoding |
| serde_json | 1.0.151 | 2026-07-20 | JSONL record parsing; its `memchr` is shared with the prefilter |
| sha2 | 0.11.0 | 2026-03-25 | SHA-256 for analytical IDs and fingerprints; MSRV 1.85 |
| thiserror | 2.0.20 | 2026-08-08 | Typed library errors (`rust-rules`) |
| memchr | 2.8.3 | 2026-07-08 | `memmem` line prefilter ported from ccusage |
| jiff (`std` only) | 0.2.35 | 2026-07-25 | RFC 3339 timestamps; 0.2.36 and 0.2.37 (2026-09-12) held back; no time zone database features yet |
| zstd (no default features) | 0.13.3 | 2025-02-20 | `.jsonl.zst` decoding; 0.14.0 (2026-09-04) held back |
| zstd-safe | 7.2.4 | 2025-03-20 | 7.3.0 (2026-09-04, relicensed BSD-3-Clause) held back |
| zstd-sys | 2.0.16+zstd.1.5.7 | 2025-09-04 | Bundled libzstd 1.5.7, built with `cc`; 2.1.0 (2026-09-04) held back |
| proptest (dev, `std` only) | 1.11.0 | 2026-03-24 | Property tests; no `fork` or `timeout` features |
| tempfile (dev) | 3.27.0 | 2026-03-11 | Isolated filesystem test roots |

Transitive crates pinned below their newest release with `cargo update --precise`: `cc`
1.4.4 (1.4.6 is 2026-09-13), `find-msvc-tools` 0.1.11, `hybrid-array` 0.4.14,
`jiff-core` 0.1.0, `portable-atomic-util` 0.2.7, `zerocopy` and `zerocopy-derive`
0.8.56, and `bitflags` 2.13.1. The newest locked crate is `cpufeatures` 0.3.1, published
2026-08-26. Of jiff’s locked dependencies, `defmt` and `log` are optional features left
off, `portable-atomic` builds only for targets without pointer-width atomics, and
`jiff-static` sits behind a never-true `cfg(any())`.

<!-- This document follows common-doc-guidelines.md.
See github.com/jlevy/practical-prose and review guidelines before editing.
-->
