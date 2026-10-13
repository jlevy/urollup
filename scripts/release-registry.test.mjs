import assert from "node:assert/strict";
import test from "node:test";

import {
  USER_AGENT,
  cratesPublishCommand,
  decideCrates,
  decideGithubRelease,
  decidePypi,
  describeDecision,
  observeCrate,
  observeGithubRelease,
  observePypi,
  publishGithubRelease,
  stageGithubRelease,
} from "./release-registry.mjs";

const digest = (seed) => seed.repeat(64).slice(0, 64);
const WHEELS = [
  { name: "urollup-0.1.0-py3-none-win_amd64.whl", sha256: digest("a") },
  { name: "urollup-0.1.0-py3-none-macosx_11_0_arm64.whl", sha256: digest("b") },
];
const CRATES = [
  { name: "urollup-core", sha256: digest("c") },
  { name: "urollup", sha256: digest("d") },
];

test("PyPI: absent, identical complete, identical partial, conflicting and unavailable", () => {
  const files = (list) => ({ status: "present", files: list.map((file) => ({ ...file, packagetype: "bdist_wheel", yanked: false })) });
  assert.equal(decidePypi(WHEELS, { status: "absent" }).state, "absent");
  assert.deepEqual(decidePypi(WHEELS, { status: "absent" }).publish, WHEELS.map((wheel) => wheel.name));
  const complete = decidePypi(WHEELS, files(WHEELS));
  assert.equal(complete.state, "identical-complete");
  assert.deepEqual(complete.publish, []);
  const partial = decidePypi(WHEELS, files([WHEELS[0]]));
  assert.equal(partial.state, "identical-partial");
  assert.deepEqual(partial.publish, [WHEELS[1].name]);
  const conflict = decidePypi(WHEELS, files([{ name: WHEELS[0].name, sha256: digest("f") }]));
  assert.equal(conflict.state, "conflict");
  assert.equal(conflict.blocking, true);
  assert.deepEqual(conflict.publish, [], "a checksum conflict never publishes and never counts as success");
  assert.equal(decidePypi(WHEELS, files([...WHEELS, { name: "urollup-0.1.0.tar.gz", sha256: digest("e") }])).state, "conflict");
  assert.match(
    decidePypi(WHEELS, { status: "present", files: [{ ...WHEELS[0], packagetype: "sdist" }] }).conflicts.join("\n"),
    /urollup publishes wheels only/,
  );
  assert.match(decidePypi(WHEELS, { status: "present", files: [{ ...WHEELS[0], packagetype: "bdist_wheel", yanked: true }] }).conflicts.join("\n"), /yanked/);
  const unavailable = decidePypi(WHEELS, { status: "unavailable", reason: "HTTP 503" });
  assert.equal(unavailable.state, "unavailable");
  assert.equal(unavailable.blocking, true, "unknown registry state is never treated as absent");
});

test("crates.io: dependency order, partial resumption and conflicts", () => {
  const present = (sha256, yanked = false) => ({ status: "present", checksum: sha256, yanked });
  const absent = { status: "absent" };
  const none = decideCrates(CRATES, { "urollup-core": absent, urollup: absent });
  assert.equal(none.state, "absent");
  assert.deepEqual(cratesPublishCommand(none), ["cargo", "publish", "--locked", "--workspace"]);
  const partial = decideCrates(CRATES, { "urollup-core": present(digest("c")), urollup: absent });
  assert.equal(partial.state, "identical-partial");
  assert.deepEqual(cratesPublishCommand(partial), ["cargo", "publish", "--locked", "-p", "urollup"]);
  const complete = decideCrates(CRATES, { "urollup-core": present(digest("c")), urollup: present(digest("d")) });
  assert.equal(complete.state, "identical-complete");
  assert.equal(cratesPublishCommand(complete), null);
  const conflict = decideCrates(CRATES, { "urollup-core": present(digest("9")), urollup: absent });
  assert.equal(conflict.state, "conflict");
  assert.equal(cratesPublishCommand(conflict), null);
  assert.match(decideCrates(CRATES, { "urollup-core": absent, urollup: present(digest("d")) }).conflicts.join("\n"), /urollup is published before its dependency urollup-core/);
  assert.match(decideCrates(CRATES, { "urollup-core": present(digest("c"), true), urollup: absent }).conflicts.join("\n"), /urollup-core is yanked/);
  assert.equal(decideCrates(CRATES, { "urollup-core": { status: "unavailable", reason: "timeout" }, urollup: absent }).state, "unavailable");
  assert.equal(decideCrates(CRATES, { "urollup-core": absent }).state, "unavailable");
});

test("GitHub: drafts resume, a public release never takes new or different assets", () => {
  const assets = [
    { name: "urollup-0.1.0-x86_64-unknown-linux-musl.tar.gz", sha256: digest("1") },
    { name: "SHA256SUMS", sha256: digest("2") },
  ];
  const absent = decideGithubRelease(assets, { status: "absent" });
  assert.equal(absent.state, "absent");
  assert.equal(absent.release, "create-draft");
  const draftPartial = decideGithubRelease(assets, { status: "present", draft: true, assets: [assets[0]] });
  assert.equal(draftPartial.state, "identical-partial");
  assert.deepEqual(draftPartial.publish, ["SHA256SUMS"]);
  assert.equal(decideGithubRelease(assets, { status: "present", draft: true, assets: [] }).state, "absent");
  assert.equal(decideGithubRelease(assets, { status: "present", draft: false, assets }).state, "identical-complete");
  assert.equal(decideGithubRelease(assets, { status: "present", draft: false, assets: [assets[0]] }).state, "conflict");
  assert.equal(decideGithubRelease(assets, { status: "present", draft: false, assets: [] }).state, "conflict");
  assert.equal(decideGithubRelease(assets, { status: "present", draft: true, assets: [{ name: assets[0].name, sha256: null }] }).state, "conflict");
  assert.equal(decideGithubRelease(assets, { status: "present", draft: true, assets: [...assets, { name: "notes.txt", sha256: digest("3") }] }).state, "conflict");
  assert.equal(decideGithubRelease(assets, { status: "unavailable", reason: "x" }).blocking, true);
  assert.match(describeDecision(draftPartial), /github: identical-partial; would publish SHA256SUMS; already identical: urollup-0\.1\.0/);
});

function fakeFetch(routes) {
  const calls = [];
  const impl = async (url, options = {}) => {
    calls.push({ url, method: options.method ?? "GET", headers: options.headers ?? {} });
    const route = routes.find((candidate) => candidate.match(url, options.method ?? "GET"));
    if (!route) {
      return { ok: false, status: 404, json: async () => ({}) };
    }
    const result = typeof route.respond === "function" ? route.respond(url, options) : route.respond;
    if (result instanceof Error) {
      throw result;
    }
    return { ok: result.status >= 200 && result.status < 300, status: result.status, json: async () => result.body };
  };
  return { impl, calls };
}

test("observers read anonymously with a project User-Agent and map HTTP states", async () => {
  const { impl, calls } = fakeFetch([
    { match: (url) => url.includes("crates.io/api/v1/crates/urollup-core/0.1.0"), respond: { status: 200, body: { version: { num: "0.1.0", checksum: digest("c"), yanked: false } } } },
    { match: (url) => url.includes("crates.io/api/v1/crates/urollup/0.1.0"), respond: { status: 503, body: {} } },
    { match: (url) => url.includes("pypi.org/pypi/urollup/0.1.0/json"), respond: { status: 200, body: { urls: [{ filename: WHEELS[0].name, digests: { sha256: digest("a") }, packagetype: "bdist_wheel", yanked: false }] } } },
  ]);
  assert.deepEqual(await observeCrate("urollup-core", "0.1.0", impl), { status: "present", checksum: digest("c"), yanked: false });
  assert.equal((await observeCrate("urollup", "0.1.0", impl)).status, "unavailable");
  assert.equal((await observePypi("urollup", "0.1.0", impl)).files[0].sha256, digest("a"));
  assert.equal((await observePypi("other", "0.1.0", impl)).status, "absent");
  assert.ok(calls.every((call) => call.headers["User-Agent"] === USER_AGENT && !("Authorization" in call.headers)));
  assert.ok(!/@|levy/i.test(USER_AGENT), "the User-Agent names the project, not a person");
  const failing = async () => {
    throw new Error("ECONNRESET");
  };
  assert.equal((await observeCrate("urollup", "0.1.0", failing)).status, "unavailable");
});

test("GitHub staging creates one draft, uploads only missing assets and verifies them", async () => {
  const files = [
    { name: "a.tar.gz", sha256: digest("1"), data: Buffer.from("a") },
    { name: "SHA256SUMS", sha256: digest("2"), data: Buffer.from("s") },
  ];
  let release = null;
  const uploads = [];
  const { impl, calls } = fakeFetch([
    { match: (url, method) => method === "GET" && url.endsWith("/releases?per_page=100"), respond: () => ({ status: 200, body: release ? [release] : [] }) },
    {
      match: (url, method) => method === "POST" && url.endsWith("/repos/jlevy/urollup/releases"),
      respond: (url, options) => {
        release = { id: 42, tag_name: JSON.parse(options.body).tag_name, draft: JSON.parse(options.body).draft, assets: [] };
        return { status: 201, body: release };
      },
    },
    {
      match: (url, method) => method === "POST" && url.startsWith("https://uploads.github.com/repos/jlevy/urollup/releases/42/assets"),
      respond: (url) => {
        const name = decodeURIComponent(new URL(url).searchParams.get("name"));
        uploads.push(name);
        release.assets.push({ id: uploads.length, name, digest: `sha256:${files.find((file) => file.name === name).sha256}` });
        return { status: 201, body: {} };
      },
    },
    { match: (url, method) => method === "PATCH" && url.endsWith("/releases/42"), respond: () => ((release.draft = false), { status: 200, body: release }) },
  ]);
  const staged = await stageGithubRelease({ repository: "jlevy/urollup", tag: "v0.1.0", commit: "c", title: "urollup 0.1.0", notes: "n", prerelease: false, files, token: "t", fetchImpl: impl });
  assert.equal(staged.releaseId, 42);
  assert.deepEqual(uploads, ["a.tar.gz", "SHA256SUMS"]);
  assert.equal(release.draft, true);
  await stageGithubRelease({ repository: "jlevy/urollup", tag: "v0.1.0", commit: "c", title: "t", notes: "n", prerelease: false, files, token: "t", fetchImpl: impl });
  assert.deepEqual(uploads, ["a.tar.gz", "SHA256SUMS"], "a rerun uploads nothing that is already identical");
  const published = await publishGithubRelease({ repository: "jlevy/urollup", tag: "v0.1.0", files, token: "t", fetchImpl: impl });
  assert.equal(published.alreadyPublic, false);
  assert.equal(release.draft, false);
  assert.ok(calls.filter((call) => call.method !== "GET").every((call) => call.headers.Authorization === "Bearer t"));
  release.assets[0].digest = `sha256:${digest("9")}`;
  await assert.rejects(
    stageGithubRelease({ repository: "jlevy/urollup", tag: "v0.1.0", commit: "c", title: "t", notes: "n", prerelease: false, files, token: "t", fetchImpl: impl }),
    /conflict/,
  );
  await assert.rejects(publishGithubRelease({ repository: "jlevy/urollup", tag: "v0.1.0", files, token: "t", fetchImpl: impl }), /refusing to publish/);
});

test("a read-only token sees published releases only; drafts need the write token", async () => {
  const { impl, calls } = fakeFetch([
    { match: (url) => url.endsWith("/releases/tags/v0.1.0"), respond: { status: 200, body: { id: 1, draft: false, assets: [{ id: 2, name: "x", digest: `sha256:${digest("1")}` }] } } },
  ]);
  const observed = await observeGithubRelease("jlevy/urollup", "v0.1.0", { token: "read", fetchImpl: impl });
  assert.deepEqual(observed, { status: "present", id: 1, draft: false, assets: [{ name: "x", sha256: digest("1"), id: 2 }] });
  assert.match(calls[0].url, /\/releases\/tags\/v0\.1\.0$/);
  const many = fakeFetch([{ match: (url) => url.endsWith("/releases?per_page=100"), respond: { status: 200, body: [{ tag_name: "v0.1.0" }, { tag_name: "v0.1.0" }] } }]);
  assert.equal((await observeGithubRelease("jlevy/urollup", "v0.1.0", { token: "w", includeDrafts: true, fetchImpl: many.impl })).status, "unavailable");
});
