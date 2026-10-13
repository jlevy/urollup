import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

import {
  AUTHORITY_VARIABLES,
  COMMANDS,
  LATE_ASSETS,
  authorityProblems,
  dispatchArgs,
  hostId,
  manylinuxCommand,
  maturinPinProblems,
  parseOptions,
  pinnedToolchain,
  publicationGuards,
  resolveTagCommit,
} from "./release.mjs";
import { artifactById, buildEnvironment } from "./release-targets.mjs";
import { makeReleaseBundle } from "./release-test-binaries.mjs";

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
  assert.match(run.stderr, /must hold no credential or OIDC endpoint.*ACTIONS_ID_TOKEN_REQUEST_URL/);
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

test("the publish subcommands refuse a rehearsal bundle, a branch run and a tampered bundle", () => {
  const bundle = makeReleaseBundle({ commit: "0".repeat(40), mode: "rehearsal" });
  try {
    for (const command of ["stage-github", "publish-crates", "publish-pypi", "publish-github"]) {
      const run = spawnSync(process.execPath, [SCRIPT, command, "--bundle", bundle.dir], {
        env: { PATH: process.env.PATH, GITHUB_REF: "refs/tags/v0.1.0", RELEASE_CRATES_TOKEN: "x" },
        encoding: "utf8",
      });
      assert.equal(run.status, 1, command);
      assert.match(run.stderr, /publication runs only from the pushed tag v0\.1\.0/, command);
    }
    writeFileSync(path.join(bundle.dir, "SHA256SUMS"), `${"f".repeat(64)}  urollup-0.1.0-x86_64-unknown-linux-musl.tar.gz\n`);
    const tampered = spawnSync(process.execPath, [SCRIPT, "stage-github", "--bundle", bundle.dir], { env: { PATH: process.env.PATH, GITHUB_REF: "refs/tags/v0.1.0" }, encoding: "utf8" });
    assert.match(tampered.stderr, /release bundle is inconsistent/);
  } finally {
    bundle.cleanup();
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

function fakePlanContext({ annotated = true, head = "c".repeat(40), tagCommit = "c".repeat(40), root = process.cwd() } = {}) {
  const calls = [];
  return {
    calls,
    ctx: {
      env: { PATH: process.env.PATH },
      root,
      run: (program, args) => {
        calls.push([program, ...args]);
        if (args[0] === "ls-remote") {
          return { status: 0, stdout: annotated ? `${"a".repeat(40)}\trefs/tags/v0.1.0\n${tagCommit}\trefs/tags/v0.1.0^{}\n` : `${tagCommit}\trefs/tags/v0.1.0\n`, stderr: "" };
        }
        if (args[0] === "rev-parse") {
          return { status: 0, stdout: `${head}\n`, stderr: "" };
        }
        return { status: 0, stdout: "", stderr: "" };
      },
      fetchImpl: async () => ({ ok: false, status: 404, json: async () => ({}) }),
    },
  };
}

test("a tag run releases the peeled tag commit, never an unpeeled GITHUB_SHA", () => {
  assert.equal(resolveTagCommit(fakePlanContext().ctx, "v0.1.0"), "c".repeat(40));
  assert.throws(() => resolveTagCommit(fakePlanContext({ annotated: false }).ctx, "v0.1.0"), /lightweight tag/);
  assert.throws(() => resolveTagCommit(fakePlanContext({ head: "d".repeat(40) }).ctx, "v0.1.0"), /checkout is at d+, but v0\.1\.0 names c+/);
});

test("a tag run refuses a missing signers file and unconfigured settings; a rehearsal only reports them", async () => {
  const root = mkdtempSync(path.join(tmpdir(), "urollup-guards-"));
  try {
    const publish = await publicationGuards(fakePlanContext({ root }).ctx, { tag: "v0.1.0", repository: "jlevy/urollup", token: "read", publish: true });
    assert.match(publish.problems.join("\n"), /release-allowed-signers does not exist/);
    assert.match(publish.problems.join("\n"), /release environment does not exist/);
    const planContext = fakePlanContext({ root });
    const rehearsal = await publicationGuards(planContext.ctx, { tag: "v0.1.0", repository: "jlevy/urollup", token: "read", publish: false });
    assert.deepEqual(rehearsal.problems, []);
    assert.equal(rehearsal.facts.signers_file, "missing");
    assert.match(rehearsal.facts.settings_problems.join("\n"), /release environment does not exist/);
    assert.ok(!planContext.calls.some((call) => call.includes("verify-tag") || call.includes("fetch")), "a rehearsal fetches and verifies no tag");
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});
