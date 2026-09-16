# Provenance

This file records code, configuration and tests ported into urollup from other
repositories by the same maintainer, with the source path and commit for each.
The maintainer holds full license rights to these sources, so ported files carry
urollup’s MIT license.
Third-party code under other licenses will be listed in a separate `THIRD-PARTY-NOTICES`
file with the notices it requires, once any is ported.

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

These urollup files follow fdu patterns without copying a source file:
`crates/urollup/src/cli.rs` (the `run` function with injected writers, from
`crates/fdu/src/cli.rs`), `scripts/check-dependency-guard.mjs` (the captured
`cargo tree` guard in fdu’s `make lib-only`), `scripts/install-cargo-deny.sh` (the
digest-verified download in `scripts/bootstrap-gh-cli.sh`) and `scripts/prove-gates.mjs`
(fdu’s violation-fed script tests and stub-driven make test).

<!-- This document follows common-doc-guidelines.md.
See github.com/jlevy/practical-prose and review guidelines before editing.
-->
