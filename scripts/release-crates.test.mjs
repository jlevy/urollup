import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import test from "node:test";

import { checkCargoVersion, checkPackageListing, checkPackagedManifest, checkVcsInfo, extractCrate, pathifyLock } from "./release-crates.mjs";
import { makeTarGz } from "./release-test-binaries.mjs";

const COMMIT = "0123456789abcdef0123456789abcdef01234567";

test("workspace packaging needs Cargo 1.90 or newer", () => {
  assert.deepEqual(checkCargoVersion("cargo 1.98.0 (0d84d3 2026-08-11)\n"), []);
  assert.deepEqual(checkCargoVersion("cargo 1.90.0\n"), []);
  assert.match(checkCargoVersion("cargo 1.89.1 (x)\n").join("\n"), /older than 1\.90/);
  assert.match(checkCargoVersion("rustc 1.98.0").join("\n"), /cannot read a Cargo version/);
});

const CORE_FILES = [".cargo_vcs_info.json", "Cargo.lock", "Cargo.toml", "Cargo.toml.orig", "README.md", "src/lib.rs", "src/query/aggregate.rs"];

test("package listings hold sources and generated files only", () => {
  assert.deepEqual(checkPackageListing("urollup-core", CORE_FILES), []);
  const executable = [".cargo_vcs_info.json", "Cargo.lock", "Cargo.toml", "Cargo.toml.orig", "README.md", "src/main.rs", "src/cli.rs"];
  assert.deepEqual(checkPackageListing("urollup", executable), []);
  assert.match(checkPackageListing("urollup", [...executable, "tests/cli_process.rs"]).join("\n"), /unexpected packaged file tests\/cli_process\.rs/, "tests that need unshipped fixtures stay out of the package");
  assert.match(checkPackageListing("urollup-core", [...CORE_FILES, "tests/fixtures/claude-project/x/README.md"]).join("\n"), /unexpected packaged file tests\/fixtures/);
  assert.match(checkPackageListing("urollup-core", [...CORE_FILES, "tests/gate-probes/files/x.probe"]).join("\n"), /unexpected packaged file/);
  assert.match(checkPackageListing("urollup-core", CORE_FILES.filter((file) => file !== "README.md")).join("\n"), /lacks README\.md/);
  assert.match(checkPackageListing("urollup", CORE_FILES).join("\n"), /lacks src\/main\.rs/);
  assert.match(checkPackageListing("urollup-py", CORE_FILES).join("\n"), /unknown package/);
});

test("a package must come from the release commit with a clean tree", () => {
  const info = (git, pathInVcs = "crates/urollup") => JSON.stringify({ git, path_in_vcs: pathInVcs });
  assert.deepEqual(checkVcsInfo(info({ sha1: COMMIT }), { commit: COMMIT, pathInVcs: "crates/urollup" }), []);
  assert.match(checkVcsInfo(info({ sha1: "f".repeat(40) }), { commit: COMMIT, pathInVcs: "crates/urollup" }).join("\n"), /not the release commit/);
  assert.match(checkVcsInfo(info({ sha1: COMMIT, dirty: true }), { commit: COMMIT, pathInVcs: "crates/urollup" }).join("\n"), /dirty working tree/);
  assert.match(checkVcsInfo(info({ sha1: COMMIT }, "crates/other"), { commit: COMMIT, pathInVcs: "crates/urollup" }).join("\n"), /path_in_vcs/);
  assert.match(checkVcsInfo("{", { commit: COMMIT, pathInVcs: "x" }).join("\n"), /not JSON/);
});

const MANIFEST = `[package]
edition = "2024"
rust-version = "1.85"
name = "urollup"
version = "0.1.0"
description = "Trustworthy token and usage rollups"
readme = "README.md"
license = "MIT"
repository = "https://github.com/jlevy/urollup"

[features]
default = ["serve"]
serve = []
`;

test("packaged metadata names the release, license, repository, MSRV, README and default features", () => {
  const files = ["README.md", "src/main.rs"];
  assert.deepEqual(checkPackagedManifest("urollup", MANIFEST, { version: "0.1.0", files }), []);
  assert.match(checkPackagedManifest("urollup", MANIFEST, { version: "0.2.0", files }).join("\n"), /version is "0\.1\.0", expected "0\.2\.0"/);
  assert.match(checkPackagedManifest("urollup", MANIFEST.replace('"MIT"', '"Apache-2.0"'), { version: "0.1.0", files }).join("\n"), /license/);
  assert.match(checkPackagedManifest("urollup", MANIFEST.replace('rust-version = "1.85"\n', ""), { version: "0.1.0", files }).join("\n"), /rust-version is missing/);
  assert.match(checkPackagedManifest("urollup", MANIFEST.replace('default = ["serve"]', "default = []"), { version: "0.1.0", files }).join("\n"), /default features are \[\]/);
  assert.match(checkPackagedManifest("urollup", MANIFEST, { version: "0.1.0", files: ["src/main.rs"] }).join("\n"), /README/);
  assert.match(checkPackagedManifest("urollup", MANIFEST.replace(/^description = .*\n/m, ""), { version: "0.1.0", files }).join("\n"), /description is missing/);
});

const LOCK = `version = 4

[[package]]
name = "clap"
version = "4.6.6"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "${"a".repeat(64)}"

[[package]]
name = "urollup"
version = "0.1.0"
dependencies = [
 "clap",
 "urollup-core",
]

[[package]]
name = "urollup-core"
version = "0.1.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "${"b".repeat(64)}"
dependencies = [
 "memchr",
]
`;

test("the patched install keeps every other locked version", () => {
  const rewritten = pathifyLock(LOCK, "urollup-core");
  assert.match(rewritten, /name = "urollup-core"\nversion = "0\.1\.0"\ndependencies = \[/);
  assert.ok(!rewritten.includes("b".repeat(64)));
  assert.ok(rewritten.includes(`checksum = "${"a".repeat(64)}"`), "other packages keep their registry source and checksum");
  assert.throws(() => pathifyLock(LOCK, "urollup-cli"), /0 entries for urollup-cli/);
});

test("a .crate extracts only below its own name and version", () => {
  const dir = mkdtempSync(path.join(tmpdir(), "urollup-crate-test-"));
  try {
    const good = makeTarGz([
      { name: "urollup-0.1.0/Cargo.toml", data: Buffer.from("[package]\n") },
      { name: "urollup-0.1.0/src/main.rs", data: Buffer.from("fn main() {}\n") },
    ]);
    assert.deepEqual(extractCrate(good, "urollup-0.1.0", dir), ["Cargo.toml", "src/main.rs"]);
    assert.equal(readFileSync(path.join(dir, "urollup-0.1.0", "src", "main.rs"), "utf8"), "fn main() {}\n");
    assert.throws(() => extractCrate(makeTarGz([{ name: "urollup-0.1.0/../escape", data: Buffer.from("x") }]), "urollup-0.1.0", dir), /outside urollup-0\.1\.0/);
    assert.throws(() => extractCrate(makeTarGz([{ name: "other-0.1.0/x", data: Buffer.from("x") }]), "urollup-0.1.0", dir), /outside/);
    assert.throws(() => extractCrate(makeTarGz([{ name: "urollup-0.1.0/link", data: Buffer.alloc(0), type: "2" }]), "urollup-0.1.0", dir), /is a symlink/);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

const CARGO = spawnSync("cargo", ["--version"], { encoding: "utf8" });

test("with --no-verify, Cargo runs no build script, so none can read the crates.io token", { skip: (CARGO.error || CARGO.status !== 0) && "cargo is not available" }, () => {
  const dir = mkdtempSync(path.join(tmpdir(), "urollup-token-probe-"));
  try {
    mkdirSync(path.join(dir, "src"));
    writeFileSync(
      path.join(dir, "Cargo.toml"),
      '[package]\nname = "token-probe"\nversion = "0.1.0"\nedition = "2021"\nlicense = "MIT"\ndescription = "probe"\nbuild = "build.rs"\n\n[workspace]\n',
    );
    writeFileSync(path.join(dir, "src", "main.rs"), "fn main() {}\n");
    writeFileSync(
      path.join(dir, "build.rs"),
      'fn main() { let seen = std::env::var("CARGO_REGISTRY_TOKEN").is_ok(); std::fs::write(std::env::var("PROBE_MARKER").unwrap(), if seen { "token" } else { "no token" }).unwrap(); }\n',
    );
    const marker = path.join(dir, "marker");
    const env = { ...process.env, CARGO_REGISTRY_TOKEN: "probe-token", PROBE_MARKER: marker, CARGO_TARGET_DIR: path.join(dir, "target") };
    const cargo = (args) => spawnSync("cargo", args, { cwd: dir, env, encoding: "utf8" });
    const noVerify = cargo(["package", "--no-verify", "--offline", "--allow-dirty"]);
    assert.equal(noVerify.status, 0, noVerify.stderr);
    assert.equal(existsSync(marker), false, "cargo package --no-verify ran a build script");
    // The control: verification does run the build script, which then sees the token.
    const verified = cargo(["package", "--offline", "--allow-dirty"]);
    assert.equal(verified.status, 0, verified.stderr);
    assert.equal(readFileSync(marker, "utf8"), "token");
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});
