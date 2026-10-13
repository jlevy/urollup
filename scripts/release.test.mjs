import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

import { AUTHORITY_VARIABLES, COMMANDS, LATE_ASSETS, authorityProblems, dispatchArgs, hostId, manylinuxCommand, maturinPinProblems, parseOptions, pinnedToolchain } from "./release.mjs";
import { artifactById, buildEnvironment } from "./release-targets.mjs";

const SCRIPT = path.join(path.dirname(fileURLToPath(import.meta.url)), "release.mjs");

test("options are --name value pairs from an allowlist", () => {
  assert.deepEqual(parseOptions(["--artifact", "wheel-linux-x86_64", "--out", "/tmp/x"], ["artifact", "out"]), { artifact: "wheel-linux-x86_64", out: "/tmp/x" });
  assert.deepEqual(parseOptions(["--allow-unmerged", "true"], ["allow-unmerged"]), { allowUnmerged: "true" });
  assert.throws(() => parseOptions(["--publish", "yes"], ["out"]), /unknown option --publish/);
  assert.throws(() => parseOptions(["--out"], ["out"]), /--out needs a value/);
  assert.throws(() => parseOptions(["--out", "--artifact"], ["out", "artifact"]), /--out needs a value/);
});

test("host ids name the release hosts", () => {
  assert.equal(hostId("linux", "x64"), "linux-x86_64");
  assert.equal(hostId("linux", "arm64"), "linux-aarch64");
  assert.equal(hostId("darwin", "arm64"), "macos-aarch64");
  assert.equal(hostId("darwin", "x64"), "macos-x86_64");
  assert.equal(hostId("win32", "x64"), "windows-x86_64");
});

test("a build, validation or assembly job may hold no OIDC endpoint and no credential", () => {
  assert.deepEqual(authorityProblems({ PATH: "/usr/bin", ACTIONS_RUNTIME_TOKEN: "artifact-only", RUNNER_TEMP: "/t" }), []);
  for (const name of AUTHORITY_VARIABLES) {
    assert.deepEqual(authorityProblems({ [name]: "x" }), [name]);
  }
  assert.deepEqual(authorityProblems({ ACTIONS_ID_TOKEN_REQUEST_URL: "" }), [], "an empty variable grants nothing");
  assert.deepEqual(authorityProblems({ CARGO_REGISTRIES_MIRROR_TOKEN: "x" }), ["CARGO_REGISTRIES_MIRROR_TOKEN"]);
  const run = spawnSync(process.execPath, [SCRIPT, "assert-no-authority"], { env: { PATH: process.env.PATH, ACTIONS_ID_TOKEN_REQUEST_URL: "https://token.invalid" }, encoding: "utf8" });
  assert.equal(run.status, 1);
  assert.match(run.stderr, /must hold no publishing authority.*ACTIONS_ID_TOKEN_REQUEST_URL/);
});

test("the manylinux build runs the locked Maturin offline in the pinned image", () => {
  const artifact = artifactById("0.1.0", "wheel-linux-aarch64");
  const env = buildEnvironment(artifact, { cargoHome: "/home/runner/.cargo" });
  const args = manylinuxCommand({
    artifact,
    root: "/home/runner/work/urollup/urollup",
    cargoHomeDir: "/home/runner/.cargo",
    sysroot: "/home/runner/.rustup/toolchains/1.98.0-aarch64-unknown-linux-gnu",
    maturin: "/home/runner/work/urollup/urollup/.venv/bin/maturin",
    uid: 1001,
    gid: 118,
    outDir: "/home/runner/work/urollup/urollup/target/wheel-out",
    env,
  });
  assert.equal(args[0], "run");
  assert.ok(args.includes(artifact.container));
  assert.ok(args.includes("CARGO_NET_OFFLINE=true"));
  assert.ok(args.includes("/home/runner/.rustup/toolchains/1.98.0-aarch64-unknown-linux-gnu:/opt/rust:ro"));
  assert.ok(args.includes("1001:118"), "the container writes files as the runner user");
  assert.ok(args.includes("CARGO_ENCODED_RUSTFLAGS=--remap-path-prefix=/cargo=/cargo"), "registry paths read /cargo inside and outside the container");
  const tail = args.slice(args.indexOf("bash"));
  assert.deepEqual(tail, [
    "bash",
    "/io/.venv/bin/maturin",
    "build",
    "--release",
    "--locked",
    "--manifest-path",
    "crates/urollup/Cargo.toml",
    "--target",
    "aarch64-unknown-linux-gnu",
    "--compatibility",
    "manylinux2014",
    "--auditwheel",
    "check",
    "--out",
    "/io/target/wheel-out",
  ]);
});

test("the pinned toolchain must support workspace publication", () => {
  assert.equal(pinnedToolchain('[toolchain]\nchannel = "1.98.0"\n'), "1.98.0");
  assert.throws(() => pinnedToolchain('[toolchain]\nchannel = "1.89.0"\n'), /older than 1\.90/);
  assert.throws(() => pinnedToolchain('[toolchain]\nchannel = "stable"\n'), /exact channel/);
});

test("every subcommand the workflow calls exists", () => {
  for (const command of ["plan", "gate-git", "assert-no-authority", "build", "crates", "smoke", "benchmark", "assemble", "stage-github", "publish-crates", "publish-pypi", "publish-github", "probe", "evidence", "attach-evidence", "check"]) {
    assert.equal(typeof COMMANDS[command], "function", command);
  }
  assert.deepEqual(LATE_ASSETS, ["release-evidence.json"]);
  const run = spawnSync(process.execPath, [SCRIPT, "publish-everything"], { encoding: "utf8" });
  assert.equal(run.status, 1);
  assert.match(run.stderr, /usage: release\.mjs/);
});

test("publication refuses a rehearsal bundle and any run that is not its tag", () => {
  const bundle = mkdtempSync(path.join(tmpdir(), "urollup-bundle-test-"));
  try {
    mkdirSync(path.join(bundle, "assets"));
    const data = Buffer.from("archive");
    writeFileSync(path.join(bundle, "assets", "urollup-0.1.0-x86_64-unknown-linux-musl.tar.gz"), data);
    writeFileSync(path.join(bundle, "SHA256SUMS"), `${createHash("sha256").update(data).digest("hex")}  urollup-0.1.0-x86_64-unknown-linux-musl.tar.gz\n`);
    writeFileSync(path.join(bundle, "release-notes.md"), "notes\n");
    const manifest = { mode: "rehearsal", version: "0.1.0", tag: "v0.1.0", commit: "0".repeat(40), artifacts: [], cargo_packages: [] };
    writeFileSync(path.join(bundle, "release-manifest.json"), JSON.stringify(manifest));
    for (const command of ["stage-github", "publish-crates", "publish-pypi", "publish-github"]) {
      const run = spawnSync(process.execPath, [SCRIPT, command, "--bundle", bundle], { env: { PATH: process.env.PATH, GITHUB_REF: "refs/tags/v0.1.0" }, encoding: "utf8" });
      assert.equal(run.status, 1, command);
      assert.match(run.stderr, /publication runs only from the pushed tag v0\.1\.0/, command);
    }
    writeFileSync(path.join(bundle, "release-manifest.json"), JSON.stringify({ ...manifest, mode: "publish" }));
    const branch = spawnSync(process.execPath, [SCRIPT, "stage-github", "--bundle", bundle], { env: { PATH: process.env.PATH, GITHUB_REF: "refs/heads/main" }, encoding: "utf8" });
    assert.match(branch.stderr, /publication runs only from the pushed tag/);
    writeFileSync(path.join(bundle, "SHA256SUMS"), `${"f".repeat(64)}  urollup-0.1.0-x86_64-unknown-linux-musl.tar.gz\n`);
    const tampered = spawnSync(process.execPath, [SCRIPT, "stage-github", "--bundle", bundle], { env: { PATH: process.env.PATH, GITHUB_REF: "refs/tags/v0.1.0" }, encoding: "utf8" });
    assert.match(tampered.stderr, /does not match its SHA256SUMS:\nchecksum conflict/);
  } finally {
    rmSync(bundle, { recursive: true, force: true });
  }
});

test("the wheel backend and the release jobs use one Maturin", () => {
  const backend = '[build-system]\nrequires = ["maturin==1.15.0"]\n';
  assert.deepEqual(maturinPinProblems(backend, 'dev = []\nrelease = ["maturin==1.15.0"]\n'), []);
  assert.match(maturinPinProblems(backend, 'release = ["maturin==1.14.1"]\n').join("\n"), /builds with maturin 1\.15\.0, but the release group locks 1\.14\.1/);
  assert.match(maturinPinProblems('requires = ["maturin>=1"]', 'release = ["maturin==1.15.0"]\n').join("\n"), /\(unpinned\)/);
});

test("a CI rehearsal is dispatched for the pushed commit and the workspace version", () => {
  assert.deepEqual(dispatchArgs({ branch: "feat/x", commit: "c".repeat(40), version: "0.1.0", allowUnmerged: "true" }), [
    "workflow",
    "run",
    "release.yml",
    "--ref",
    "feat/x",
    "-f",
    `commit=${"c".repeat(40)}`,
    "-f",
    "version=0.1.0",
    "-f",
    "allow-unmerged=true",
  ]);
  assert.equal(dispatchArgs({ branch: "main", commit: "c", version: "0.1.0", allowUnmerged: "false" }).at(-1), "allow-unmerged=false");
});
