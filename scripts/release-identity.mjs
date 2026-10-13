// Release identity: one version, one tag and one commit for every channel.
//
// The workspace version in Cargo.toml is the only version source
// (docs/project/specs/active/plan-2026-09-16-first-release-publishing.md, Release Identity
// and Invariants). This module parses it, checks every place that must agree with it, and
// resolves the identity of a release run from the triggering event: a `v*` tag publishes,
// and a dispatch rehearses an explicit commit and version with no write authority. Any
// mismatch is refused rather than reconciled.

/** SemVer 2.0.0 (semver.org), split into its parts. */
const SEMVER =
  /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-((?:0|[1-9]\d*|\d*[a-zA-Z-][0-9a-zA-Z-]*)(?:\.(?:0|[1-9]\d*|\d*[a-zA-Z-][0-9a-zA-Z-]*))*))?(?:\+([0-9a-zA-Z-]+(?:\.[0-9a-zA-Z-]+)*))?$/;

/** The prerelease forms a release may use, and their PEP 440 spelling for PyPI. */
const PRERELEASE = /^(alpha|beta|rc)\.(0|[1-9]\d*)$/;
const PEP440_PRERELEASE = { alpha: "a", beta: "b", rc: "rc" };

export const COMMIT = /^[0-9a-f]{40}$/;

function fail(message) {
  throw new Error(message);
}

/**
 * Parse a release version. Build metadata is refused because crates.io ignores it and
 * PyPI forbids local versions, so two different builds could share one published version.
 * A prerelease must be `alpha.N`, `beta.N` or `rc.N`, the forms with one PEP 440 spelling.
 */
export function parseReleaseVersion(text) {
  const match = typeof text === "string" ? text.match(SEMVER) : null;
  if (!match) {
    fail(`${JSON.stringify(text)} is not a SemVer 2.0.0 version`);
  }
  const [, major, minor, patch, prerelease, build] = match;
  if (build !== undefined) {
    fail(`${text} carries build metadata (+${build}), which crates.io ignores and PyPI forbids`);
  }
  if (prerelease !== undefined && !PRERELEASE.test(prerelease)) {
    fail(`${text} has prerelease ${prerelease}; a release prerelease must be alpha.N, beta.N or rc.N`);
  }
  return {
    version: text,
    major: Number(major),
    minor: Number(minor),
    patch: Number(patch),
    prerelease: prerelease ?? null,
  };
}

/** The PEP 440 version Maturin writes for a Cargo version (`0.1.0-rc.1` is `0.1.0rc1`). */
export function pep440Version(version) {
  const parsed = parseReleaseVersion(version);
  const base = `${parsed.major}.${parsed.minor}.${parsed.patch}`;
  if (parsed.prerelease === null) {
    return base;
  }
  const [, kind, number] = parsed.prerelease.match(PRERELEASE);
  return `${base}${PEP440_PRERELEASE[kind]}${number}`;
}

/** The version a release tag names; the tag must be exactly `v` and a release version. */
export function versionFromTag(tag) {
  if (typeof tag !== "string" || !tag.startsWith("v")) {
    fail(`release tag ${JSON.stringify(tag)} must be v followed by the version, such as v0.1.0`);
  }
  return parseReleaseVersion(tag.slice(1)).version;
}

export function tagForVersion(version) {
  return `v${parseReleaseVersion(version).version}`;
}

// ---------------------------------------------------------------------------------------
// Manifests

/** The body of one TOML table, up to the next table header. */
function tomlTable(text, name) {
  const lines = text.split(/\r?\n/);
  const header = `[${name}]`;
  const start = lines.findIndex((line) => line.trim() === header);
  if (start < 0) {
    return null;
  }
  const body = [];
  for (const line of lines.slice(start + 1)) {
    if (/^\s*\[/.test(line)) {
      break;
    }
    body.push(line);
  }
  return body.join("\n");
}

function tomlString(table, key) {
  const match = table?.match(new RegExp(`^${key.replace(/[.]/g, "\\.")}\\s*=\\s*"([^"]*)"\\s*(?:#.*)?$`, "m"));
  return match ? match[1] : null;
}

/** The `[workspace.package]` version, which every crate and wheel inherits. */
export function workspaceVersion(cargoToml) {
  const table = tomlTable(cargoToml, "workspace.package");
  const version = tomlString(table, "version");
  if (version === null) {
    fail("Cargo.toml has no [workspace.package] version");
  }
  return parseReleaseVersion(version).version;
}

/**
 * Check that a member crate inherits the workspace version, and that the workspace's
 * `urollup-core` requirement names the same version, so `cargo publish --workspace` cannot
 * upload an executable that depends on a different core release.
 */
export function checkCargoManifests({ workspaceToml, members }) {
  const version = workspaceVersion(workspaceToml);
  const problems = [];
  for (const [name, text] of Object.entries(members)) {
    const table = tomlTable(text, "package");
    if (!table || !/^version\.workspace\s*=\s*true\s*$/m.test(table)) {
      problems.push(`${name} must set version.workspace = true`);
    }
  }
  const dependencies = tomlTable(workspaceToml, "workspace.dependencies") ?? "";
  const core = dependencies.match(/^urollup-core\s*=\s*\{([^}]*)\}/m);
  const coreVersion = core?.[1].match(/version\s*=\s*"([^"]*)"/)?.[1];
  if (coreVersion !== version) {
    problems.push(
      `[workspace.dependencies] urollup-core must require version ${JSON.stringify(version)}, found ${JSON.stringify(coreVersion ?? null)}`,
    );
  }
  return { version, problems };
}

/**
 * Check the PyPI distribution metadata: the distribution is `urollup`, Maturin derives its
 * version from Cargo (`dynamic = ["version"]`), and no literal version can drift from it.
 */
export function checkPyproject(pyprojectToml) {
  const problems = [];
  const project = tomlTable(pyprojectToml, "project");
  if (project === null) {
    return ["crates/urollup/pyproject.toml has no [project] table"];
  }
  if (tomlString(project, "name") !== "urollup") {
    problems.push('the PyPI distribution must be named "urollup"');
  }
  if (/^version\s*=/m.test(project)) {
    problems.push("the PyPI version must not be a literal; Maturin derives it from the Cargo version");
  }
  const dynamic = project.match(/^dynamic\s*=\s*\[([^\]]*)\]/m)?.[1] ?? "";
  if (!/"version"/.test(dynamic)) {
    problems.push('[project] must declare dynamic = ["version"]');
  }
  if (/^requires-python\s*=/m.test(project)) {
    problems.push("the binary wheel loads no Python ABI, so it must not declare requires-python");
  }
  const maturin = tomlTable(pyprojectToml, "tool.maturin");
  if (tomlString(maturin, "bindings") !== "bin") {
    problems.push('[tool.maturin] must set bindings = "bin"');
  }
  const backend = tomlTable(pyprojectToml, "build-system");
  if (!/^requires\s*=\s*\["maturin==\d+\.\d+\.\d+"\]\s*$/m.test(backend ?? "")) {
    problems.push("[build-system] must require one exact maturin==X.Y.Z");
  }
  return problems;
}

/** The exact first line `urollup --version` prints for a release. */
export function expectedVersionOutput(version) {
  return `urollup ${version}`;
}

/** Refuse `urollup --version` output that is not exactly this release. */
export function checkVersionOutput(stdout, version) {
  const lines = stdout.replace(/\r\n/g, "\n").split("\n");
  if (lines.length !== 2 || lines[1] !== "" || lines[0] !== expectedVersionOutput(version)) {
    return [`urollup --version printed ${JSON.stringify(stdout)}, expected ${JSON.stringify(`${expectedVersionOutput(version)}\n`)}`];
  }
  return [];
}

// ---------------------------------------------------------------------------------------
// Run identity

/**
 * Resolve what a release run is from its trigger.
 *
 * - A push of `refs/tags/v<version>` publishes; the tag must name the workspace version.
 * - A `workflow_dispatch` rehearses: it requires the commit and version as inputs, the
 *   commit must be the head of the dispatched ref (so the release machinery that runs is
 *   the candidate's own), and the version must be the workspace version.
 *
 * Any other trigger is refused. Returns `{mode, version, tag, pep440, commit, ref}`.
 */
export function resolveRunIdentity({ event, ref, sha, inputs = {}, workspaceToml }) {
  const version = workspaceVersion(workspaceToml);
  if (!COMMIT.test(sha ?? "")) {
    fail(`the workflow commit ${JSON.stringify(sha)} is not a full commit ID`);
  }
  if (event === "push") {
    if (typeof ref !== "string" || !ref.startsWith("refs/tags/")) {
      fail(`a release push must be a tag, not ${JSON.stringify(ref)}`);
    }
    const tag = ref.slice("refs/tags/".length);
    const tagVersion = versionFromTag(tag);
    if (tagVersion !== version) {
      fail(`tag ${tag} names ${tagVersion}, but the workspace version is ${version}`);
    }
    return { mode: "publish", version, tag, pep440: pep440Version(version), commit: sha, ref };
  }
  if (event === "workflow_dispatch") {
    const commit = (inputs.commit ?? "").trim();
    const requested = (inputs.version ?? "").trim();
    if (!COMMIT.test(commit)) {
      fail(`the rehearsal commit input ${JSON.stringify(inputs.commit ?? "")} must be a full 40-character commit ID`);
    }
    if (commit !== sha) {
      fail(`the rehearsal commit input ${commit} is not the head of ${ref} (${sha}); dispatch the ref whose head is the candidate`);
    }
    const requestedVersion = parseReleaseVersion(requested).version;
    if (requestedVersion !== version) {
      fail(`the rehearsal version input ${requestedVersion} does not match the workspace version ${version}`);
    }
    return { mode: "rehearsal", version, tag: tagForVersion(version), pep440: pep440Version(version), commit, ref };
  }
  fail(`release runs start only from a v* tag push or workflow_dispatch, not ${JSON.stringify(event)}`);
}
