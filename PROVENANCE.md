# Provenance

This file records code, configuration and tests ported into urollup from other
repositories by the same maintainer, with the source path and commit for each.
The maintainer holds full license rights to these sources, so ported files carry
urollup’s MIT license.
Third-party code under other licenses is listed in
[THIRD-PARTY-NOTICES.md](THIRD-PARTY-NOTICES.md) with the notices it requires.

Every adapted file also names its source in a header comment where the format allows
one.

## fdu at `afbb2ee`

Source:
[jlevy/fdu at `afbb2ee`](https://github.com/jlevy/fdu/tree/afbb2eef01e94f37a4462549b0828ca8337a5f4c),
the reference repository for the
[Rust CLI engineering baseline](docs/project/research/research-2026-09-13-rust-cli-engineering-baseline.md).

| urollup file | fdu source | What changed |
| --- | --- | --- |
| `Cargo.toml` | `Cargo.toml` | Two members, shared version, baseline lint additions (`let_underscore_future`, `wildcard_enum_match_arm`), unwinding kept for `serve` instead of PyO3 |
| `rust-toolchain.toml` | `rust-toolchain.toml` | Pinned to 1.98.0 |
| `rustfmt.toml` | `rustfmt.toml` | Unchanged |
| `deny.toml` | `deny.toml` | `[graph] all-features`, bead-and-removal rule for ignores, `serve` boundary note |
| `Makefile` | `Makefile` | Kept the `check` ordering, `uv-version` guard, golden and audit targets; added toolchain, lint-policy, taplo, uv lock, dependency-guard and gate-proof targets; dropped Python bindings, parity, watch, gitignore and performance targets |
| `.github/workflows/ci.yml` | `.github/workflows/ci.yml` | Same trust controls and action pins; jobs for format, lint, test matrix, docs and uv, dependency guard, MSRV, audit and gate proofs; cargo-deny installed from a digest-pinned release |
| `.npmrc` | `.npmrc` | Unchanged |
| `package.json`, `package-lock.json` | `package.json`, `package-lock.json` | tryscript 0.2.1 kept; `yaml` and the esbuild override dropped; `@taplo/cli` added |
| `supply-chain-policy.json` | `supply-chain-policy.json` | urollup’s first-party list, get-tbd 0.8.1, cargo-deny and uv bootstrap entries, Rust 1.98.0 and Node 24.19.0 |
| `scripts/check-supply-chain.mjs` | `scripts/check-supply-chain.mjs` | One root `uv.lock`; read-only top-level permissions in every workflow; no write grants in pull-request jobs; expired exceptions fail; nested agent worktrees skipped |
| `scripts/check-supply-chain.test.mjs` | `scripts/check-supply-chain.test.mjs` | Tests for the added rules |
| `scripts/check-uv-version.test.mjs` | `scripts/check-uv-version.test.mjs` | urollup files; matches `uv` as a command word rather than a substring |
| `scripts/run-golden.mjs` | `scripts/run-golden.mjs` | One surface; honors `CARGO_TARGET_DIR`; an empty corpus fails; runs in the hermetic environment of `scripts/golden-env.mjs`, guards `--update` against unstaged changes, and requires the pass count to equal the selected blocks |
| `scripts/check-golden-invocations.mjs` | `scripts/check-golden-invocations.mjs` | `$UROLLUP_BIN`; no helper-script rule, since there are no helpers; adds the sandbox, environment, annotation, wildcard, shell-portability, timezone and canary rules |
| `scripts/check-portability.mjs` | `scripts/check-portability.mjs` | Every file under `tests/golden`; an empty corpus fails |
| `SUPPLY-CHAIN-SECURITY.md` | `SUPPLY-CHAIN-SECURITY.md` | urollup commands, bootstrap scripts and the reviewed version record |
| `SECURITY.md` | `SECURITY.md` | Adds urollup’s private-data rule for reports and fixtures |

## metaproc at `d101cd9`

Source:
[jlevy/metaproc at `d101cd9`](https://github.com/jlevy/metaproc/tree/d101cd9d7f41bcb7e590fe679f058e1026cfe8bc).

| urollup file | metaproc source | What changed |
| --- | --- | --- |
| `scripts/check-fixtures.mjs` (privacy scanner) | `devtools/public_hygiene.py` | Ported the home-path, email and credential patterns to JavaScript and added Claude Code’s encoded project names, the account and home-directory name of the machine running the check, and more key formats; dropped the hashed private-vocabulary list and the archive walker |

## squares at `f2e24e0`

Source:
[jlevy/squares at `f2e24e0`](https://github.com/jlevy/squares/tree/f2e24e07be8c94fa3ac603c3534dce7c454da99b).

| urollup file | squares source | What changed |
| --- | --- | --- |
| `crates/urollup-core/tests/fixtures/codex-rollout/legacy-subagent-prefix/` | `packing/tests/test_codex_log_rollup.py` synthetic record builders and prefix-cut cases | Rebuilt as committed JSONL with urollup’s expected results, extended with `token_usage_record`, repeated snapshots, orphaned parents and counter epochs |

## Test fixtures

`crates/urollup-core/tests/fixtures/` is synthetic, with one case sanitized from the
maintainer’s own Claude Code session logs by `scripts/sanitize-claude-fixture.mjs` on
2026-09-15, which keeps structure and usage and replaces every ID, path, name and
free-text value; no original content is retained.
Cases whose shape follows a third-party project’s tests are attributed in the
[fixtures README](crates/urollup-core/tests/fixtures/README.md); no source file was
copied, so no third-party notice file is required yet.

These urollup files follow fdu patterns without copying a source file:
`crates/urollup/src/cli.rs` (the `run` function with injected writers, from
`crates/fdu/src/cli.rs`), `scripts/check-dependency-guard.mjs` (the captured
`cargo tree` guard in fdu’s `make lib-only`), `scripts/install-cargo-deny.sh` (the
digest-verified download in `scripts/bootstrap-gh-cli.sh`) and `scripts/prove-gates.mjs`
(fdu’s violation-fed script tests and stub-driven make test).

## ccusage at `bd7f89b`

Source:
[ccusage/ccusage at `bd7f89b`](https://github.com/ccusage/ccusage/tree/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1),
MIT, Copyright (c) 2025 ryoppippi.
The license text is in [THIRD-PARTY-NOTICES.md](THIRD-PARTY-NOTICES.md); the reuse modes
come from the research brief’s
[reusable code table](docs/project/research/research-2026-09-13-portable-agent-usage.md#reusable-code-and-tests).

| urollup file | ccusage source | What changed |
| --- | --- | --- |
| `crates/urollup-core/src/sources/prefilter.rs` | `rust/crates/ccusage-core/src/fast.rs` | `Vec` instead of `SmallVec`; one constructor pair for the two modes; an empty marker list admits every line; documents that a miss is a counted skip, never a dropped record |
| `crates/urollup-core/src/sources/parallel.rs` | `rust/adapters/common/src/lib.rs` | Both `expect` panics are a `ParallelReadError`; an explicit worker bound; weights come from the caller instead of a `metadata` call that reads an unreadable file as empty; no `read_dir` collection, which the roots walk owns |

<!-- This document follows common-doc-guidelines.md.
See github.com/jlevy/practical-prose and review guidelines before editing.
-->
