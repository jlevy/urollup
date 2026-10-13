// The credential-free crates.io rehearsal (plan, crates.io Packages).
//
// `cargo package --locked --workspace` (Cargo 1.90 or newer) packages and verifies both
// crates together, resolving the unpublished urollup-core through a temporary local
// registry. This module then checks what a publication would upload: each package's file
// list and metadata, that it came from the release commit with a clean tree, that packaging
// is deterministic, that each extracted package builds outside the workspace, and that the
// extracted urollup installs, with urollup-core patched to its extracted sibling, into a
// temporary root whose executable passes the smoke tests.

import { spawnSync } from "node:child_process";
import { cpSync, existsSync, mkdirSync, readFileSync, readdirSync, rmSync, writeFileSync } from "node:fs";
import path from "node:path";

import { readTarGz } from "./release-archive.mjs";
import { sha256 } from "./release-inspect.mjs";
import { CARGO_PACKAGES, crateName } from "./release-targets.mjs";

/** The oldest Cargo whose workspace publication orders and verifies interdependent crates. */
export const MINIMUM_CARGO = [1, 90];

/** The repository and minimum Rust version every package must declare. */
export const PACKAGE_METADATA = {
  license: "MIT",
  repository: "https://github.com/jlevy/urollup",
  "rust-version": "1.85",
};

/** Files a package may contain, beyond Cargo's own generated files. */
const ALLOWED = {
  "urollup-core": [/^src\/[A-Za-z0-9_/.-]+\.rs$/, /^README\.md$/],
  urollup: [/^src\/[A-Za-z0-9_/.-]+\.rs$/, /^tests\/[A-Za-z0-9_-]+\.rs$/, /^README\.md$/],
};
const GENERATED = [".cargo_vcs_info.json", "Cargo.lock", "Cargo.toml", "Cargo.toml.orig"];

function fail(message) {
  throw new Error(message);
}

/** Refuse a Cargo older than the workspace-publication baseline. */
export function checkCargoVersion(text) {
  const match = text.match(/^cargo (\d+)\.(\d+)\.(\d+)/);
  if (!match) {
    return [`cannot read a Cargo version from ${JSON.stringify(text.trim())}`];
  }
  const [major, minor] = [Number(match[1]), Number(match[2])];
  if (major < MINIMUM_CARGO[0] || (major === MINIMUM_CARGO[0] && minor < MINIMUM_CARGO[1])) {
    return [`cargo ${match[1]}.${match[2]}.${match[3]} is older than ${MINIMUM_CARGO.join(".")}, which workspace packaging and publication require`];
  }
  return [];
}

/** Check one package's `cargo package --list` output against its allowlist. */
export function checkPackageListing(crate, files) {
  const problems = [];
  const allowed = ALLOWED[crate];
  if (!allowed) {
    return [`unknown package ${crate}`];
  }
  for (const required of [...GENERATED, "README.md"]) {
    if (!files.includes(required)) {
      problems.push(`${crate}: the package lacks ${required}`);
    }
  }
  const entry = crate === "urollup" ? "src/main.rs" : "src/lib.rs";
  if (!files.includes(entry)) {
    problems.push(`${crate}: the package lacks ${entry}`);
  }
  for (const file of files) {
    if (!GENERATED.includes(file) && !allowed.some((pattern) => pattern.test(file))) {
      problems.push(`${crate}: unexpected packaged file ${file}`);
    }
  }
  return problems;
}

/** Check `.cargo_vcs_info.json`: the release commit, a clean tree, the member's path. */
export function checkVcsInfo(text, { commit, pathInVcs }) {
  let info;
  try {
    info = JSON.parse(text);
  } catch (error) {
    return [`.cargo_vcs_info.json is not JSON: ${error.message}`];
  }
  const problems = [];
  if (info.git?.sha1 !== commit) {
    problems.push(`the package was made from ${info.git?.sha1 ?? "an unknown commit"}, not the release commit ${commit}`);
  }
  if (info.git?.dirty) {
    problems.push("the package was made from a dirty working tree");
  }
  if (info.path_in_vcs !== pathInVcs) {
    problems.push(`the package's path_in_vcs is ${info.path_in_vcs}, expected ${pathInVcs}`);
  }
  return problems;
}

function tomlValue(text, table, key) {
  const lines = text.split(/\r?\n/);
  let current = null;
  for (const line of lines) {
    const header = line.match(/^\[([^\]]+)\]\s*$/);
    if (header) {
      current = header[1];
      continue;
    }
    if (current !== table) {
      continue;
    }
    const match = line.match(new RegExp(`^${key}\\s*=\\s*(.+?)\\s*$`));
    if (match) {
      return match[1];
    }
  }
  return null;
}

/** Check a package's normalized Cargo.toml: identity, license, repository, MSRV, README, features. */
export function checkPackagedManifest(crate, text, { version, files }) {
  const problems = [];
  const value = (key) => tomlValue(text, "package", key);
  const expectString = (key, expected) => {
    const found = value(key);
    if (found !== JSON.stringify(expected)) {
      problems.push(`${crate}: package ${key} is ${found ?? "missing"}, expected ${JSON.stringify(expected)}`);
    }
  };
  expectString("name", crate);
  expectString("version", version);
  for (const [key, expected] of Object.entries(PACKAGE_METADATA)) {
    expectString(key, expected);
  }
  const description = value("description");
  if (!description || description === '""') {
    problems.push(`${crate}: package description is missing`);
  }
  const readme = value("readme");
  if (readme !== '"README.md"' || !files.includes("README.md")) {
    problems.push(`${crate}: the package README is ${readme ?? "missing"}`);
  }
  if (crate === "urollup") {
    const defaults = tomlValue(text, "features", "default");
    if (defaults !== '["serve"]') {
      problems.push(`urollup: default features are ${defaults ?? "missing"}, expected ["serve"]`);
    }
  }
  return problems;
}

/**
 * Rewrite a packaged Cargo.lock so `crate` is a path dependency, as Cargo records a
 * `[patch]` that points at a local directory. Every other entry, and so every other
 * dependency version, stays as packaged, which `cargo install --locked` then enforces.
 */
export function pathifyLock(lockText, crate) {
  const blocks = lockText.split(/\n(?=\[\[package\]\]\n)/);
  let changed = 0;
  const rewritten = blocks.map((block) => {
    if (!new RegExp(`^\\[\\[package\\]\\]\\nname = "${crate}"\\n`).test(block)) {
      return block;
    }
    changed += 1;
    return block
      .split("\n")
      .filter((line) => !/^source = /.test(line) && !/^checksum = /.test(line))
      .join("\n");
  });
  if (changed !== 1) {
    fail(`the packaged Cargo.lock has ${changed} entries for ${crate}, expected one`);
  }
  return rewritten.join("\n");
}

/** Extract a .crate (gzip tar under `<name>-<version>/`) into `dir`, refusing anything unsafe. */
export function extractCrate(buffer, expectedPrefix, dir) {
  const entries = readTarGz(buffer);
  const files = [];
  for (const entry of entries) {
    if (entry.type !== "file") {
      fail(`${expectedPrefix}: packaged entry ${entry.name} is a ${entry.type}`);
    }
    const parts = entry.name.split("/");
    if (parts[0] !== expectedPrefix || parts.length < 2 || parts.some((part) => part === "" || part === "." || part === "..")) {
      fail(`${expectedPrefix}: packaged entry ${entry.name} is outside ${expectedPrefix}/`);
    }
    const target = path.join(dir, ...parts);
    mkdirSync(path.dirname(target), { recursive: true });
    writeFileSync(target, entry.data);
    files.push(parts.slice(1).join("/"));
  }
  return files.sort();
}

// ---------------------------------------------------------------------------------------
// Orchestration

function cargo(args, { cwd, env, label }) {
  const result = spawnSync("cargo", args, { cwd, env, encoding: "utf8", maxBuffer: 256 * 1024 * 1024, stdio: ["ignore", "pipe", "pipe"] });
  if (result.status !== 0) {
    const tail = `${result.stdout ?? ""}${result.stderr ?? ""}`.trim().split("\n").slice(-30).join("\n");
    fail(`${label ?? `cargo ${args.join(" ")}`} failed (exit ${result.status ?? result.signal}):\n${tail}`);
  }
  return result.stdout;
}

function packagedCrates(directory, version) {
  return CARGO_PACKAGES.map((crate) => {
    const file = path.join(directory, "package", crateName(crate, version));
    if (!existsSync(file)) {
      fail(`cargo package did not produce ${crateName(crate, version)}`);
    }
    return { crate, file, data: readFileSync(file) };
  });
}

/**
 * Run the whole crates.io rehearsal in `work` (outside the workspace), copying the packages
 * to `out`. `smoke(executable, installDirs)` is the installed-executable smoke test.
 * Returns the record written beside the packages.
 */
export function rehearseCrates({ root, commit, version, work, out, smoke, allowDirty = false, env = process.env }) {
  const problems = [];
  const cargoEnv = { ...env };
  delete cargoEnv.CARGO_TARGET_DIR;
  const versionText = cargo(["--version"], { cwd: root, env: cargoEnv });
  problems.push(...checkCargoVersion(versionText));
  if (problems.length > 0) {
    return { problems };
  }
  rmSync(work, { recursive: true, force: true });
  mkdirSync(work, { recursive: true });
  const dirty = allowDirty ? ["--allow-dirty"] : [];

  const first = path.join(work, "package-a");
  cargo(["package", "--locked", "--workspace", ...dirty], { cwd: root, env: { ...cargoEnv, CARGO_TARGET_DIR: first }, label: "cargo package --locked --workspace" });
  const packages = packagedCrates(first, version);

  const second = path.join(work, "package-b");
  cargo(["package", "--locked", "--workspace", "--no-verify", ...dirty], { cwd: root, env: { ...cargoEnv, CARGO_TARGET_DIR: second }, label: "cargo package --no-verify (determinism check)" });
  for (const again of packagedCrates(second, version)) {
    const original = packages.find((item) => item.crate === again.crate);
    if (sha256(again.data) !== sha256(original.data)) {
      problems.push(`${again.crate}: packaging the same commit twice gave different bytes, so a rerun could not prove a published package identical`);
    }
  }

  const record = { schema: "urollup-release-crates/v1", version, commit, cargo: versionText.trim(), packages: [], checks: [] };
  const extracted = path.join(work, "extracted");
  mkdirSync(extracted, { recursive: true });
  for (const item of packages) {
    const listing = cargo(["package", "--list", "--locked", "-p", item.crate, ...dirty], { cwd: root, env: cargoEnv })
      .split(/\r?\n/)
      .filter(Boolean)
      .sort();
    problems.push(...checkPackageListing(item.crate, listing));
    const prefix = `${item.crate}-${version}`;
    const files = extractCrate(item.data, prefix, extracted);
    if (JSON.stringify(files) !== JSON.stringify(listing)) {
      problems.push(`${item.crate}: the .crate holds ${files.length} files, but cargo package --list names ${listing.length}`);
    }
    const dir = path.join(extracted, prefix);
    if (!allowDirty) {
      problems.push(...checkVcsInfo(readFileSync(path.join(dir, ".cargo_vcs_info.json"), "utf8"), { commit, pathInVcs: `crates/${item.crate}` }).map((problem) => `${item.crate}: ${problem}`));
    }
    problems.push(...checkPackagedManifest(item.crate, readFileSync(path.join(dir, "Cargo.toml"), "utf8"), { version, files }));
    record.packages.push({ crate: item.crate, name: crateName(item.crate, version), size: item.data.length, sha256: sha256(item.data), files: listing });
  }
  if (problems.length > 0) {
    return { record, problems };
  }

  const coreDir = path.join(extracted, `urollup-core-${version}`);
  cargo(["build", "--locked"], { cwd: coreDir, env: { ...cargoEnv, CARGO_TARGET_DIR: path.join(work, "build-core") }, label: "build the extracted urollup-core outside the workspace" });
  record.checks.push({ check: "urollup-core builds outside the workspace" });

  const source = path.join(work, "install-source");
  cpSync(path.join(extracted, `urollup-${version}`), source, { recursive: true });
  writeFileSync(path.join(source, "Cargo.lock"), pathifyLock(readFileSync(path.join(source, "Cargo.lock"), "utf8"), "urollup-core"));
  const installRoot = path.join(work, "install-root");
  const patch = `patch.crates-io.urollup-core.path=${JSON.stringify(coreDir.split(path.sep).join("/"))}`;
  cargo(["install", "--locked", "--path", source, "--root", installRoot, "--config", patch], {
    cwd: work,
    env: { ...cargoEnv, CARGO_TARGET_DIR: path.join(work, "build-install") },
    label: "cargo install the extracted urollup with urollup-core patched to its extracted sibling",
  });
  record.checks.push({ check: "urollup installs from its package with the packaged dependency versions" });
  const binDir = path.join(installRoot, "bin");
  const executable = readdirSync(binDir).find((name) => name === "urollup" || name === "urollup.exe");
  if (!executable) {
    return { record, problems: [`cargo install left no urollup in ${binDir}`] };
  }
  const smoked = smoke(path.join(binDir, executable), [binDir]);
  record.checks.push(...smoked.checks);
  problems.push(...smoked.problems);

  mkdirSync(out, { recursive: true });
  for (const item of packages) {
    writeFileSync(path.join(out, crateName(item.crate, version)), item.data);
  }
  record.status = problems.length === 0 ? "passed" : "failed";
  writeFileSync(path.join(out, "crates.json"), `${JSON.stringify(record, null, 2)}\n`);
  return { record, problems };
}
