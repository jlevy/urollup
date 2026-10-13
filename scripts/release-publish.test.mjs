// Tests of the tag-only release path against fake registries, a fake GitHub API and fake
// processes. A rehearsal never runs these jobs, so these tests are their only exercise
// before the first tag.

import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import test from "node:test";

import {
  RELEASE_WORKFLOW,
  attachEvidence,
  attestationVerifyArgs,
  evidence,
  judgeReleaseSettings,
  loadBundle,
  observeReleaseSettings,
  parseLsRemote,
  probe,
  publishCrates,
  publishGithub,
  publishPypi,
  requirePublishRun,
  stageGithub,
  unverifiableSettings,
  verifyLocalReleaseTag,
  verifyReleaseTag,
  verifyTagSignature,
  verifyTagTarget,
  waitForPypiFiles,
  withoutCredentials,
} from "./release-publish.mjs";
import { SMOKE_CASES } from "./release-smoke.mjs";
import { fixtureReportFor, makeReleaseBundle } from "./release-test-binaries.mjs";
import { RELEASE_TARGETS } from "./release-targets.mjs";

const COMMIT = "c".repeat(40);
const TAG_OBJECT = "a".repeat(40);
const OTHER = "d".repeat(40);
const REPOSITORY = "jlevy/urollup";
const sha256 = (data) => createHash("sha256").update(data).digest("hex");
const makeBundle = (options) => makeReleaseBundle({ commit: COMMIT, ...options });

// ---------------------------------------------------------------------------------------
// Fixtures

function response(status, body) {
  const bytes = Buffer.isBuffer(body) ? body : Buffer.from(typeof body === "string" ? body : JSON.stringify(body ?? {}));
  return {
    ok: status >= 200 && status < 300,
    status,
    json: async () => JSON.parse(bytes.toString("utf8")),
    arrayBuffer: async () => bytes.buffer.slice(bytes.byteOffset, bytes.byteOffset + bytes.length),
  };
}

/**
 * A fake world: crates.io, PyPI and GitHub state plus a git remote whose tag can move. Each
 * fetch and process call is recorded with its environment.
 */
function fakeWorld({ bundle, crates = {}, pypi = null, pypiProject = false, crateProjects = {}, release = null, tagCommit = COMMIT, annotated = true, run = () => null } = {}) {
  const state = { crates: { ...crates }, pypi, release, tagCommit, uploads: [], patches: [], deletes: [], pypiIndexCalls: 0 };
  const calls = [];
  const fetches = [];
  const fetchImpl = async (url, options = {}) => {
    const method = options.method ?? "GET";
    fetches.push({ url, method, headers: options.headers ?? {} });
    let match;
    if ((match = url.match(/^https:\/\/crates\.io\/api\/v1\/crates\/([^/]+)\/([^/]+)$/))) {
      const crate = state.crates[decodeURIComponent(match[1])];
      return crate ? response(200, { version: { num: "0.1.0", checksum: crate.checksum, yanked: false } }) : response(404);
    }
    if ((match = url.match(/^https:\/\/crates\.io\/api\/v1\/crates\/([^/]+)$/))) {
      return crateProjects[decodeURIComponent(match[1])] || state.crates[decodeURIComponent(match[1])] ? response(200, {}) : response(404);
    }
    if (url === "https://pypi.org/pypi/urollup/0.1.0/json") {
      return state.pypi ? response(200, { urls: state.pypi.map((file) => ({ filename: file.name, digests: { sha256: file.sha256 }, packagetype: "bdist_wheel", yanked: false })) }) : response(404);
    }
    if (url === "https://pypi.org/pypi/urollup/json") {
      return pypiProject || state.pypi ? response(200, {}) : response(404);
    }
    if (url === "https://pypi.org/simple/urollup/") {
      state.pypiIndexCalls += 1;
      const files = state.pypiIndexCalls > 1 && state.pypi ? state.pypi.map((file) => ({ filename: file.name })) : [];
      return response(200, { files });
    }
    if (url.startsWith(`https://api.github.com/repos/${REPOSITORY}/releases?per_page=100`)) {
      return response(200, state.release ? [state.release] : []);
    }
    if (url === `https://api.github.com/repos/${REPOSITORY}/releases/tags/v0.1.0`) {
      return state.release && !state.release.draft ? response(200, state.release) : response(404);
    }
    if (method === "POST" && url === `https://api.github.com/repos/${REPOSITORY}/releases`) {
      const body = JSON.parse(options.body);
      state.release = { id: 9, ...body, assets: [] };
      return response(201, state.release);
    }
    if (method === "POST" && url.startsWith(`https://uploads.github.com/repos/${REPOSITORY}/releases/9/assets?name=`)) {
      const name = decodeURIComponent(new URL(url).searchParams.get("name"));
      state.uploads.push(name);
      // GitHub may report the digest a moment after the upload: the first read misses it.
      state.release.assets.push({ id: state.uploads.length, name, state: "uploaded", digest: null, pendingDigest: `sha256:${sha256(options.body)}` });
      return response(201, {});
    }
    if (method === "PATCH" && url === `https://api.github.com/repos/${REPOSITORY}/releases/9`) {
      const body = JSON.parse(options.body);
      state.patches.push(body);
      Object.assign(state.release, body);
      return response(200, state.release);
    }
    if (url.startsWith(`https://github.com/${REPOSITORY}/releases/download/v0.1.0/`)) {
      const name = url.split("/").pop();
      const file = name === "SHA256SUMS" ? path.join(bundle.dir, "SHA256SUMS") : path.join(bundle.dir, "assets", name);
      return existsSync(file) ? response(200, readFileSync(file)) : response(404);
    }
    return response(404);
  };
  // Every observation after an upload reveals the pending digests, as GitHub's would.
  const settle = () => {
    for (const asset of state.release?.assets ?? []) {
      if (asset.pendingDigest && asset.digest === null && asset.seen) {
        asset.digest = asset.pendingDigest;
      }
      asset.seen = true;
    }
  };
  const observingFetch = async (url, options) => {
    if (url.includes("/releases?per_page=100")) {
      settle();
    }
    return fetchImpl(url, options);
  };
  const ctx = {
    env: { GITHUB_REF: "refs/tags/v0.1.0", GITHUB_REPOSITORY: REPOSITORY, PATH: process.env.PATH, RELEASE_WRITE_TOKEN: "write-token", RELEASE_CRATES_TOKEN: "crates-token", RELEASE_READ_TOKEN: "read-token" },
    run: (program, args, options = {}) => {
      calls.push({ program, args, env: options.env ?? {} });
      const custom = run(program, args, options, state);
      if (custom) {
        return custom;
      }
      if (program === "git" && args[0] === "ls-remote") {
        return { status: 0, stdout: annotated ? `${TAG_OBJECT}\trefs/tags/v0.1.0\n${state.tagCommit}\trefs/tags/v0.1.0^{}\n` : `${state.tagCommit}\trefs/tags/v0.1.0\n`, stderr: "" };
      }
      if (program === "git" && args[0] === "rev-parse") {
        return { status: 0, stdout: `${COMMIT}\n`, stderr: "" };
      }
      return { status: 0, stdout: "", stderr: "" };
    },
    fetchImpl: observingFetch,
    sleep: async () => {},
    smoke: ({ label }) => ({ checks: [{ check: `smoke ${label}` }], problems: [] }),
    log: () => {},
    root: process.cwd(),
    tmpdir: tmpdir(),
    host: "linux-x86_64",
    workflowRun: { run_id: "1" },
  };
  return { ctx, state, calls, fetches };
}

/** A fake `cargo package` that writes the bundle's exact .crate bytes into its target dir. */
function packagesRehearsedBytes(bundle, { tamper = false } = {}) {
  return (program, args, options) => {
    if (program === "cargo" && args[0] === "package") {
      const dir = path.join(options.env.CARGO_TARGET_DIR, "package");
      mkdirSync(dir, { recursive: true });
      for (const item of bundle.cargo) {
        writeFileSync(path.join(dir, item.file), tamper ? Buffer.from("different bytes") : readFileSync(path.join(bundle.dir, "crates", item.file)));
      }
      return { status: 0, stdout: "", stderr: "" };
    }
    return null;
  };
}

// ---------------------------------------------------------------------------------------
// The tag

test("ls-remote output is peeled, and only an annotated tag naming the commit passes", () => {
  assert.deepEqual(parseLsRemote(`${TAG_OBJECT}\trefs/tags/v0.1.0\n${COMMIT}\trefs/tags/v0.1.0^{}\n`, "v0.1.0"), { object: TAG_OBJECT, commit: COMMIT, annotated: true });
  assert.deepEqual(parseLsRemote(`${COMMIT}\trefs/tags/v0.1.0\n`, "v0.1.0"), { object: COMMIT, commit: COMMIT, annotated: false });
  assert.equal(parseLsRemote(`${COMMIT}\trefs/tags/v0.1.0-rc.1\n`, "v0.1.0"), null);
  const bundle = makeBundle();
  try {
    assert.equal(verifyTagTarget(fakeWorld({ bundle }).ctx, { tag: "v0.1.0", commit: COMMIT }).commit, COMMIT);
    assert.throws(() => verifyTagTarget(fakeWorld({ bundle, tagCommit: OTHER }).ctx, { tag: "v0.1.0", commit: COMMIT }), /now names d+, not the release commit/);
    assert.throws(() => verifyTagTarget(fakeWorld({ bundle, annotated: false }).ctx, { tag: "v0.1.0", commit: COMMIT }), /lightweight tag/);
  } finally {
    bundle.cleanup();
  }
});

const SSH_KEYGEN = process.platform !== "win32" && spawnSync("ssh-keygen", ["-?"], { encoding: "utf8" }).error === undefined;

test("a release tag must carry a signature from an allowed signer", { skip: !SSH_KEYGEN && "ssh-keygen is not available" }, () => {
  const dir = mkdtempSync(path.join(tmpdir(), "urollup-signed-tag-"));
  try {
    const git = (...args) => {
      const result = spawnSync("git", ["-c", "user.name=Release Test", "-c", "user.email=release@example.invalid", ...args], { cwd: dir, encoding: "utf8" });
      assert.equal(result.status, 0, result.stderr);
      return result.stdout;
    };
    for (const name of ["allowed", "other"]) {
      const made = spawnSync("ssh-keygen", ["-q", "-t", "ed25519", "-N", "", "-C", name, "-f", path.join(dir, name)], { encoding: "utf8" });
      assert.equal(made.status, 0, made.stderr);
    }
    git("init", "-q");
    writeFileSync(path.join(dir, "file"), "x\n");
    git("add", "file");
    git("commit", "-q", "-m", "release");
    git("-c", "gpg.format=ssh", "-c", `user.signingkey=${path.join(dir, "allowed.pub")}`, "tag", "-s", "v0.1.0", "-m", "urollup 0.1.0");
    git("-c", "gpg.format=ssh", "-c", `user.signingkey=${path.join(dir, "other.pub")}`, "tag", "-s", "v0.1.1", "-m", "urollup 0.1.1");
    git("tag", "-a", "v0.1.2", "-m", "unsigned");
    const signers = path.join(dir, "allowed_signers");
    writeFileSync(signers, `urollup-release namespaces="git" ${readFileSync(path.join(dir, "allowed.pub"), "utf8")}`);
    const ctx = { env: { PATH: process.env.PATH, HOME: process.env.HOME }, run: (program, args, options) => spawnSync(program, args, { ...options, encoding: "utf8" }), root: dir };
    assert.match(verifyTagSignature(ctx, { tag: "v0.1.0", signersFile: signers }), /Good "git" signature for urollup-release/);
    assert.throws(() => verifyTagSignature(ctx, { tag: "v0.1.1", signersFile: signers }), /does not carry a signature from an allowed release signer/);
    assert.throws(() => verifyTagSignature(ctx, { tag: "v0.1.2", signersFile: signers }), /does not carry a signature from an allowed release signer/);
    assert.throws(() => verifyTagSignature(ctx, { tag: "v0.1.0", signersFile: path.join(dir, "missing") }), /does not exist; add the maintainer's SSH signing key/);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

/** A git repository with a release commit, signing keys and an allowed-signers file. */
function signingRepository() {
  const dir = mkdtempSync(path.join(tmpdir(), "urollup-release-tag-"));
  const git = (...args) => {
    const result = spawnSync("git", ["-c", "user.name=Release Test", "-c", "user.email=release@example.invalid", ...args], { cwd: dir, encoding: "utf8" });
    assert.equal(result.status, 0, result.stderr);
    return result.stdout.trim();
  };
  for (const name of ["allowed", "other"]) {
    const made = spawnSync("ssh-keygen", ["-q", "-t", "ed25519", "-N", "", "-C", name, "-f", path.join(dir, name)], { encoding: "utf8" });
    assert.equal(made.status, 0, made.stderr);
  }
  const signersLine = `urollup-release namespaces="git" ${readFileSync(path.join(dir, "allowed.pub"), "utf8")}`;
  git("init", "-q");
  mkdirSync(path.join(dir, ".github"));
  writeFileSync(path.join(dir, ".github", "release-allowed-signers"), signersLine);
  writeFileSync(path.join(dir, "Cargo.toml"), '[workspace.package]\nversion = "0.1.0"\n');
  git("add", ".");
  git("commit", "-q", "-m", "release");
  const sign = (key, name, message = name) => git("-c", "gpg.format=ssh", "-c", `user.signingkey=${path.join(dir, `${key}.pub`)}`, "tag", "-s", name, "-m", message);
  const ctx = { env: { PATH: process.env.PATH, HOME: process.env.HOME }, run: (program, args, options) => spawnSync(program, args, { ...options, encoding: "utf8" }), root: dir, tmpdir: tmpdir() };
  return { dir, git, sign, ctx, signersFile: path.join(dir, ".github", "release-allowed-signers"), cleanup: () => rmSync(dir, { recursive: true, force: true }) };
}

test("the verified signature must be the release tag's own, on the release commit", { skip: !SSH_KEYGEN && "ssh-keygen is not available" }, () => {
  const repo = signingRepository();
  try {
    repo.sign("allowed", "v0.1.0", "urollup 0.1.0");
    const commit = repo.git("rev-parse", "HEAD");
    const object = repo.git("rev-parse", "refs/tags/v0.1.0");
    assert.match(verifyReleaseTag(repo.ctx, { tag: "v0.1.0", object, commit, signersFile: repo.signersFile }), /Good "git" signature for urollup-release/);
    assert.throws(() => verifyReleaseTag(repo.ctx, { tag: "v0.1.0", object: OTHER, commit, signersFile: repo.signersFile }), /is object .*, not the d+ the remote reported/);
    assert.throws(() => verifyReleaseTag(repo.ctx, { tag: "v0.1.0", object, commit: OTHER, signersFile: repo.signersFile }), /points at .*, not the release commit d+/);

    // Review B1's reproduction: a tag the maintainer signed for something else, re-pointed
    // as the release ref by someone who can push but has no key.
    repo.sign("allowed", "scratch-signing-test", "not a release");
    const scratch = repo.git("rev-parse", "refs/tags/scratch-signing-test");
    repo.git("update-ref", "refs/tags/v0.1.0", scratch);
    assert.match(verifyTagSignature(repo.ctx, { tag: "v0.1.0", signersFile: repo.signersFile }), /Good "git" signature/, "the signature alone still verifies");
    assert.throws(() => verifyReleaseTag(repo.ctx, { tag: "v0.1.0", object: scratch, commit, signersFile: repo.signersFile }), /was made for the tag "scratch-signing-test", not v0\.1\.0/);

    repo.git("tag", "-f", "v0.1.0");
    assert.throws(() => verifyReleaseTag(repo.ctx, { tag: "v0.1.0", object: commit, commit, signersFile: repo.signersFile }), /lightweight tag/);
  } finally {
    repo.cleanup();
  }
});

test("the maintainer can check a signed tag locally before pushing it", { skip: !SSH_KEYGEN && "ssh-keygen is not available" }, () => {
  const repo = signingRepository();
  const version = (text) => text.match(/version = "([^"]+)"/)[1];
  try {
    repo.sign("allowed", "v0.1.0", "urollup 0.1.0");
    const result = verifyLocalReleaseTag(repo.ctx, { tag: "v0.1.0", version });
    assert.equal(result.commit, repo.git("rev-parse", "HEAD"));
    assert.match(result.signature, /Good "git" signature/);
    assert.match(result.notes.join(""), /origin\/main is not fetched here/);
    repo.sign("other", "v0.1.0-other", "urollup 0.1.0");
    assert.throws(() => verifyLocalReleaseTag(repo.ctx, { tag: "v0.1.0-other", version }), /workspace version is not 0\.1\.0-other/);
    repo.git("tag", "-d", "v0.1.0");
    repo.sign("other", "v0.1.0", "urollup 0.1.0");
    assert.throws(() => verifyLocalReleaseTag(repo.ctx, { tag: "v0.1.0", version }), /does not carry a signature from an allowed release signer/);

    // With main fetched, the tagged commit must be on it.
    repo.git("tag", "-d", "v0.1.0");
    repo.sign("allowed", "v0.1.0", "urollup 0.1.0");
    repo.git("update-ref", "refs/remotes/origin/main", "HEAD");
    assert.deepEqual(verifyLocalReleaseTag(repo.ctx, { tag: "v0.1.0", version }).notes, []);
    repo.git("commit", "-q", "--allow-empty", "-m", "unmerged");
    repo.git("tag", "-d", "v0.1.0");
    repo.sign("allowed", "v0.1.0", "urollup 0.1.0");
    assert.throws(() => verifyLocalReleaseTag(repo.ctx, { tag: "v0.1.0", version }), /is not on origin\/main; fetch main, or tag a merged commit/);
  } finally {
    repo.cleanup();
  }
});

// ---------------------------------------------------------------------------------------
// Repository settings

const GOOD_SETTINGS = {
  environment: {
    status: "present",
    body: {
      name: "release",
      can_admins_bypass: false,
      protection_rules: [{ type: "required_reviewers", reviewers: [{ type: "User", reviewer: { id: 1 } }] }, { type: "branch_policy" }],
      deployment_branch_policy: { protected_branches: false, custom_branch_policies: true },
    },
  },
  policies: { status: "present", body: { branch_policies: [{ name: "v*", type: "tag" }] } },
  rulesets: {
    status: "present",
    list: [{ target: "tag", enforcement: "active", conditions: { ref_name: { include: ["refs/tags/v*"], exclude: [] } }, rules: [{ type: "creation" }, { type: "update" }, { type: "deletion" }] }],
  },
};

test("publication requires a protected release environment and a v* tag ruleset", () => {
  assert.deepEqual(judgeReleaseSettings(GOOD_SETTINGS), []);
  const change = (edit) => {
    const copy = structuredClone(GOOD_SETTINGS);
    edit(copy);
    return judgeReleaseSettings(copy).join("\n");
  };
  assert.match(change((copy) => (copy.environment = { status: "absent" })), /does not exist; GitHub would create it unprotected/);
  assert.match(change((copy) => (copy.environment.body.protection_rules = [])), /no required reviewer/);
  assert.match(change((copy) => (copy.environment.body.can_admins_bypass = true)), /administrators can bypass/);
  assert.deepEqual(change((copy) => delete copy.environment.body.can_admins_bypass), "", "a response without the field is not a refusal");
  assert.match(change((copy) => (copy.environment.body.deployment_branch_policy = null)), /deploy only from selected tags/);
  assert.match(change((copy) => (copy.policies.body.branch_policies = [{ name: "main", type: "branch" }])), /only deployment policy must be the tag pattern v\*, found branch main/);
  assert.match(change((copy) => copy.policies.body.branch_policies.push({ name: "*", type: "tag" })), /only deployment policy/);
  assert.match(change((copy) => (copy.rulesets.list[0].enforcement = "evaluate")), /no active tag ruleset/);
  assert.match(change((copy) => (copy.rulesets.list[0].rules = [{ type: "creation" }, { type: "deletion" }])), /blocks tag updates and deletions/);
  assert.match(change((copy) => (copy.rulesets.list[0].conditions.ref_name.include = ["refs/tags/*-rc*"])), /no active tag ruleset/);
  assert.match(change((copy) => (copy.rulesets = { status: "unavailable", reason: "HTTP 500" })), /rulesets could not be read/);
  // Review B2: exclusions and visible bypass lists.
  assert.match(change((copy) => (copy.rulesets.list[0].conditions.ref_name.exclude = ["refs/tags/v*"])), /with nothing excluded/);
  assert.match(change((copy) => (copy.rulesets.list[0].conditions.ref_name.exclude = ["refs/tags/v0.1.0"])), /with nothing excluded/);
  assert.match(
    change((copy) => (copy.rulesets.list[0].bypass_actors = [{ actor_type: "RepositoryRole", actor_id: 5, bypass_mode: "always" }])),
    /lets 1 actor\(s\) bypass it; that ruleset needs an empty bypass list/,
  );
  assert.deepEqual(change((copy) => (copy.rulesets.list[0].bypass_actors = [])), "");
  assert.deepEqual(change((copy) => (copy.rulesets.list[0].conditions.ref_name.include = ["~ALL"])), "", "a ruleset on every tag also guards v*");
  assert.deepEqual(
    change((copy) => copy.rulesets.list.unshift({ ...structuredClone(copy.rulesets.list[0]), bypass_actors: [{ actor_type: "RepositoryRole", actor_id: 5, bypass_mode: "always" }] })),
    "",
    "a separate creation ruleset may carry a bypass while the immutability ruleset has none",
  );
  assert.match(change((copy) => (copy.rulesets.list[0].target = "branch")), /no active tag ruleset/);
});

test("what a read token cannot see is reported for the maintainer to confirm", () => {
  const visible = structuredClone(GOOD_SETTINGS);
  visible.rulesets.list[0].bypass_actors = [];
  assert.deepEqual(unverifiableSettings(visible), [], "an administrator's view shows the list");
  const hidden = structuredClone(GOOD_SETTINGS);
  hidden.rulesets.list[0].bypass_actors = null;
  assert.match(unverifiableSettings(hidden).join(""), /bypass list is hidden from the job token; confirm at setup that it is empty/);
  assert.deepEqual(judgeReleaseSettings(hidden), [], "a hidden list is not a refusal, only a note");
});

test("a job token refused with 403 falls back to an anonymous read of public settings", async () => {
  const seen = [];
  const ctx = {
    fetchImpl: async (url, options) => {
      seen.push(options.headers.Authorization ?? "anonymous");
      if (options.headers.Authorization) {
        return response(403, { message: "Resource not accessible by integration" });
      }
      return url.includes("/rulesets") ? response(200, []) : response(404);
    },
  };
  const settings = await observeReleaseSettings(ctx, { repository: REPOSITORY, token: "read" });
  assert.equal(settings.environment.status, "absent");
  assert.equal(settings.rulesets.status, "present");
  assert.deepEqual(seen, ["Bearer read", "anonymous", "Bearer read", "anonymous"]);
});

test("settings are read with the read token, and a missing environment is reported", async () => {
  const urls = [];
  const ctx = {
    fetchImpl: async (url, options) => {
      urls.push([url, options.headers.Authorization]);
      if (url.endsWith("/rulesets?includes_parents=true&per_page=100")) {
        return response(200, [{ id: 5, target: "tag" }, { id: 6, target: "branch" }]);
      }
      if (url.endsWith("/rulesets/5")) {
        return response(200, GOOD_SETTINGS.rulesets.list[0]);
      }
      return response(404);
    },
  };
  const settings = await observeReleaseSettings(ctx, { repository: REPOSITORY, token: "read" });
  assert.equal(settings.environment.status, "absent");
  assert.equal(settings.rulesets.list.length, 1);
  assert.match(judgeReleaseSettings(settings).join("\n"), /release environment does not exist/);
  assert.ok(urls.every(([, auth]) => auth === "Bearer read"));
  assert.ok(!urls.some(([url]) => url.endsWith("/rulesets/6")), "only tag rulesets are read in detail");
});

test("attestation verification is bound to this workflow, tag and commit", () => {
  assert.deepEqual(attestationVerifyArgs({ file: "a.tar.gz", repository: REPOSITORY, tag: "v0.1.0", commit: COMMIT }), [
    "attestation",
    "verify",
    "a.tar.gz",
    "--repo",
    REPOSITORY,
    "--signer-workflow",
    `${REPOSITORY}/${RELEASE_WORKFLOW}`,
    "--source-ref",
    "refs/tags/v0.1.0",
    "--source-digest",
    COMMIT,
    "--deny-self-hosted-runners",
  ]);
});

test("credentials and the OIDC endpoint are removed from a subprocess environment", () => {
  const clean = withoutCredentials({ PATH: "/bin", CARGO_REGISTRY_TOKEN: "x", RELEASE_CRATES_TOKEN: "x", GH_TOKEN: "x", ACTIONS_ID_TOKEN_REQUEST_URL: "x", CARGO_REGISTRIES_MIRROR_TOKEN: "x", HOME: "/h" });
  assert.deepEqual(clean, { PATH: "/bin", HOME: "/h" });
});

// ---------------------------------------------------------------------------------------
// The bundle and the run

test("a bundle must agree with itself before anything is promoted", () => {
  const bundle = makeBundle();
  try {
    assert.equal(loadBundle(bundle.dir).assets.length, 12);
    const manifestFile = path.join(bundle.dir, "release-manifest.json");
    const manifest = JSON.parse(readFileSync(manifestFile, "utf8"));
    const rewrite = (edit) => writeFileSync(manifestFile, JSON.stringify(edit(structuredClone(manifest))));
    rewrite((copy) => ((copy.artifacts[3].sha256 = "f".repeat(64)), copy));
    assert.throws(() => loadBundle(bundle.dir), /manifest's digest or size for .* differs from the file and SHA256SUMS/);
    rewrite((copy) => ((copy.artifacts[3].size += 1), copy));
    assert.throws(() => loadBundle(bundle.dir), /digest or size/);
    rewrite((copy) => ((copy.artifacts = copy.artifacts.slice(1)), copy));
    assert.throws(() => loadBundle(bundle.dir), /is in the bundle but not in the manifest/);
    rewrite((copy) => ((copy.cargo_packages[0].sha256 = "f".repeat(64)), copy));
    assert.throws(() => loadBundle(bundle.dir), /urollup-core-0\.1\.0\.crate differs from the manifest/);
    rewrite((copy) => ((copy.commit = OTHER), copy));
    assert.throws(() => loadBundle(bundle.dir), /manifest's commit d+ is not the plan's c+/);
    rewrite((copy) => copy);
    writeFileSync(path.join(bundle.dir, "assets", manifest.artifacts[0].name), "tampered");
    assert.throws(() => loadBundle(bundle.dir), /checksum conflict/);
  } finally {
    bundle.cleanup();
  }
});

test("publication runs only for its tag, at its commit, while the tag still names it", () => {
  const bundle = makeBundle();
  try {
    const { manifest } = loadBundle(bundle.dir);
    const world = fakeWorld({ bundle });
    assert.doesNotThrow(() => requirePublishRun(world.ctx, manifest));
    assert.throws(() => requirePublishRun(fakeWorld({ bundle }).ctx, { ...manifest, mode: "rehearsal" }), /publication runs only from the pushed tag/);
    const branch = fakeWorld({ bundle });
    branch.ctx.env.GITHUB_REF = "refs/heads/main";
    assert.throws(() => requirePublishRun(branch.ctx, manifest), /publication runs only from the pushed tag/);
    const moved = fakeWorld({ bundle, tagCommit: OTHER });
    assert.throws(() => requirePublishRun(moved.ctx, manifest), /v0\.1\.0 now names d+/);
    const head = fakeWorld({ bundle, run: (program, args) => (program === "git" && args[0] === "rev-parse" ? { status: 0, stdout: `${OTHER}\n` } : null) });
    assert.throws(() => requirePublishRun(head.ctx, manifest), /checkout is not the release commit/);
  } finally {
    bundle.cleanup();
  }
});

// ---------------------------------------------------------------------------------------
// crates.io

function cratesWorld(bundle, options = {}) {
  const world = fakeWorld({
    bundle,
    ...options,
    run: (program, args, runOptions, state) => {
      const packaged = packagesRehearsedBytes(bundle, options)(program, args, runOptions);
      if (packaged) {
        return packaged;
      }
      if (program === "cargo" && args[0] === "publish") {
        if (options.publishFails) {
          state.crates["urollup-core"] = { checksum: bundle.cargo[0].sha256 };
          return { status: 101, stdout: "", stderr: "error: failed to publish urollup" };
        }
        const names = args.includes("--workspace") ? ["urollup-core", "urollup"] : args.filter((_, index) => args[index - 1] === "-p");
        for (const name of names) {
          state.crates[name] = { checksum: bundle.cargo.find((item) => item.name === name).sha256 };
        }
        return { status: 0, stdout: "", stderr: "" };
      }
      return options.run?.(program, args, runOptions, state) ?? null;
    },
  });
  return world;
}

test("crates.io receives the rehearsed bytes, and only cargo publish --no-verify sees the token", async () => {
  const bundle = makeBundle();
  try {
    const { ctx, calls, state } = cratesWorld(bundle);
    const result = await publishCrates(ctx, { bundle: bundle.dir });
    assert.equal(result.after.state, "identical-complete");
    assert.deepEqual(Object.keys(state.crates).sort(), ["urollup", "urollup-core"]);
    const cargo = calls.filter((call) => call.program === "cargo");
    assert.deepEqual(cargo.map((call) => call.args.slice(0, 1)[0]), ["package", "publish"]);
    for (const call of cargo) {
      assert.ok(call.args.includes("--no-verify"), `${call.args.join(" ")} must not build`);
    }
    const withToken = calls.filter((call) => Object.values(call.env).includes("crates-token"));
    assert.deepEqual(withToken.map((call) => `${call.program} ${call.args[0]}`), ["cargo publish"]);
    assert.equal(withToken[0].env.CARGO_REGISTRY_TOKEN, "crates-token");
    assert.ok(!("RELEASE_CRATES_TOKEN" in ctx.env) && !("CARGO_REGISTRY_TOKEN" in ctx.env), "the token leaves the process environment");
    assert.deepEqual(cargo[1].args, ["publish", "--locked", "--no-verify", "--workspace"]);
    const lsRemoteBeforePublish = calls.slice(0, calls.indexOf(cargo[1])).filter((call) => call.args[0] === "ls-remote").length;
    assert.ok(lsRemoteBeforePublish >= 2, "the tag is re-checked at the start and immediately before the upload");
  } finally {
    bundle.cleanup();
  }
});

test("crates.io resumes a partial publication and refuses different bytes or a conflict", async () => {
  const bundle = makeBundle();
  try {
    const partial = cratesWorld(bundle, { crates: { "urollup-core": { checksum: bundle.cargo[0].sha256 } } });
    await publishCrates(partial.ctx, { bundle: bundle.dir });
    assert.deepEqual(partial.calls.find((call) => call.args[0] === "publish").args, ["publish", "--locked", "--no-verify", "-p", "urollup"]);
    assert.deepEqual(partial.calls.find((call) => call.args[0] === "package").args, ["package", "--locked", "--no-verify", "-p", "urollup"], "the digest check packages exactly what the resume uploads");

    const tampered = cratesWorld(bundle, { tamper: true });
    await assert.rejects(publishCrates(tampered.ctx, { bundle: bundle.dir }), /packages to .*, not the rehearsed/);
    assert.ok(!tampered.calls.some((call) => call.args[0] === "publish"), "nothing is uploaded after a digest mismatch");

    const conflict = cratesWorld(bundle, { crates: { "urollup-core": { checksum: "f".repeat(64) } } });
    await assert.rejects(publishCrates(conflict.ctx, { bundle: bundle.dir }), /refusing to publish to crates\.io: crates: conflict/);
    assert.ok(!conflict.calls.some((call) => call.program === "cargo"));

    const squatted = cratesWorld(bundle, { crateProjects: { urollup: true } });
    await assert.rejects(publishCrates(squatted.ctx, { bundle: bundle.dir }), /already exists without this version/);

    const moved = cratesWorld(bundle);
    moved.ctx.run = ((original) => (program, args, options) => {
      if (program === "cargo" && args[0] === "package") {
        moved.state.tagCommit = OTHER;
      }
      return original(program, args, options);
    })(moved.ctx.run);
    await assert.rejects(publishCrates(moved.ctx, { bundle: bundle.dir }), /now names d+/);
    assert.ok(!moved.calls.some((call) => call.args[0] === "publish"), "a tag moved during the job stops the upload");
  } finally {
    bundle.cleanup();
  }
});

test("a failed crates.io upload records what the registry holds before failing", async () => {
  const bundle = makeBundle();
  try {
    const lines = [];
    const { ctx } = cratesWorld(bundle, { publishFails: true });
    ctx.log = (entry) => lines.push(...entry);
    await assert.rejects(publishCrates(ctx, { bundle: bundle.dir }), /failed to publish urollup/);
    assert.match(lines.join("\n"), /crates\.io publishing failed; the registry now reads: crates: identical-partial; would publish urollup; already identical: urollup-core/);
  } finally {
    bundle.cleanup();
  }
});

test("missing RELEASE_CRATES_TOKEN stops before anything runs", async () => {
  const bundle = makeBundle();
  try {
    const { ctx, calls } = cratesWorld(bundle);
    delete ctx.env.RELEASE_CRATES_TOKEN;
    await assert.rejects(publishCrates(ctx, { bundle: bundle.dir }), /RELEASE_CRATES_TOKEN is not set/);
    assert.equal(calls.length, 0);
  } finally {
    bundle.cleanup();
  }
});

// ---------------------------------------------------------------------------------------
// PyPI

function pypiWorld(bundle, options = {}) {
  const wheels = bundle.artifacts.filter((artifact) => artifact.kind === "wheel");
  return fakeWorld({
    bundle,
    ...options,
    run: (program, args, runOptions, state) => {
      if (args[0] === "publish") {
        if (options.uploadFails) {
          state.pypi = [wheels[0]];
          return { status: 2, stdout: "", stderr: "error: upload failed" };
        }
        const names = args.filter((arg) => arg.endsWith(".whl")).map((file) => path.basename(file));
        state.pypi = [...(state.pypi ?? []), ...wheels.filter((wheel) => names.includes(wheel.name))];
        return { status: 0, stdout: "", stderr: "" };
      }
      return null;
    },
  });
}

test("PyPI uploads exactly the missing wheels through trusted publishing", async () => {
  const bundle = makeBundle();
  try {
    const wheels = bundle.artifacts.filter((artifact) => artifact.kind === "wheel");
    const fresh = pypiWorld(bundle);
    const result = await publishPypi(fresh.ctx, { bundle: bundle.dir });
    assert.equal(result.after.state, "identical-complete");
    const upload = fresh.calls.find((call) => call.args[0] === "publish");
    assert.deepEqual(upload.args.slice(0, 5), ["publish", "--trusted-publishing", "always", "--check-url", "https://pypi.org/simple/"]);
    assert.deepEqual(upload.args.slice(5).map((file) => path.basename(file)).sort(), wheels.map((wheel) => wheel.name).sort());
    assert.equal(upload.env.UV_NO_CONFIG, "1");

    const partial = pypiWorld(bundle, { pypi: wheels.slice(0, 2) });
    await publishPypi(partial.ctx, { bundle: bundle.dir });
    assert.deepEqual(partial.calls.find((call) => call.args[0] === "publish").args.slice(5).map((file) => path.basename(file)).sort(), wheels.slice(2).map((wheel) => wheel.name).sort());

    const complete = pypiWorld(bundle, { pypi: wheels });
    await publishPypi(complete.ctx, { bundle: bundle.dir });
    assert.ok(!complete.calls.some((call) => call.args[0] === "publish"), "an identical, complete release uploads nothing");

    const conflict = pypiWorld(bundle, { pypi: [{ ...wheels[0], sha256: "f".repeat(64) }] });
    await assert.rejects(publishPypi(conflict.ctx, { bundle: bundle.dir }), /refusing to publish to PyPI: pypi: conflict/);

    const squatted = pypiWorld(bundle, { pypiProject: true });
    await assert.rejects(publishPypi(squatted.ctx, { bundle: bundle.dir }), /PyPI project urollup already exists/);

    const lines = [];
    const failing = pypiWorld(bundle, { uploadFails: true });
    failing.ctx.log = (entry) => lines.push(...entry);
    await assert.rejects(publishPypi(failing.ctx, { bundle: bundle.dir }), /uv publish failed/);
    assert.match(lines.join("\n"), /PyPI publishing failed; the registry now reads: pypi: identical-partial/);
  } finally {
    bundle.cleanup();
  }
});

// ---------------------------------------------------------------------------------------
// GitHub Release

test("staging creates the draft, uploads every asset, and waits for GitHub's digests", async () => {
  const bundle = makeBundle();
  try {
    const { ctx, state, calls } = fakeWorld({ bundle });
    const slept = [];
    ctx.sleep = async (milliseconds) => slept.push(milliseconds);
    const result = await stageGithub(ctx, { bundle: bundle.dir });
    assert.equal(result.releaseId, 9);
    assert.equal(state.uploads.length, 12);
    assert.equal(state.release.draft, true);
    assert.equal(state.release.target_commitish, COMMIT);
    assert.ok(slept.length >= 1, "a digest missing right after upload is re-read, not treated as a conflict");
    assert.ok(calls.filter((call) => call.args[0] === "ls-remote").length >= 14, "the tag is re-checked before the create and before each upload");
  } finally {
    bundle.cleanup();
  }
});

test("an existing draft is brought to the planned name, notes, prerelease flag and target", async () => {
  const bundle = makeBundle();
  try {
    const assets = loadBundle(bundle.dir).assets.map((asset, index) => ({ id: index + 1, name: asset.name, state: "uploaded", digest: `sha256:${asset.sha256}` }));
    const stale = { id: 9, tag_name: "v0.1.0", draft: true, name: "old", body: "old notes", prerelease: true, target_commitish: OTHER, assets };
    const { ctx, state } = fakeWorld({ bundle, release: stale });
    const result = await stageGithub(ctx, { bundle: bundle.dir });
    assert.deepEqual(result.reconciled.sort(), ["body", "name", "prerelease", "target_commitish"]);
    assert.deepEqual(state.patches[0], { name: "urollup 0.1.0", body: "First release.\n", prerelease: false, target_commitish: COMMIT });
    assert.equal(state.uploads.length, 0);
  } finally {
    bundle.cleanup();
  }
});

test("the release goes public only after both registries verify, and with planned metadata", async () => {
  const bundle = makeBundle();
  try {
    const wheels = bundle.artifacts.filter((artifact) => artifact.kind === "wheel");
    const crates = Object.fromEntries(bundle.cargo.map((item) => [item.name, { checksum: item.sha256 }]));
    const assets = loadBundle(bundle.dir).assets.map((asset, index) => ({ id: index + 1, name: asset.name, state: "uploaded", digest: `sha256:${asset.sha256}` }));
    const draft = () => ({ id: 9, tag_name: "v0.1.0", draft: true, name: "urollup 0.1.0", body: "First release.\n", prerelease: false, target_commitish: COMMIT, assets: structuredClone(assets) });

    const missingPypi = fakeWorld({ bundle, crates, release: draft() });
    await assert.rejects(publishGithub(missingPypi.ctx, { bundle: bundle.dir }), /both registries must verify/);

    const ready = fakeWorld({ bundle, crates, pypi: wheels, release: draft() });
    const result = await publishGithub(ready.ctx, { bundle: bundle.dir });
    assert.equal(result.alreadyPublic, false);
    assert.deepEqual(ready.state.patches, [{ draft: false }]);

    const renamed = fakeWorld({ bundle, crates, pypi: wheels, release: { ...draft(), name: "something else" } });
    await assert.rejects(publishGithub(renamed.ctx, { bundle: bundle.dir }), /release's name differ from the plan/);
  } finally {
    bundle.cleanup();
  }
});

// ---------------------------------------------------------------------------------------
// Probes, evidence and attaching it

function probeWorld(bundle, { installFailures = 0, corruptDownload = false } = {}) {
  const wheels = bundle.artifacts.filter((artifact) => artifact.kind === "wheel");
  let installs = 0;
  const world = fakeWorld({
    bundle,
    pypi: wheels,
    run: (program, args, options) => {
      if (args[0] === "tool" && args[1] === "run") {
        return args.includes("--version")
          ? { status: 0, stdout: "urollup 0.1.0\n", stderr: "" }
          : { status: 0, stdout: JSON.stringify(fixtureReportFor(SMOKE_CASES[0])), stderr: "Installed 1 package\n" };
      }
      if (args[0] === "tool" && args[1] === "install") {
        installs += 1;
        if (installs <= installFailures) {
          return { status: 2, stdout: "", stderr: "No solution found: urollup==0.1.0 is not available yet" };
        }
        mkdirSync(options.env.UV_TOOL_BIN_DIR, { recursive: true });
        const file = path.join(options.env.UV_TOOL_BIN_DIR, process.platform === "win32" ? "urollup.exe" : "urollup");
        writeFileSync(file, bundle.executable, { mode: 0o755 });
        return { status: 0, stdout: "", stderr: "" };
      }
      return null;
    },
  });
  if (corruptDownload) {
    const original = world.ctx.fetchImpl;
    world.ctx.fetchImpl = async (url, options) => (url.endsWith(".tar.gz") ? response(200, Buffer.from("not the release")) : original(url, options));
  }
  return world;
}

test("a probe installs from all three channels, binding attestation to the release", async () => {
  const bundle = makeBundle();
  const out = mkdtempSync(path.join(tmpdir(), "urollup-probe-out-"));
  try {
    const { ctx, calls } = probeWorld(bundle, { installFailures: 2 });
    const record = await probe(ctx, { host: "linux-x86_64", bundle: bundle.dir, out });
    assert.equal(record.status, "passed");
    assert.deepEqual(Object.keys(record.channels), ["github", "pypi", "crates"]);
    const gh = calls.find((call) => call.program === "gh");
    assert.ok(gh.args.includes("--source-digest") && gh.args.includes(COMMIT) && gh.args.includes("--deny-self-hosted-runners"));
    assert.equal(gh.env.GH_TOKEN, "read-token");
    const others = calls.filter((call) => call.program !== "gh");
    assert.ok(others.every((call) => !Object.values(call.env).includes("read-token")), "only gh receives the read token");
    assert.equal(calls.filter((call) => call.args[0] === "tool" && call.args[1] === "install").length, 3, "install is retried while PyPI catches up");
    assert.ok(existsSync(path.join(out, "probe-linux-x86_64.json")));
  } finally {
    bundle.cleanup();
    rmSync(out, { recursive: true, force: true });
  }
});

test("a probe fails on a download that does not match, and on the wrong host", async () => {
  const bundle = makeBundle();
  const out = mkdtempSync(path.join(tmpdir(), "urollup-probe-out-"));
  try {
    const corrupt = probeWorld(bundle, { corruptDownload: true });
    await assert.rejects(probe(corrupt.ctx, { host: "linux-x86_64", bundle: bundle.dir, out }), /installation probes failed on linux-x86_64/);
    const record = JSON.parse(readFileSync(path.join(out, "probe-linux-x86_64.json"), "utf8"));
    assert.match(record.channels.github.problems.join("\n"), /checksum conflict|not the manifest's/);
    assert.ok(!corrupt.calls.some((call) => call.program === "gh"), "a mismatched download is never verified or run");
    const elsewhere = probeWorld(bundle);
    await assert.rejects(probe(elsewhere.ctx, { host: "macos-aarch64", bundle: bundle.dir, out }), /must run on its native host/);
  } finally {
    bundle.cleanup();
    rmSync(out, { recursive: true, force: true });
  }
});

test("PyPI probes wait until the simple index lists every wheel", async () => {
  const bundle = makeBundle();
  try {
    const { ctx, state } = probeWorld(bundle);
    const slept = [];
    ctx.sleep = async (milliseconds) => slept.push(milliseconds);
    await waitForPypiFiles(ctx, { project: "urollup", files: bundle.artifacts.filter((artifact) => artifact.kind === "wheel").map((artifact) => artifact.name) });
    assert.equal(state.pypiIndexCalls, 2);
    assert.equal(slept.length, 1);
  } finally {
    bundle.cleanup();
  }
});

function writeProbes(dir, status = "passed", skip = null) {
  for (const target of RELEASE_TARGETS) {
    if (target.id === skip) {
      continue;
    }
    mkdirSync(path.join(dir, `probe-${target.id}`), { recursive: true });
    writeFileSync(path.join(dir, `probe-${target.id}`, `probe-${target.id}.json`), JSON.stringify({ host: target.id, status }));
  }
}

test("evidence is written only when every probe passed and every registry is complete", async () => {
  const bundle = makeBundle();
  const probes = mkdtempSync(path.join(tmpdir(), "urollup-probes-"));
  const out = mkdtempSync(path.join(tmpdir(), "urollup-evidence-"));
  try {
    const wheels = bundle.artifacts.filter((artifact) => artifact.kind === "wheel");
    const crates = Object.fromEntries(bundle.cargo.map((item) => [item.name, { checksum: item.sha256 }]));
    const assets = loadBundle(bundle.dir).assets.map((asset, index) => ({ id: index + 1, name: asset.name, state: "uploaded", digest: `sha256:${asset.sha256}` }));
    const release = { id: 9, tag_name: "v0.1.0", draft: false, assets };
    writeProbes(probes, "passed", "windows-x86_64");
    await assert.rejects(evidence(fakeWorld({ bundle, crates, pypi: wheels, release }).ctx, { bundle: bundle.dir, probes, out }), /probe windows-x86_64 is missing/);
    writeProbes(probes, "passed");
    await assert.rejects(evidence(fakeWorld({ bundle, crates, release }).ctx, { bundle: bundle.dir, probes, out }), /pypi: absent/);
    const record = await evidence(fakeWorld({ bundle, crates, pypi: wheels, release }).ctx, { bundle: bundle.dir, probes, out });
    assert.equal(record.complete, true);
    assert.equal(record.manifest_sha256, loadBundle(bundle.dir).assets.find((asset) => asset.name === "release-manifest.json").sha256);
  } finally {
    bundle.cleanup();
    rmSync(probes, { recursive: true, force: true });
    rmSync(out, { recursive: true, force: true });
  }
});

test("evidence is attached once, only to the public release it describes", async () => {
  const bundle = makeBundle();
  const evidenceDir = mkdtempSync(path.join(tmpdir(), "urollup-evidence-"));
  try {
    const manifestDigest = loadBundle(bundle.dir).assets.find((asset) => asset.name === "release-manifest.json").sha256;
    const record = { complete: true, version: "0.1.0", tag: "v0.1.0", commit: COMMIT, manifest_sha256: manifestDigest };
    writeFileSync(path.join(evidenceDir, "release-evidence.json"), JSON.stringify(record));
    const data = readFileSync(path.join(evidenceDir, "release-evidence.json"));
    const release = (draft, extra = []) => ({ id: 9, tag_name: "v0.1.0", draft, assets: extra });

    await assert.rejects(attachEvidence(fakeWorld({ bundle, release: release(true) }).ctx, { bundle: bundle.dir, evidenceDir }), /must be public/);
    const same = await attachEvidence(fakeWorld({ bundle, release: release(false, [{ id: 1, name: "release-evidence.json", digest: `sha256:${sha256(data)}` }]) }).ctx, { bundle: bundle.dir, evidenceDir });
    assert.equal(same.attached, false);
    await assert.rejects(
      attachEvidence(fakeWorld({ bundle, release: release(false, [{ id: 1, name: "release-evidence.json", digest: `sha256:${"f".repeat(64)}` }]) }).ctx, { bundle: bundle.dir, evidenceDir }),
      /evidence is never replaced/,
    );
    const world = fakeWorld({ bundle, release: release(false) });
    const attached = await attachEvidence(world.ctx, { bundle: bundle.dir, evidenceDir });
    assert.equal(attached.attached, true);
    assert.ok(world.fetches.some((call) => call.method === "POST" && call.url.endsWith("name=release-evidence.json")));

    writeFileSync(path.join(evidenceDir, "release-evidence.json"), JSON.stringify({ ...record, commit: OTHER }));
    await assert.rejects(attachEvidence(fakeWorld({ bundle, release: release(false) }).ctx, { bundle: bundle.dir, evidenceDir }), /does not describe this release's bundle/);
  } finally {
    bundle.cleanup();
    rmSync(evidenceDir, { recursive: true, force: true });
  }
});
