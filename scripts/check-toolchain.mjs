#!/usr/bin/env node
// Toolchain preflight for `make check`: fail up front, with the install command, when a
// tool the gate needs is missing or is not the reviewed version.
//
// Each check exists for a failure that otherwise reads as something else:
//   * a rustc other than the rust-toolchain.toml pin lints with a different rule set, so
//     clippy passes locally and fails in CI (or the reverse) with no diff to explain it;
//   * a missing MSRV toolchain makes `cargo +1.85.0` ask rustup to download one mid-gate;
//   * a cargo-deny other than the version CI runs can disagree about advisories and
//     license expressions.
// The uv floor has its own preflight (`make uv-version`), ported from fdu.

import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

export function parseToolchainChannel(text) {
  const channel = text.match(/^\s*channel\s*=\s*"([^"]+)"\s*$/m)?.[1];
  if (!channel || !/^\d+\.\d+\.\d+$/.test(channel)) {
    throw new Error("rust-toolchain.toml must pin an exact MAJOR.MINOR.PATCH channel");
  }
  return channel;
}

export function parseRustcVersion(output) {
  return output.match(/^rustc (\d+\.\d+\.\d+)(?:\s|$)/m)?.[1];
}

export function parseCargoDenyVersion(output) {
  return output.match(/^cargo-deny (\d+\.\d+\.\d+)(?:\s|$)/m)?.[1];
}

export function hasRustupToolchain(listOutput, version) {
  return listOutput.split("\n").some((line) => line.trim().startsWith(`${version}-`));
}

/**
 * Compare observed tool state with the pins and return one problem per mismatch.
 * `undefined` output means the tool could not be run at all.
 */
export function toolchainProblems({ channel, rustc, msrv, toolchains, cargoDeny, cargoDenyOutput }) {
  const problems = [];
  const rustcVersion = rustc === undefined ? undefined : parseRustcVersion(rustc);
  if (rustcVersion !== channel) {
    problems.push(
      `rustc is ${rustcVersion ?? "not runnable"}, but rust-toolchain.toml pins ${channel}.\n` +
        `  Install rustup (https://rustup.rs) so the pin applies, or run: rustup toolchain install ${channel} --profile minimal --component clippy,rustfmt`,
    );
  }
  if (toolchains === undefined || !hasRustupToolchain(toolchains, msrv)) {
    problems.push(
      `the MSRV toolchain ${msrv} is not installed.\n` +
        `  Install it with: rustup toolchain install ${msrv} --profile minimal`,
    );
  }
  const denyVersion = cargoDenyOutput === undefined ? undefined : parseCargoDenyVersion(cargoDenyOutput);
  if (denyVersion !== cargoDeny) {
    problems.push(
      `cargo-deny is ${denyVersion ?? "not installed"}, but the audit gate pins ${cargoDeny} (the CI version).\n` +
        `  Install it with: cargo install cargo-deny --locked --version ${cargoDeny}`,
    );
  }
  return problems;
}

function capture(command, args) {
  const result = spawnSync(command, args, { cwd: ROOT, encoding: "utf8" });
  return result.error || result.status !== 0 ? undefined : result.stdout;
}

export function parseArgs(argv) {
  const options = {};
  for (let index = 0; index < argv.length; index += 2) {
    const key = argv[index];
    const value = argv[index + 1];
    if (!["--msrv", "--cargo-deny"].includes(key) || value === undefined) {
      throw new Error("usage: check-toolchain.mjs --msrv X.Y.Z --cargo-deny X.Y.Z");
    }
    options[key.slice(2)] = value;
  }
  if (!options.msrv || !options["cargo-deny"]) {
    throw new Error("usage: check-toolchain.mjs --msrv X.Y.Z --cargo-deny X.Y.Z");
  }
  return { msrv: options.msrv, cargoDeny: options["cargo-deny"] };
}

function main() {
  const { msrv, cargoDeny } = parseArgs(process.argv.slice(2));
  const channel = parseToolchainChannel(readFileSync(path.join(ROOT, "rust-toolchain.toml"), "utf8"));
  const problems = toolchainProblems({
    channel,
    rustc: capture(process.env.RUSTC ?? "rustc", ["--version"]),
    msrv,
    toolchains: capture("rustup", ["toolchain", "list"]),
    cargoDeny,
    cargoDenyOutput: capture(process.env.CARGO ?? "cargo", ["deny", "--version"]),
  });
  if (problems.length > 0) {
    for (const problem of problems) {
      console.error(`toolchain: ${problem}`);
    }
    process.exitCode = 1;
    return;
  }
  console.log(`toolchain: rustc ${channel}, MSRV ${msrv} and cargo-deny ${cargoDeny} are in place`);
}

if (import.meta.url === pathToFileURL(process.argv[1] ?? "").href) {
  try {
    main();
  } catch (error) {
    console.error(`toolchain: ${error.message}`);
    process.exitCode = 1;
  }
}
