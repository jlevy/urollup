# Local development workflows. `make check` is the handoff gate: if it passes, CI should.
# Recipes stay orchestration; every decision lives in a tested script under scripts/.
#
# Adapted from fdu `Makefile` at afbb2ee; see PROVENANCE.md.

.DEFAULT_GOAL := help

CARGO ?= cargo
NODE ?= node
NPM ?= npm
UV ?= uv
# The MSRV (`rust-version` in Cargo.toml) and the cargo-deny release CI installs. The
# supply-chain policy verifies both literals against the CI workflow and the installer.
MSRV ?= 1.85.0
CARGO_DENY_VERSION := 0.20.2
NODE_INSTALL_STAMP := node_modules/.package-lock.json

# Always pass the repository's uv.toml, so user-level uv configuration never changes
# resolution (AGENTS.md).
UV_RUN = $(UV) --config-file uv.toml run --frozen
FLOWMARK = $(UV_RUN) flowmark
TAPLO := node_modules/.bin/taplo

.PHONY: help build test rust-test golden golden-update golden-lint e2e-results check toolchain uv-version \
	supply-chain lint-policy fixtures-check fmt-check toml-fmt-check docs-format-check uv-lock-check \
	clippy docs dependency-guard msrv audit npm-audit gate-proofs fix clean

help:
	@echo "make build              Debug build of the workspace"
	@echo "make test               Rust tests, CLI goldens and end-to-end result checks"
	@echo "make golden             Build and compare the tryscript CLI goldens, hermetically"
	@echo "make golden-update      Regenerate intentional golden changes, then compare"
	@echo "                        (GOLDEN=<session> limits the update to named sessions)"
	@echo "make e2e-results        Check reconciled results on every fixture case"
	@echo "make check              Handoff gate: everything CI enforces, fastest first"
	@echo "make fix                Format Rust, TOML and Markdown"
	@echo "make supply-chain       Verify release age, provenance, pins and CI trust controls"
	@echo "make fixtures-check     Check the dialect fixtures parse, match expected.json and stay synthetic"
	@echo "make dependency-guard   Prove no HTTP or async crate reaches the core or a no-serve build"
	@echo "make msrv               Compile and test the workspace on Rust $(MSRV)"
	@echo "make audit              cargo-deny advisories, licenses, bans and sources"
	@echo "make gate-proofs        Prove each gate fails on its committed violation"

build:
	$(CARGO) build --locked --workspace

test: rust-test golden e2e-results

rust-test:
	$(CARGO) test --locked --workspace
	$(CARGO) test --locked --workspace --no-default-features

$(NODE_INSTALL_STAMP): package.json package-lock.json .npmrc
	$(NPM) ci --ignore-scripts

# The harness scripts are gates too, so their decision logic is tested before they run.
GOLDEN_HARNESS_TESTS := scripts/golden-env.test.mjs scripts/check-golden-invocations.test.mjs \
	scripts/run-golden.test.mjs scripts/new-e2e-golden.test.mjs

golden-lint:
	$(NODE) --test $(GOLDEN_HARNESS_TESTS)
	$(NODE) scripts/check-golden-invocations.mjs
	$(NODE) scripts/check-portability.mjs

golden: build golden-lint $(NODE_INSTALL_STAMP)
	$(NODE) scripts/run-golden.mjs

# tryscript returns 1 when it updates a previously failing block, so only that status is
# tolerated; run-golden exits 3 when unstaged golden changes would mix into the diff. The
# immediate comparison is authoritative and catches execution failures or incomplete
# updates. `--update` writes what it saw, so read the diff, and `make golden-lint` refuses
# literals it expanded from machine-specific output.
GOLDEN ?=

golden-update: build $(NODE_INSTALL_STAMP)
	$(NODE) scripts/run-golden.mjs --update $(GOLDEN) || test $$? -eq 1
	$(MAKE) golden

# Structured final-result checks beside the transcript goldens (tests/golden/README.md).
# Commands still stubbed and a fixture corpus not yet landed print PENDING with their bead.
e2e-results: build
	$(NODE) --test scripts/check-e2e-results.test.mjs
	$(NODE) scripts/check-e2e-results.mjs

# Everything CI enforces, in the order that fails fastest.
check: toolchain uv-version supply-chain lint-policy fixtures-check fmt-check toml-fmt-check \
	docs-format-check uv-lock-check clippy test docs dependency-guard msrv audit npm-audit gate-proofs

# A rustc other than the pin, a missing MSRV toolchain or a different cargo-deny each fail
# later in a way that reads as something else; say so up front instead.
toolchain:
	$(NODE) --test scripts/check-toolchain.test.mjs
	$(NODE) scripts/check-toolchain.mjs --msrv $(MSRV) --cargo-deny $(CARGO_DENY_VERSION)

# uv.toml expresses the supply-chain cool-off as a relative `exclude-newer` ("14 days").
# uv releases older than this cannot parse that form: they abort with
# `failed to parse year in date "14 days"`, which reads like a corrupt config rather than a
# stale tool, and it takes out every uv-backed target at once. Fail early and say so.
#
# CI installs uv through astral-sh/setup-uv in .github/workflows/ci.yml. The supply-chain
# policy verifies that this floor and the CI pin remain identical.
UV_MIN_VERSION := 0.12.1

uv-version:
	@command -v "$(UV)" >/dev/null 2>&1 || { \
		echo "error: uv is not installed, and this repository needs uv >= $(UV_MIN_VERSION)."; \
		echo "       Install the reviewed $(UV_MIN_VERSION) release using the official instructions:"; \
		echo "       https://docs.astral.sh/uv/getting-started/installation/"; \
		exit 1; }
	@version_output=$$("$(UV)" --version 2>/dev/null) || { \
		echo "error: could not run uv --version; reinstall the reviewed $(UV_MIN_VERSION) release."; \
		exit 1; \
	}; \
	have=$$(printf '%s\n' "$$version_output" | awk 'NF >= 2 && $$1 == "uv" { print $$2; exit }'); \
	relation=$$(awk -v have="$$have" -v need="$(UV_MIN_VERSION)" 'BEGIN { \
		if (have !~ /^[0-9]+\.[0-9]+\.[0-9]+$$/ || need !~ /^[0-9]+\.[0-9]+\.[0-9]+$$/) { print "invalid"; exit; } \
		split(have, actual, "."); split(need, minimum, "."); \
		for (i = 1; i <= 3; i++) { \
			if (actual[i] + 0 < minimum[i] + 0) { print "old"; exit; } \
			if (actual[i] + 0 > minimum[i] + 0) { print "ok"; exit; } \
		} \
		print "ok"; \
	}'); \
	if [ "$$relation" = "old" ]; then \
		echo "error: uv $$have is too old; this repository needs uv >= $(UV_MIN_VERSION)"; \
		echo "       (the version CI pins in .github/workflows/ci.yml)."; \
		echo "       Older releases cannot parse the relative 'exclude-newer' in uv.toml"; \
		echo "       and fail with a misleading TOML date error."; \
		echo "       Upgrade to the reviewed release with:"; \
		echo "         curl -LsSf https://astral.sh/uv/$(UV_MIN_VERSION)/install.sh | sh"; \
		echo "       'uv self update $(UV_MIN_VERSION)' also works, but only when uv owns its own"; \
		echo "       install; it fails with 'not found for the app uv' under an external manager."; \
		exit 1; \
	elif [ "$$relation" != "ok" ]; then \
		echo "error: could not determine a stable uv version from: $$version_output"; \
		echo "       Reinstall the reviewed $(UV_MIN_VERSION) release before continuing."; \
		exit 1; \
	fi

# Standalone entry points must fail before any recipe asks uv to parse repository
# configuration. Keep this list aligned with the recipe-coverage test in
# scripts/check-uv-version.test.mjs.
UV_BACKED_TARGETS := docs-format-check uv-lock-check fix

$(UV_BACKED_TARGETS): uv-version

supply-chain:
	$(NODE) --test scripts/check-supply-chain.test.mjs scripts/check-uv-version.test.mjs
	$(NODE) scripts/check-supply-chain.mjs

lint-policy:
	$(NODE) --test scripts/check-lint-policy.test.mjs
	$(NODE) scripts/check-lint-policy.mjs

# Frozen test data needs a gate of its own, before any build: the fixtures are a public,
# synthetic corpus, and a case whose expected.json drifts from its records, or a record
# that carries a real path, name or key, is not something a Rust test would catch.
fixtures-check:
	$(NODE) --test scripts/check-fixtures.test.mjs scripts/sanitize-claude-fixture.test.mjs
	$(NODE) scripts/check-fixtures.mjs

fmt-check:
	$(CARGO) fmt --all --check

toml-fmt-check: $(NODE_INSTALL_STAMP)
	$(TAPLO) fmt --check

# `--auto` owns repository-wide file discovery; .flowmarkignore excludes generated skills.
docs-format-check:
	$(FLOWMARK) --auto --check .

uv-lock-check:
	$(UV) --config-file uv.toml lock --check

# `--all-targets` is load-bearing: without it tests are not linted. Both supported feature
# sets are linted, since `serve` code is invisible to a no-default-features run and the
# reverse.
clippy:
	$(CARGO) clippy --locked --workspace --all-targets -- -D warnings
	$(CARGO) clippy --locked --workspace --all-targets --no-default-features -- -D warnings

docs:
	RUSTDOCFLAGS="-D warnings" $(CARGO) doc --locked --workspace --no-deps

# How a build without the web UI is made, and proof the crate split held: no HTTP or
# async-runtime crate in the core or in that build's tree (design §7.1).
dependency-guard:
	$(CARGO) build --locked -p urollup --no-default-features
	$(NODE) --test scripts/check-dependency-guard.test.mjs
	$(NODE) scripts/check-dependency-guard.mjs

msrv:
	$(CARGO) +$(MSRV) check --locked --workspace --all-targets
	$(CARGO) +$(MSRV) test --locked --workspace

audit:
	$(CARGO) deny --locked check

npm-audit: $(NODE_INSTALL_STAMP)
	$(NPM) audit --audit-level=moderate
	$(NPM) audit signatures

# Each gate is run against its committed violation in a scratch copy of the repository
# and must fail with the expected diagnostic (tests/gate-probes/README.md).
gate-proofs: $(NODE_INSTALL_STAMP)
	$(NODE) --test scripts/prove-gates.test.mjs
	$(NODE) scripts/prove-gates.mjs

fix: $(NODE_INSTALL_STAMP)
	$(CARGO) fmt --all
	$(TAPLO) fmt
	$(FLOWMARK) --auto .

clean:
	$(CARGO) clean
