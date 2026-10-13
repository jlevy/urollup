import assert from "node:assert/strict";
import test from "node:test";

import {
  MANIFEST_SCHEMA,
  buildManifest,
  changelogSection,
  checkCompleteness,
  checkReleaseNotes,
  compareChecksums,
  formatChecksums,
  parseChecksums,
  renderReleaseNotes,
} from "./release-manifest.mjs";
import { plannedArtifacts, plannedCrates } from "./release-targets.mjs";

const digest = (seed) => seed.repeat(64).slice(0, 64);
const IDENTITY = { mode: "rehearsal", version: "0.1.0", tag: "v0.1.0", pep440: "0.1.0", commit: "0123456789abcdef0123456789abcdef01234567" };

function fullSet() {
  const artifacts = plannedArtifacts("0.1.0").map((artifact, index) => ({ name: artifact.name, sha256: digest(index.toString(16)), size: 1000 + index }));
  const validations = plannedArtifacts("0.1.0").map((artifact, index) => ({
    artifact: artifact.name,
    id: artifact.id,
    sha256: digest(index.toString(16)),
    status: "passed",
    host: artifact.host,
    facts: { executable_sha256: digest("e") },
  }));
  const crates = plannedCrates("0.1.0").map((crate, index) => ({ ...crate, sha256: digest(String.fromCharCode(97 + index)), size: 50 }));
  return { artifacts, validations, crates };
}

test("SHA256SUMS is sorted, two-space separated and round-trips", () => {
  const text = formatChecksums([
    { name: "b.whl", sha256: digest("b") },
    { name: "a.tar.gz", sha256: digest("a") },
  ]);
  assert.equal(text, `${digest("a")}  a.tar.gz\n${digest("b")}  b.whl\n`);
  assert.deepEqual([...parseChecksums(text)], [["a.tar.gz", digest("a")], ["b.whl", digest("b")]]);
  assert.throws(() => formatChecksums([]), /empty artifact set/);
  assert.throws(() => formatChecksums([{ name: "a", sha256: "xyz" }]), /no SHA-256 digest/);
  assert.throws(() => formatChecksums([{ name: "a", sha256: digest("a") }, { name: "a", sha256: digest("b") }]), /duplicated/);
  assert.throws(() => parseChecksums(`${digest("a")}  a\n${digest("b")}  a\n`), /lists a twice/);
  assert.throws(() => parseChecksums("deadbeef  a\n"), /malformed/);
});

test("a checksum conflict, a missing file or an unlisted file never passes", () => {
  const expected = parseChecksums(formatChecksums([{ name: "a", sha256: digest("a") }, { name: "b", sha256: digest("b") }]));
  assert.deepEqual(compareChecksums(expected, [{ name: "a", sha256: digest("a") }, { name: "b", sha256: digest("b") }]), []);
  assert.match(compareChecksums(expected, [{ name: "a", sha256: digest("c") }, { name: "b", sha256: digest("b") }]).join("\n"), /checksum conflict for a/);
  assert.match(compareChecksums(expected, [{ name: "a", sha256: digest("a") }]).join("\n"), /b is listed in SHA256SUMS but missing/);
  assert.match(compareChecksums(expected, [{ name: "a", sha256: digest("a") }, { name: "b", sha256: digest("b") }, { name: "c", sha256: digest("c") }]).join("\n"), /c is not listed/);
});

test("the complete matrix passes, and an empty, partial or foreign set fails", () => {
  const set = fullSet();
  assert.deepEqual(checkCompleteness({ version: "0.1.0", ...set }), []);
  assert.match(checkCompleteness({ version: "0.1.0", ...set, artifacts: [] }).join("\n"), /no artifacts; an empty set is never a release/);
  assert.match(
    checkCompleteness({ version: "0.1.0", ...set, artifacts: set.artifacts.filter((artifact) => !artifact.name.includes("windows")) }).join("\n"),
    /missing archive urollup-0\.1\.0-x86_64-pc-windows-msvc\.zip/,
  );
  assert.match(checkCompleteness({ version: "0.1.0", ...set, artifacts: [...set.artifacts, { name: "urollup-0.1.0-riscv64.tar.gz", sha256: digest("f"), size: 1 }] }).join("\n"), /unexpected artifact/);
  assert.match(checkCompleteness({ version: "0.1.0", ...set, crates: set.crates.slice(1) }).join("\n"), /missing Cargo package urollup-core-0\.1\.0\.crate/);
  assert.match(checkCompleteness({ version: "0.1.0", ...set, artifacts: set.artifacts.map((artifact, index) => (index === 0 ? { ...artifact, size: 0 } : artifact)) }).join("\n"), /empty/);
});

test("every artifact needs a passing validation of the same bytes on its native host", () => {
  const set = fullSet();
  const swap = (change) => ({ version: "0.1.0", ...set, validations: set.validations.map((validation, index) => (index === 4 ? change(validation) : validation)) });
  assert.match(checkCompleteness({ version: "0.1.0", ...set, validations: set.validations.slice(1) }).join("\n"), /has no native validation record/);
  assert.match(checkCompleteness(swap((validation) => ({ ...validation, sha256: digest("9") }))).join("\n"), /was validated as/);
  assert.match(checkCompleteness(swap((validation) => ({ ...validation, status: "failed" }))).join("\n"), /did not pass validation/);
  assert.match(checkCompleteness(swap((validation) => ({ ...validation, host: "linux-x86_64" }))).join("\n"), /validated on linux-x86_64, not its native macos-aarch64 host/);
});

test("the manifest records identity, toolchain, targets, features, digests and hosts", () => {
  const set = fullSet();
  const builds = set.artifacts.map((artifact) => ({ artifact: artifact.name, runner_image: "ubuntu24 20261005.1", rustc: "rustc 1.98.0", builder: "cargo build" }));
  const manifest = buildManifest({ identity: IDENTITY, toolchain: "1.98.0", sourceDateEpoch: 1, workflowRun: { run_id: "7" }, ...set, builds });
  assert.equal(manifest.schema, MANIFEST_SCHEMA);
  assert.equal(manifest.commit, IDENTITY.commit);
  assert.equal(manifest.rust_toolchain, "1.98.0");
  assert.equal(manifest.artifacts.length, 10);
  const wheel = manifest.artifacts.find((artifact) => artifact.kind === "wheel" && artifact.target === "x86_64-unknown-linux-gnu");
  assert.equal(wheel.wheel_platform_tag, "manylinux_2_17_x86_64.manylinux2014_x86_64");
  assert.deepEqual(wheel.features, ["default"]);
  assert.match(wheel.build.container, /@sha256:/);
  assert.deepEqual(manifest.cargo_packages.map((item) => item.name), ["urollup-core", "urollup"]);
  assert.ok(!("channels" in manifest) && !("published" in manifest), "the manifest holds identity, never publication progress");
});

const CHANGELOG = `# Changelog

## 0.2.0 - Unreleased

### Added

- Weekly rollups.

### Fixed

- A miscount.

## 0.1.0 - 2026-10-20

First release.

### Commands

- report
`;

test("the CHANGELOG section for a version is found, and Unreleased blocks a tag", () => {
  const first = changelogSection(CHANGELOG, "0.1.0");
  assert.equal(first.released, true);
  assert.equal(first.first, true);
  assert.match(first.body, /^First release\./);
  const second = changelogSection(CHANGELOG, "0.2.0");
  assert.equal(second.released, false);
  assert.equal(second.first, false);
  assert.deepEqual(checkReleaseNotes(second, { mode: "rehearsal" }), []);
  assert.match(checkReleaseNotes(second, { mode: "publish" }).join("\n"), /still says Unreleased/);
  assert.throws(() => changelogSection(CHANGELOG, "0.3.0"), /no "## 0\.3\.0" section/);
});

test("first-release notes may not list fixes to defects that never shipped", () => {
  const draft = changelogSection("# Changelog\n\n## 0.1.0 - 2026-10-20\n\n### Added\n\n- report\n\n### Fixed\n\n- Codex cache writes counted twice.\n", "0.1.0");
  assert.match(checkReleaseNotes(draft, { mode: "publish" }).join("\n"), /first release's notes list fixes/);
  assert.match(checkReleaseNotes(changelogSection("## 0.1.0\n\n## 0.0.9\n", "0.1.0"), { mode: "rehearsal" }).join("\n"), /is empty/);
});

test("draft release notes carry the section, install commands and every digest", () => {
  const set = fullSet();
  const manifest = buildManifest({ identity: IDENTITY, toolchain: "1.98.0", sourceDateEpoch: 1, workflowRun: null, ...set, builds: [] });
  const notes = renderReleaseNotes({ identity: IDENTITY, section: changelogSection(CHANGELOG, "0.1.0"), artifacts: manifest.artifacts, repository: "jlevy/urollup" });
  assert.match(notes, /^First release\./);
  assert.match(notes, /uvx --isolated urollup@0\.1\.0 --version/);
  assert.match(notes, /uv tool install urollup==0\.1\.0/);
  assert.match(notes, /cargo install --locked urollup@0\.1\.0/);
  assert.match(notes, /gh attestation verify <file> --repo jlevy\/urollup/);
  for (const artifact of manifest.artifacts) {
    assert.ok(notes.includes(artifact.sha256) && notes.includes(artifact.name));
  }
});
