import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
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
import { writeTarGz } from "./release-archive.mjs";
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
  for (const renamed of ["RELEASE_CRATES_TOKEN", "RELEASE_READ_TOKEN", "RELEASE_WRITE_TOKEN"]) {
    assert.deepEqual(authorityProblems({ [renamed]: "x" }), [renamed], `${renamed} is a credential to assert-no-authority too`);
  }
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
    sourceDateEpoch: 1791912228,
  });
  assert.ok(args.includes("SOURCE_DATE_EPOCH=1791912228"), "Maturin stamps wheel members with the commit time");
  for (const bad of [undefined, "1791912228", 1.5, 0, Number.NaN]) {
    assert.throws(() => manylinuxCommand({ artifact, root: "/r", cargoHomeDir: "/c", sysroot: "/s", maturin: "/r/m", uid: 1, gid: 1, outDir: "/r/o", env, sourceDateEpoch: bad }), /SOURCE_DATE_EPOCH must be whole seconds/);
  }
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

test("validation never skips the member-time check, and records the time it checked against", () => {
  // Review C8: a validate job that lost the plan's SOURCE_DATE_EPOCH fails instead of
  // skipping the check, and the record says which time every member was compared with.
  const dir = mkdtempSync(path.join(tmpdir(), "urollup-smoke-time-"));
  try {
    const artifact = artifactById("0.1.0", "archive-linux-x86_64");
    const input = path.join(dir, "in");
    mkdirSync(input);
    writeFileSync(path.join(input, artifact.name), writeTarGz([{ name: "urollup", data: Buffer.from("not an executable"), mode: 0o755 }], { mtime: 1791919744 }));
    const validate = (env) => {
      const out = path.join(dir, `out-${env.SOURCE_DATE_EPOCH ?? "none"}`);
      const run = spawnSync(process.execPath, [SCRIPT, "smoke", "--artifact", artifact.id, "--input", input, "--out", out], { env: { PATH: process.env.PATH, ...env }, encoding: "utf8" });
      assert.equal(run.status, 1, "the fake archive never passes validation");
      return JSON.parse(readFileSync(path.join(out, `validation-${artifact.id}.json`), "utf8"));
    };
    assert.match(validate({}).problems.join("\n"), /SOURCE_DATE_EPOCH is not set, so the member-time check cannot run/);
    assert.match(validate({ SOURCE_DATE_EPOCH: "" }).problems.join("\n"), /SOURCE_DATE_EPOCH is not set/);
    const checked = validate({ SOURCE_DATE_EPOCH: "1791919744" });
    assert.equal(checked.facts.member_time, 1791919744);
    assert.doesNotMatch(checked.problems.join("\n"), /not stamped with the commit time/);
    assert.match(validate({ SOURCE_DATE_EPOCH: "1791919000" }).problems.join("\n"), /urollup not stamped with the commit time 1791919000/);
  } finally {
    rmSync(dir, { recursive: true, force: true });
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
        if (args[0] === "rev-parse" && args.at(-1) === "refs/tags/v0.1.0") {
          return { status: 0, stdout: `${"a".repeat(40)}\n`, stderr: "" };
        }
        if (args[0] === "rev-parse" && args.at(-1).endsWith("^{commit}")) {
          return { status: 0, stdout: `${tagCommit}\n`, stderr: "" };
        }
        if (args[0] === "rev-parse") {
          return { status: 0, stdout: `${head}\n`, stderr: "" };
        }
        if (args[0] === "cat-file" && args[1] === "-t") {
          return { status: 0, stdout: "tag\n", stderr: "" };
        }
        if (args[0] === "cat-file") {
          return { status: 0, stdout: `object ${tagCommit}\ntype commit\ntag v0.1.0\ntagger T <t@example.invalid> 1791919744 -0700\n\nurollup 0.1.0\n`, stderr: "" };
        }
        if (args.includes("verify-tag")) {
          return { status: 0, stdout: "", stderr: 'Good "git" signature for urollup-release with ED25519 key SHA256:test\n' };
        }
        return { status: 0, stdout: "", stderr: "" };
      },
      fetchImpl: async () => ({ ok: false, status: 404, json: async () => ({}) }),
    },
  };
}

test("a tag run releases the peeled tag commit, never an unpeeled GITHUB_SHA", () => {
  assert.deepEqual(resolveTagCommit(fakePlanContext().ctx, "v0.1.0"), { object: "a".repeat(40), commit: "c".repeat(40), annotated: true });
  assert.throws(() => resolveTagCommit(fakePlanContext({ annotated: false }).ctx, "v0.1.0"), /lightweight tag/);
  assert.throws(() => resolveTagCommit(fakePlanContext({ head: "d".repeat(40) }).ctx, "v0.1.0"), /checkout is at d+, but v0\.1\.0 names c+/);
});

test("a tag run refuses a missing signers file and unconfigured settings; a rehearsal only reports them", async () => {
  const root = mkdtempSync(path.join(tmpdir(), "urollup-guards-"));
  try {
    const peeled = { object: "a".repeat(40), commit: "c".repeat(40), annotated: true };
    const publish = await publicationGuards(fakePlanContext({ root }).ctx, { tag: "v0.1.0", peeled, repository: "jlevy/urollup", token: "read", publish: true });
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

test("a tag run records the verified tag object and its time for the approver, and how settings were read", async () => {
  const root = mkdtempSync(path.join(tmpdir(), "urollup-guards-"));
  try {
    mkdirSync(path.join(root, ".github"));
    writeFileSync(path.join(root, ".github", "release-allowed-signers"), "urollup-release ssh-ed25519 AAAA\n");
    const peeled = { object: "a".repeat(40), commit: "c".repeat(40), annotated: true };
    const planContext = fakePlanContext({ root });
    const guards = await publicationGuards(planContext.ctx, { tag: "v0.1.0", peeled, repository: "jlevy/urollup", token: "read", publish: true });
    assert.equal(guards.facts.tag_object, "a".repeat(40));
    assert.equal(guards.facts.tagger_date, "2026-10-13T19:29:04Z");
    assert.match(guards.facts.signature, /Good "git" signature for urollup-release/);
    const verify = planContext.calls.find((call) => call.includes("verify-tag"));
    assert.equal(verify.at(-1), "a".repeat(40), "the signature check names the object ID, never the tag name");
    for (const call of planContext.calls.filter((candidate) => candidate[1] === "cat-file" || (candidate[1] === "rev-parse" && candidate.at(-1).endsWith("^{commit}")))) {
      assert.ok(call.at(-1).startsWith("a".repeat(40)), `${call.join(" ")} must name the verified object`);
    }
    assert.deepEqual(guards.facts.settings_reads, [
      { what: "environment", via: "job token" },
      { what: "rulesets", via: "job token" },
    ]);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});
