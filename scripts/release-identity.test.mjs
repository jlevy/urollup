import assert from "node:assert/strict";
import test from "node:test";

import {
  checkCargoManifests,
  checkPyproject,
  checkVersionOutput,
  parseReleaseVersion,
  pep440Version,
  resolveRunIdentity,
  tagForVersion,
  versionFromTag,
  workspaceVersion,
} from "./release-identity.mjs";

const COMMIT = "0123456789abcdef0123456789abcdef01234567";
const OTHER = "fedcba9876543210fedcba9876543210fedcba98";

function workspace(version = "0.1.0", coreVersion = version) {
  return [
    "[workspace]",
    'members = ["crates/urollup-core", "crates/urollup"]',
    "",
    "[workspace.package]",
    `version = "${version}"`,
    'edition = "2024"',
    "",
    "[workspace.dependencies]",
    `urollup-core = { version = "${coreVersion}", path = "crates/urollup-core" }`,
    "",
  ].join("\n");
}

const MEMBER = '[package]\nname = "urollup"\nversion.workspace = true\n';

const PYPROJECT = [
  "[build-system]",
  'requires = ["maturin==1.15.0"]',
  'build-backend = "maturin"',
  "",
  "[project]",
  'name = "urollup"',
  'dynamic = ["version"]',
  "",
  "[tool.maturin]",
  'bindings = "bin"',
  "",
].join("\n");

test("release versions are SemVer without build metadata, with PEP 440 prereleases", () => {
  assert.equal(parseReleaseVersion("0.1.0").prerelease, null);
  assert.equal(parseReleaseVersion("1.2.3-rc.4").prerelease, "rc.4");
  for (const bad of ["0.1", "v0.1.0", "01.0.0", "0.1.0+build.5", "0.1.0-SNAPSHOT", "0.1.0-alpha", "0.1.0-rc.01", " 0.1.0", ""]) {
    assert.throws(() => parseReleaseVersion(bad), undefined, bad);
  }
  assert.equal(pep440Version("0.1.0"), "0.1.0");
  assert.equal(pep440Version("0.2.0-alpha.1"), "0.2.0a1");
  assert.equal(pep440Version("0.2.0-beta.0"), "0.2.0b0");
  assert.equal(pep440Version("1.0.0-rc.12"), "1.0.0rc12");
});

test("tags are exactly v and a release version", () => {
  assert.equal(versionFromTag("v0.1.0"), "0.1.0");
  assert.equal(versionFromTag("v1.0.0-rc.1"), "1.0.0-rc.1");
  assert.equal(tagForVersion("0.1.0"), "v0.1.0");
  for (const bad of ["0.1.0", "V0.1.0", "v0.1", "release-0.1.0", "v0.1.0+meta", "v0.1.0-preview"]) {
    assert.throws(() => versionFromTag(bad), undefined, bad);
  }
});

test("the workspace version is the only version source, and the core requirement must match it", () => {
  assert.equal(workspaceVersion(workspace()), "0.1.0");
  assert.throws(() => workspaceVersion("[workspace]\n"), /no \[workspace.package\] version/);
  assert.deepEqual(checkCargoManifests({ workspaceToml: workspace(), members: { urollup: MEMBER } }).problems, []);
  assert.match(
    checkCargoManifests({ workspaceToml: workspace("0.1.0", "0.1.1"), members: { urollup: MEMBER } }).problems.join("\n"),
    /urollup-core must require version "0\.1\.0", found "0\.1\.1"/,
  );
  assert.match(
    checkCargoManifests({ workspaceToml: workspace(), members: { urollup: '[package]\nname = "urollup"\nversion = "0.1.0"\n' } }).problems.join("\n"),
    /urollup must set version.workspace = true/,
  );
});

test("the PyPI metadata derives its version from Cargo and imposes no Python floor", () => {
  assert.deepEqual(checkPyproject(PYPROJECT), []);
  assert.match(checkPyproject(PYPROJECT.replace('dynamic = ["version"]', 'version = "0.1.0"')).join("\n"), /must not be a literal/);
  assert.match(checkPyproject(PYPROJECT.replace('name = "urollup"', 'name = "urollup-cli"')).join("\n"), /must be named "urollup"/);
  assert.match(checkPyproject(PYPROJECT.replace('bindings = "bin"', 'bindings = "pyo3"')).join("\n"), /bindings = "bin"/);
  assert.match(checkPyproject(PYPROJECT.replace('"maturin==1.15.0"', '"maturin>=1.15"')).join("\n"), /exact maturin/);
  assert.match(checkPyproject(PYPROJECT.replace('dynamic = ["version"]', 'dynamic = ["version"]\nrequires-python = ">=3.12"')).join("\n"), /requires-python/);
});

test("urollup --version must print exactly the release", () => {
  assert.deepEqual(checkVersionOutput("urollup 0.1.0\n", "0.1.0"), []);
  assert.deepEqual(checkVersionOutput("urollup 0.1.0\r\n", "0.1.0"), []);
  assert.equal(checkVersionOutput("urollup 0.1.1\n", "0.1.0").length, 1);
  assert.equal(checkVersionOutput("urollup 0.1.0", "0.1.0").length, 1);
  assert.equal(checkVersionOutput("urollup 0.1.0\nextra\n", "0.1.0").length, 1);
});

test("a tag push publishes, and only when the tag names the workspace version", () => {
  const identity = resolveRunIdentity({ event: "push", ref: "refs/tags/v0.1.0", sha: COMMIT, workspaceToml: workspace() });
  assert.deepEqual(identity, { mode: "publish", version: "0.1.0", tag: "v0.1.0", pep440: "0.1.0", commit: COMMIT, ref: "refs/tags/v0.1.0" });
  assert.throws(
    () => resolveRunIdentity({ event: "push", ref: "refs/tags/v0.1.1", sha: COMMIT, workspaceToml: workspace() }),
    /tag v0\.1\.1 names 0\.1\.1, but the workspace version is 0\.1\.0/,
  );
  assert.throws(() => resolveRunIdentity({ event: "push", ref: "refs/heads/main", sha: COMMIT, workspaceToml: workspace() }), /must be a tag/);
  assert.throws(() => resolveRunIdentity({ event: "push", ref: "refs/tags/0.1.0", sha: COMMIT, workspaceToml: workspace() }), /must be v followed/);
});

test("a dispatch rehearses an explicit commit and version and can never publish", () => {
  const inputs = { commit: COMMIT, version: "0.1.0" };
  const identity = resolveRunIdentity({ event: "workflow_dispatch", ref: "refs/heads/main", sha: COMMIT, inputs, workspaceToml: workspace() });
  assert.equal(identity.mode, "rehearsal");
  assert.equal(identity.tag, "v0.1.0");
  assert.throws(
    () => resolveRunIdentity({ event: "workflow_dispatch", ref: "refs/heads/main", sha: OTHER, inputs, workspaceToml: workspace() }),
    /is not the head of refs\/heads\/main/,
  );
  assert.throws(
    () => resolveRunIdentity({ event: "workflow_dispatch", ref: "refs/heads/main", sha: COMMIT, inputs: { ...inputs, version: "0.2.0" }, workspaceToml: workspace() }),
    /does not match the workspace version/,
  );
  assert.throws(
    () => resolveRunIdentity({ event: "workflow_dispatch", ref: "refs/heads/main", sha: COMMIT, inputs: { ...inputs, commit: "HEAD" }, workspaceToml: workspace() }),
    /full 40-character commit/,
  );
  for (const event of ["pull_request", "schedule", "release", "pull_request_target"]) {
    assert.throws(() => resolveRunIdentity({ event, ref: "refs/tags/v0.1.0", sha: COMMIT, inputs, workspaceToml: workspace() }), /only from a v\* tag push or workflow_dispatch/);
  }
});
