import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

import {
  hasRustupToolchain,
  parseArgs,
  parseToolchainChannel,
  toolchainProblems,
} from "./check-toolchain.mjs";

const ROOT = dirname(dirname(fileURLToPath(import.meta.url)));

const GOOD = {
  channel: "1.98.0",
  rustc: "rustc 1.98.0 (88d9e12ae 2026-08-18)\n",
  msrv: "1.85.0",
  toolchains: "stable-aarch64-apple-darwin (default)\n1.85.0-aarch64-apple-darwin\n1.98.0-aarch64-apple-darwin (active)\n",
  cargoDeny: "0.20.2",
  cargoDenyOutput: "cargo-deny 0.20.2\n",
};

test("the committed toolchain file pins an exact release", () => {
  assert.match(parseToolchainChannel(readFileSync(join(ROOT, "rust-toolchain.toml"), "utf8")), /^\d+\.\d+\.\d+$/);
  assert.throws(() => parseToolchainChannel('[toolchain]\nchannel = "stable"\n'), /exact/);
});

test("matching tools produce no problems", () => {
  assert.deepEqual(toolchainProblems(GOOD), []);
});

test("a rustc other than the pin is a problem with an install command", () => {
  const [problem, ...rest] = toolchainProblems({ ...GOOD, rustc: "rustc 1.97.1 (abc 2026-07-01)\n" });
  assert.equal(rest.length, 0);
  assert.match(problem, /rustc is 1\.97\.1, but rust-toolchain\.toml pins 1\.98\.0/);
  assert.match(problem, /rustup toolchain install 1\.98\.0/);
  assert.match(toolchainProblems({ ...GOOD, rustc: undefined })[0], /rustc is not runnable/);
});

test("a missing MSRV toolchain or rustup is a problem", () => {
  assert.equal(hasRustupToolchain(GOOD.toolchains, "1.85.0"), true);
  assert.equal(hasRustupToolchain("1.85.1-x86_64-unknown-linux-gnu\n", "1.85.0"), false);
  assert.match(toolchainProblems({ ...GOOD, toolchains: "stable-x\n" })[0], /MSRV toolchain 1\.85\.0 is not installed/);
  assert.match(toolchainProblems({ ...GOOD, toolchains: undefined })[0], /rustup toolchain install 1\.85\.0/);
});

test("a missing or different cargo-deny is a problem", () => {
  assert.match(toolchainProblems({ ...GOOD, cargoDenyOutput: undefined })[0], /cargo-deny is not installed/);
  assert.match(
    toolchainProblems({ ...GOOD, cargoDenyOutput: "cargo-deny 0.19.0\n" })[0],
    /cargo install cargo-deny --locked --version 0\.20\.2/,
  );
});

test("both pins are required arguments", () => {
  assert.deepEqual(parseArgs(["--msrv", "1.85.0", "--cargo-deny", "0.20.2"]), { msrv: "1.85.0", cargoDeny: "0.20.2" });
  assert.throws(() => parseArgs(["--msrv", "1.85.0"]), /usage/);
  assert.throws(() => parseArgs(["--msrv"]), /usage/);
  assert.throws(() => parseArgs(["--other", "x"]), /usage/);
});

test("the Makefile pins match the CI installer and the declared MSRV", () => {
  const makefile = readFileSync(join(ROOT, "Makefile"), "utf8");
  const installer = readFileSync(join(ROOT, "scripts", "install-cargo-deny.sh"), "utf8");
  const makeVersion = makefile.match(/^CARGO_DENY_VERSION := (\S+)$/m)?.[1];
  const installerVersion = installer.match(/^CARGO_DENY_VERSION="([^"]+)"$/m)?.[1];
  assert.ok(makeVersion, "Makefile declares CARGO_DENY_VERSION");
  assert.equal(installerVersion, makeVersion);
  const msrv = makefile.match(/^MSRV \?= (\S+)$/m)?.[1];
  const cargo = readFileSync(join(ROOT, "Cargo.toml"), "utf8");
  const rustVersion = cargo.match(/^rust-version = "([^"]+)"$/m)?.[1];
  assert.ok(msrv?.startsWith(`${rustVersion}.`) || msrv === rustVersion, `MSRV ${msrv} vs rust-version ${rustVersion}`);
});
