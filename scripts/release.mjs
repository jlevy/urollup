#!/usr/bin/env node
// The release command line: every decision the release workflow makes, as tested code.
//
// .github/workflows/release.yml calls one subcommand per step, so the workflow stays thin
// orchestration (docs/project/specs/active/plan-2026-09-16-first-release-publishing.md,
// Workflow and Permissions). Run any subcommand locally with `node scripts/release.mjs`;
// docs/publishing.md describes the local rehearsal and every command's role.
//
//   plan               Resolve version, tag, commit, matrix and prior publication state
//   gate-git           Require the release commit checked out, clean, and on main
//   assert-no-authority  Fail if this job could publish anything
//   build              Build and package one archive or wheel
//   inspect            Check one packaged artifact's contents
//   smoke              Install one artifact into an empty environment and test it
//   crates             The crates.io packaging rehearsal
//   benchmark          Time the static musl and manylinux executables (informational)
//   assemble           Check the complete matrix; write SHA256SUMS, manifest, notes, actions
//   stage-github, publish-crates, publish-pypi, publish-github, probe, evidence,
//   attach-evidence    The tag-only publication path
//   check              Repository release invariants, for `make release-check`

import { spawnSync } from "node:child_process";
import {
  appendFileSync,
  chmodSync,
  copyFileSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  readdirSync,
  realpathSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

import { readArchive, writeTarGz, writeZip } from "./release-archive.mjs";
import { rehearseCrates } from "./release-crates.mjs";
import {
  checkCargoManifests,
  checkPyproject,
  checkVersionOutput,
  parseReleaseVersion,
  pep440Version,
  resolveRunIdentity,
  tagForVersion,
  workspaceVersion,
} from "./release-identity.mjs";
import { checkArchive, checkWheel, sha256 } from "./release-inspect.mjs";
import {
  buildManifest,
  changelogSection,
  checkCompleteness,
  checkReleaseNotes,
  compareChecksums,
  formatChecksums,
  parseChecksums,
  renderReleaseNotes,
} from "./release-manifest.mjs";
import {
  cratesPublishCommand,
  decideCrates,
  decideGithubRelease,
  decidePypi,
  describeDecision,
  LATE_ASSETS,
  observeCrate,
  observeGithubRelease,
  observePypi,
  publishGithubRelease,
  stageGithubRelease,
  USER_AGENT,
} from "./release-registry.mjs";
import { SMOKE_CASES, checkFixtureReport, findPython, isolatedUvEnv, resolveOnPath, smokeInstalled } from "./release-smoke.mjs";
import {
  CARGO_PACKAGES,
  MACOS_DEPLOYMENT_TARGET,
  PRODUCT,
  RELEASE_TARGETS,
  artifactById,
  buildEnvironment,
  buildMatrix,
  executableName,
  plannedArtifacts,
  plannedCrates,
  validateTargets,
} from "./release-targets.mjs";

export const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

export { LATE_ASSETS };

/** Credentials and the OIDC endpoint: a build, validation or assembly job must hold none. */
export const AUTHORITY_VARIABLES = [
  "ACTIONS_ID_TOKEN_REQUEST_URL",
  "ACTIONS_ID_TOKEN_REQUEST_TOKEN",
  "GITHUB_TOKEN",
  "GH_TOKEN",
  "RELEASE_WRITE_TOKEN",
  "CARGO_REGISTRY_TOKEN",
  "UV_PUBLISH_TOKEN",
  "UV_PUBLISH_PASSWORD",
  "PYPI_API_TOKEN",
  "TWINE_PASSWORD",
  "MATURIN_PYPI_TOKEN",
];

function fail(message) {
  throw new Error(message);
}

// ---------------------------------------------------------------------------------------
// Small helpers

export function parseOptions(argv, allowed) {
  const options = {};
  for (let index = 0; index < argv.length; index += 1) {
    const argument = argv[index];
    const match = argument.match(/^--([a-z-]+)$/);
    if (!match || !allowed.includes(match[1])) {
      fail(`unknown option ${argument}; expected ${allowed.map((name) => `--${name}`).join(", ")}`);
    }
    const value = argv[index + 1];
    if (value === undefined || value.startsWith("--")) {
      fail(`--${match[1]} needs a value`);
    }
    options[match[1].replace(/-([a-z])/g, (_, letter) => letter.toUpperCase())] = value;
    index += 1;
  }
  return options;
}

function required(options, name) {
  if (typeof options[name] !== "string" || options[name] === "") {
    fail(`--${name.replace(/[A-Z]/g, (letter) => `-${letter.toLowerCase()}`)} is required`);
  }
  return options[name];
}

function readText(...parts) {
  return readFileSync(path.join(ROOT, ...parts), "utf8");
}

function writeJson(file, value) {
  mkdirSync(path.dirname(file), { recursive: true });
  writeFileSync(file, `${JSON.stringify(value, null, 2)}\n`);
}

function readJson(file) {
  return JSON.parse(readFileSync(file, "utf8"));
}

function exec(program, args, { env = process.env, cwd = ROOT, label, capture = false } = {}) {
  const result = spawnSync(program, args, {
    cwd,
    env,
    encoding: "utf8",
    maxBuffer: 256 * 1024 * 1024,
    stdio: capture ? ["ignore", "pipe", "pipe"] : ["ignore", "inherit", "inherit"],
  });
  if (result.error || result.status !== 0) {
    const detail = capture ? `\n${`${result.stdout ?? ""}${result.stderr ?? ""}`.trim().split("\n").slice(-30).join("\n")}` : "";
    fail(`${label ?? `${program} ${args.join(" ")}`} failed (${result.error?.message ?? `exit ${result.status ?? result.signal}`})${detail}`);
  }
  return result.stdout ?? "";
}

function git(args) {
  return exec("git", args, { capture: true }).trim();
}

/** Append `name=value` lines to the step's GitHub output file, when there is one. */
function setOutputs(outputs) {
  const file = process.env.GITHUB_OUTPUT;
  const lines = Object.entries(outputs).map(([name, value]) => `${name}=${typeof value === "string" ? value : JSON.stringify(value)}`);
  if (file) {
    appendFileSync(file, `${lines.join("\n")}\n`);
  }
}

/** Append Markdown to the run summary, and print it. */
function summary(lines) {
  const text = `${lines.join("\n")}\n`;
  if (process.env.GITHUB_STEP_SUMMARY) {
    appendFileSync(process.env.GITHUB_STEP_SUMMARY, `${text}\n`);
  }
  process.stdout.write(text);
}

/** This host as a release host id, such as linux-x86_64. */
export function hostId(platform = process.platform, arch = process.arch) {
  const osName = { linux: "linux", darwin: "macos", win32: "windows" }[platform] ?? platform;
  const cpu = { x64: "x86_64", arm64: "aarch64" }[arch] ?? arch;
  return `${osName}-${cpu}`;
}

function runnerImage() {
  return process.env.ImageOS ? `${process.env.ImageOS} ${process.env.ImageVersion ?? ""}`.trim() : null;
}

function workflowRun() {
  const { GITHUB_REPOSITORY: repository, GITHUB_RUN_ID: runId, GITHUB_SERVER_URL: server, GITHUB_WORKFLOW_REF: workflowRef } = process.env;
  if (!repository || !runId) {
    return null;
  }
  return { repository, workflow: workflowRef ?? null, run_id: runId, url: `${server ?? "https://github.com"}/${repository}/actions/runs/${runId}` };
}

/** The Rust toolchain rust-toolchain.toml pins. */
export function pinnedToolchain(text = readText("rust-toolchain.toml")) {
  const channel = text.match(/^channel\s*=\s*"([^"]+)"\s*$/m)?.[1];
  if (!channel || !/^\d+\.\d+\.\d+$/.test(channel)) {
    fail("rust-toolchain.toml must pin an exact channel such as 1.98.0");
  }
  const [major, minor] = channel.split(".").map(Number);
  if (major < 1 || (major === 1 && minor < 90)) {
    fail(`the pinned Rust ${channel} is older than 1.90, which workspace packaging and publication require`);
  }
  return channel;
}

function repositoryState() {
  const workspaceToml = readText("Cargo.toml");
  const members = Object.fromEntries(CARGO_PACKAGES.map((crate) => [crate, readText("crates", crate, "Cargo.toml")]));
  const pyproject = readText("crates", "urollup", "pyproject.toml");
  const problems = [...checkCargoManifests({ workspaceToml, members }).problems, ...checkPyproject(pyproject), ...validateTargets()];
  return { workspaceToml, version: workspaceVersion(workspaceToml), toolchain: pinnedToolchain(), problems };
}

function commitTime(commit) {
  const seconds = Number(git(["log", "-1", "--format=%ct", commit]));
  if (!Number.isInteger(seconds) || seconds <= 0) {
    fail(`cannot read the commit time of ${commit}`);
  }
  return seconds;
}

// ---------------------------------------------------------------------------------------
// Registry observation shared by plan, assemble and the publish jobs

function readToken() {
  return process.env.RELEASE_READ_TOKEN || process.env.GITHUB_TOKEN || undefined;
}

async function observeAll({ version, pep440, tag, repository, token, includeDrafts = false }) {
  const crates = {};
  for (const crate of CARGO_PACKAGES) {
    crates[crate] = await observeCrate(crate, version);
  }
  const pypi = await observePypi(PRODUCT, pep440);
  const github = repository ? await observeGithubRelease(repository, tag, { token, includeDrafts }) : { status: "unavailable", reason: "no repository is known outside GitHub Actions" };
  return { crates, pypi, github };
}

function observationSummary(observed) {
  const status = (value) => value.status + (value.status === "unavailable" ? ` (${value.reason})` : "");
  return [
    ...CARGO_PACKAGES.map((crate) => `- crates.io ${crate}: ${status(observed.crates[crate])}`),
    `- PyPI ${PRODUCT}: ${status(observed.pypi)}`,
    `- GitHub Release: ${status(observed.github)}`,
  ];
}

// ---------------------------------------------------------------------------------------
// plan

async function plan(argv) {
  const options = parseOptions(argv, ["out", "event", "ref", "commit", "version", "offline"]);
  const out = required(options, "out");
  const repo = repositoryState();
  if (repo.problems.length > 0) {
    fail(`the repository is not releasable:\n${repo.problems.map((problem) => `- ${problem}`).join("\n")}`);
  }
  const sha = process.env.GITHUB_SHA ?? git(["rev-parse", "HEAD"]);
  const identity = resolveRunIdentity({
    event: process.env.GITHUB_EVENT_NAME ?? options.event ?? "workflow_dispatch",
    ref: process.env.GITHUB_REF ?? options.ref ?? git(["symbolic-ref", "-q", "HEAD"]),
    sha,
    inputs: {
      commit: process.env.RELEASE_INPUT_COMMIT ?? options.commit,
      version: process.env.RELEASE_INPUT_VERSION ?? options.version,
    },
    workspaceToml: repo.workspaceToml,
  });
  const allowUnmerged = identity.mode === "rehearsal" && process.env.RELEASE_INPUT_ALLOW_UNMERGED === "true";
  const sourceDateEpoch = commitTime(identity.commit);
  const observed =
    options.offline === "true"
      ? null
      : await observeAll({ ...identity, repository: process.env.GITHUB_REPOSITORY, token: readToken() });
  const document = {
    schema: "urollup-release-plan/v1",
    ...identity,
    publish: identity.mode === "publish",
    allow_unmerged: allowUnmerged,
    source_date_epoch: sourceDateEpoch,
    rust_toolchain: repo.toolchain,
    macos_deployment_target: MACOS_DEPLOYMENT_TARGET,
    workflow_run: workflowRun(),
    artifacts: plannedArtifacts(identity.version),
    crates: plannedCrates(identity.version),
    prior_publication: observed,
  };
  writeJson(path.join(out, "release-plan.json"), document);
  setOutputs({
    version: identity.version,
    tag: identity.tag,
    commit: identity.commit,
    mode: identity.mode,
    publish: String(document.publish),
    "allow-unmerged": String(allowUnmerged),
    "source-date-epoch": String(sourceDateEpoch),
    matrix: buildMatrix(identity.version),
    hosts: { include: RELEASE_TARGETS.map((target) => ({ host: target.id, runner: target.runner })) },
  });
  summary([
    `## Release plan: ${identity.tag} (${identity.mode})`,
    "",
    `- Commit: \`${identity.commit}\``,
    `- Version: ${identity.version} (PyPI ${identity.pep440})`,
    `- Rust toolchain: ${repo.toolchain}; SOURCE_DATE_EPOCH ${sourceDateEpoch}`,
    `- Publication: ${document.publish ? "enabled after validation and approval" : "none; this is a rehearsal"}`,
    ...(allowUnmerged ? ["- The commit may be off main; this rehearsal is not releasable"] : []),
    `- Artifacts: ${document.artifacts.length} (${document.artifacts.map((artifact) => artifact.name).join(", ")})`,
    "",
    "### Prior publication",
    "",
    ...(observed ? observationSummary(observed) : ["- not observed (--offline)"]),
  ]);
}

// ---------------------------------------------------------------------------------------
// gate-git

function gateGit(argv) {
  const options = parseOptions(argv, ["commit", "allow-unmerged"]);
  const commit = required(options, "commit");
  const head = git(["rev-parse", "HEAD"]);
  const problems = [];
  if (head !== commit) {
    problems.push(`the checkout is at ${head}, not the release commit ${commit}`);
  }
  const status = git(["status", "--porcelain", "--untracked-files=no"]);
  if (status !== "") {
    problems.push(`the tracked tree is not clean:\n${status}`);
  }
  let onMain = false;
  const main = spawnSync("git", ["rev-parse", "--verify", "--quiet", "refs/remotes/origin/main"], { cwd: ROOT, encoding: "utf8" });
  if (main.status !== 0) {
    exec("git", ["fetch", "--no-tags", "origin", "main:refs/remotes/origin/main"], { label: "git fetch origin main" });
  }
  onMain = spawnSync("git", ["merge-base", "--is-ancestor", commit, "refs/remotes/origin/main"], { cwd: ROOT }).status === 0;
  if (!onMain && options.allowUnmerged !== "true") {
    problems.push(`${commit} is not reachable from main; a release commit must be merged first`);
  }
  if (problems.length > 0) {
    fail(problems.join("\n"));
  }
  summary([`- Release commit \`${commit}\` is checked out with a clean tracked tree; on main: ${onMain ? "yes" : "no (rehearsal only, not releasable)"}`]);
}

// ---------------------------------------------------------------------------------------
// assert-no-authority

export function authorityProblems(env) {
  const present = AUTHORITY_VARIABLES.filter((name) => env[name]);
  present.push(...Object.keys(env).filter((name) => /^CARGO_REGISTRIES_.+_TOKEN$/.test(name) && env[name]));
  return present;
}

function assertNoAuthority() {
  const present = authorityProblems(process.env);
  if (present.length > 0) {
    fail(`this job must hold no publishing authority, but its environment has ${present.join(", ")}`);
  }
  console.log("No OIDC token endpoint and no GitHub, crates.io or PyPI credential is present in this job.");
}

// ---------------------------------------------------------------------------------------
// build

function rustcVersion() {
  return exec("rustc", ["-vV"], { capture: true }).trim();
}

function maturinVersion() {
  return readText("crates", "urollup", "pyproject.toml").match(/"maturin==([0-9.]+)"/)?.[1] ?? null;
}

function cargoHome() {
  return process.env.CARGO_HOME ?? path.join(os.homedir(), ".cargo");
}

/** The docker command that builds a manylinux wheel inside the pinned image. */
export function manylinuxCommand({ artifact, root, cargoHomeDir, sysroot, maturin, uid, gid, outDir, env }) {
  const relative = (file) => `/io/${path.relative(root, file).split(path.sep).join("/")}`;
  return [
    "run",
    "--rm",
    "--user",
    `${uid}:${gid}`,
    "--env",
    "HOME=/tmp",
    "--env",
    "CARGO_HOME=/cargo",
    "--env",
    "CARGO_NET_OFFLINE=true",
    "--env",
    "CARGO_TARGET_DIR=/io/target/manylinux",
    "--env",
    `CARGO_ENCODED_RUSTFLAGS=${env.CARGO_ENCODED_RUSTFLAGS.replace(`${cargoHomeDir}=/cargo`, "/cargo=/cargo")}`,
    "--volume",
    `${root}:/io`,
    "--volume",
    `${cargoHomeDir}:/cargo`,
    "--volume",
    `${sysroot}:/opt/rust:ro`,
    "--workdir",
    "/io",
    artifact.container,
    "/bin/bash",
    "-c",
    'export PATH="/opt/rust/bin:$PATH" && exec "$@"',
    "bash",
    relative(maturin),
    "build",
    "--release",
    "--locked",
    "--manifest-path",
    "crates/urollup/Cargo.toml",
    "--target",
    artifact.rustTarget,
    "--compatibility",
    "manylinux2014",
    "--auditwheel",
    "check",
    "--out",
    relative(outDir),
  ];
}

function buildWheel(artifact, env, workDir) {
  // The manylinux container sees only the repository, mounted at /io, so its output stays
  // under target/.
  const outDir = artifact.container ? path.join(ROOT, "target", "release-wheel") : path.join(workDir, "wheel");
  rmSync(outDir, { recursive: true, force: true });
  mkdirSync(outDir, { recursive: true });
  if (artifact.container) {
    exec("uv", ["--config-file", "uv.toml", "sync", "--frozen", "--only-group", "release"], { label: "install the locked Maturin" });
    const maturin = path.join(ROOT, ".venv", "bin", "maturin");
    if (!existsSync(maturin)) {
      fail(`the locked Maturin is not at ${maturin}`);
    }
    exec("cargo", ["fetch", "--locked"], { label: "fetch the locked crates for the offline container build" });
    const sysroot = exec("rustc", ["--print", "sysroot"], { capture: true }).trim();
    exec("docker", manylinuxCommand({ artifact, root: ROOT, cargoHomeDir: cargoHome(), sysroot, maturin, uid: process.getuid(), gid: process.getgid(), outDir, env }), {
      label: `maturin build in ${artifact.container}`,
    });
  } else {
    exec(
      "uv",
      ["--config-file", "uv.toml", "run", "--frozen", "--only-group", "release", "maturin", "build", "--release", "--locked", "--manifest-path", "crates/urollup/Cargo.toml", "--target", artifact.rustTarget, "--compatibility", "pypi", "--out", outDir],
      { env: { ...process.env, ...env }, label: "maturin build" },
    );
  }
  const wheels = readdirSync(outDir).filter((name) => name.endsWith(".whl"));
  if (wheels.length !== 1 || wheels[0] !== artifact.name) {
    fail(`Maturin produced ${wheels.join(", ") || "no wheel"}, expected exactly ${artifact.name}`);
  }
  return readFileSync(path.join(outDir, wheels[0]));
}

function buildArchive(artifact, env, sourceDateEpoch) {
  exec("cargo", ["build", "--locked", "--release", "-p", PRODUCT, "--target", artifact.rustTarget], { env: { ...process.env, ...env }, label: "cargo build" });
  const binary = readFileSync(path.join(process.env.CARGO_TARGET_DIR ?? path.join(ROOT, "target"), artifact.rustTarget, "release", executableName(artifact.os)));
  const entries = [
    { name: executableName(artifact.os), data: binary, mode: 0o755 },
    { name: "README.md", data: readFileSync(path.join(ROOT, "README.md")), mode: 0o644 },
    { name: "LICENSE", data: readFileSync(path.join(ROOT, "LICENSE")), mode: 0o644 },
  ];
  return artifact.os === "windows" ? writeZip(entries, { mtime: sourceDateEpoch }) : writeTarGz(entries, { mtime: sourceDateEpoch });
}

function inspectArtifact(artifact, data, version) {
  return artifact.kind === "archive"
    ? checkArchive(artifact.name, data, artifact, { readme: readFileSync(path.join(ROOT, "README.md")), license: readFileSync(path.join(ROOT, "LICENSE")) })
    : checkWheel(artifact.name, data, artifact, { version });
}

function build(argv) {
  const options = parseOptions(argv, ["artifact", "out"]);
  const out = required(options, "out");
  const version = workspaceVersion(readText("Cargo.toml"));
  const artifact = artifactById(version, required(options, "artifact"));
  const commit = git(["rev-parse", "HEAD"]);
  const sourceDateEpoch = Number(process.env.SOURCE_DATE_EPOCH ?? commitTime(commit));
  if (!Number.isInteger(sourceDateEpoch)) {
    fail(`SOURCE_DATE_EPOCH ${process.env.SOURCE_DATE_EPOCH} is not whole seconds`);
  }
  const env = buildEnvironment(artifact, { cargoHome: cargoHome() });
  const workDir = mkdtempSync(path.join(process.env.RUNNER_TEMP ?? os.tmpdir(), "urollup-build-"));
  const data = artifact.kind === "archive" ? buildArchive(artifact, env, sourceDateEpoch) : buildWheel(artifact, env, workDir);
  const inspected = inspectArtifact(artifact, data, version);
  if (inspected.problems.length > 0) {
    fail(`the built ${artifact.name} fails inspection:\n${inspected.problems.map((problem) => `- ${problem}`).join("\n")}`);
  }
  mkdirSync(out, { recursive: true });
  writeFileSync(path.join(out, artifact.name), data);
  const record = {
    schema: "urollup-release-build/v1",
    artifact: artifact.name,
    id: artifact.id,
    commit,
    sha256: sha256(data),
    size: data.length,
    host: hostId(),
    runner_image: runnerImage(),
    rustc: rustcVersion(),
    builder: artifact.kind === "archive" ? "cargo build" : `maturin ${maturinVersion()}`,
    container: artifact.container ?? null,
    source_date_epoch: sourceDateEpoch,
    environment: Object.fromEntries(Object.entries(env).map(([name, value]) => [name, value.split(cargoHome()).join("$CARGO_HOME")])),
    facts: inspected.facts,
  };
  writeJson(path.join(out, `build-${artifact.id}.json`), record);
  rmSync(workDir, { recursive: true, force: true });
  summary([`- Built ${artifact.name}: ${record.size} bytes, sha256 \`${record.sha256}\``]);
}

// ---------------------------------------------------------------------------------------
// inspect

function inspect(argv) {
  const options = parseOptions(argv, ["artifact", "file"]);
  const version = workspaceVersion(readText("Cargo.toml"));
  const artifact = artifactById(version, required(options, "artifact"));
  const result = inspectArtifact(artifact, readFileSync(required(options, "file")), version);
  console.log(JSON.stringify(result.facts, null, 2));
  if (result.problems.length > 0) {
    fail(result.problems.join("\n"));
  }
}

// ---------------------------------------------------------------------------------------
// smoke

function extractArchiveTo(artifact, data, dir) {
  mkdirSync(dir, { recursive: true });
  for (const entry of readArchive(artifact.name, data)) {
    const target = path.join(dir, entry.name);
    writeFileSync(target, entry.data);
    if (process.platform !== "win32") {
      chmodSync(target, entry.mode & 0o777);
    }
  }
  return path.join(dir, executableName(artifact.os));
}

function uvCommand() {
  const uv = resolveOnPath("uv", process.env.PATH);
  if (!uv) {
    fail("uv is not on PATH");
  }
  return uv;
}

/** Install a wheel with `uv tool install` into isolated directories; returns where it went. */
function installWheel({ wheel, dir, index = false, requirement }) {
  const env = isolatedUvEnv(dir);
  const python = findPython();
  const args = ["tool", "install", ...(python ? ["--python", python] : []), ...(index ? [requirement] : ["--no-index", wheel])];
  exec(uvCommand(), args, { env, label: `uv ${args.join(" ")}`, capture: true });
  const binDir = env.UV_TOOL_BIN_DIR;
  const installed = resolveOnPath("urollup", binDir);
  if (!installed) {
    fail(`uv tool install left no urollup in ${binDir}`);
  }
  return { binDir, toolDir: env.UV_TOOL_DIR, installed };
}

/** Run the ephemeral `uv tool run` path: --version and one fixture report. */
function ephemeralUv({ from, dir, version, index = false }) {
  const env = isolatedUvEnv(dir);
  const python = findPython();
  const base = ["tool", "run", "--isolated", ...(python ? ["--python", python] : []), ...(index ? [] : ["--no-index"]), "--from", from, PRODUCT];
  const problems = [];
  const versionRun = spawnSync(uvCommand(), [...base, "--version"], { env, encoding: "utf8", timeout: 300_000 });
  problems.push(...(versionRun.status === 0 ? checkVersionOutput(versionRun.stdout, version) : [`uv tool run urollup --version exited ${versionRun.status}: ${versionRun.stderr}`]));
  const fixtureCase = SMOKE_CASES[0];
  const caseDir = path.join(ROOT, "crates", "urollup-core", "tests", "fixtures", ...fixtureCase.split("/"));
  const reportRun = spawnSync(uvCommand(), [...base, "report", "--source", caseDir, "--no-default-sources", "--format", "json", "--timezone", "UTC"], {
    env: { ...env, NO_COLOR: "1", CLAUDE_CONFIG_DIR: path.join(dir, "empty"), CODEX_HOME: path.join(dir, "empty") },
    encoding: "utf8",
    timeout: 300_000,
  });
  problems.push(...checkFixtureReport(fixtureCase, reportRun, { allowStderr: true }).map((problem) => `uv tool run: ${problem}`));
  return { checks: [{ check: "uv-tool-run", from: path.basename(from), version: versionRun.stdout.trim() }], problems };
}

function smoke(argv) {
  const options = parseOptions(argv, ["artifact", "input", "out"]);
  const version = workspaceVersion(readText("Cargo.toml"));
  const artifact = artifactById(version, required(options, "artifact"));
  const input = required(options, "input");
  const out = required(options, "out");
  const file = path.join(input, artifact.name);
  const record = {
    schema: "urollup-release-validation/v1",
    artifact: artifact.name,
    id: artifact.id,
    host: hostId(),
    runner_image: runnerImage(),
    status: "failed",
    checks: [],
    problems: [],
  };
  const finish = () => {
    writeJson(path.join(out, `validation-${artifact.id}.json`), record);
    summary([`- ${artifact.name} on ${record.host}: ${record.status}`, ...record.problems.map((problem) => `  - ${problem}`)]);
    if (record.status !== "passed") {
      fail(`${artifact.name} failed validation`);
    }
  };
  if (!existsSync(file)) {
    record.problems.push(`${artifact.name} was not downloaded into ${input}`);
    return finish();
  }
  const data = readFileSync(file);
  record.sha256 = sha256(data);
  record.size = data.length;
  if (record.host !== artifact.host) {
    record.problems.push(`${artifact.name} must be validated on its native ${artifact.host} host, not ${record.host}`);
  }
  const buildRecordFile = path.join(input, `build-${artifact.id}.json`);
  if (existsSync(buildRecordFile) && readJson(buildRecordFile).sha256 !== record.sha256) {
    record.problems.push(`${artifact.name} differs from the bytes its build job recorded`);
  }
  const inspected = inspectArtifact(artifact, data, version);
  record.facts = inspected.facts;
  record.problems.push(...inspected.problems);
  if (record.problems.length > 0) {
    return finish();
  }
  record.checks.push({ check: "inspection", facts: inspected.facts });

  const work = mkdtempSync(path.join(process.env.RUNNER_TEMP ?? os.tmpdir(), "urollup-smoke-"));
  try {
    if (artifact.kind === "archive") {
      const installDir = path.join(work, "install");
      const executable = extractArchiveTo(artifact, data, installDir);
      const result = smokeInstalled({ expected: executable, installDirs: [installDir], version, label: "archive" });
      record.checks.push(...result.checks);
      record.problems.push(...result.problems);
    } else {
      const ephemeral = ephemeralUv({ from: file, dir: path.join(work, "ephemeral"), version });
      record.checks.push(...ephemeral.checks);
      record.problems.push(...ephemeral.problems);
      const installed = installWheel({ wheel: file, dir: path.join(work, "persistent") });
      const installedDigest = sha256(readFileSync(realpathSync(installed.installed)));
      if (installedDigest !== inspected.facts.executable_sha256) {
        record.problems.push(`uv tool install put a urollup on PATH whose digest ${installedDigest} is not the wheel's script payload ${inspected.facts.executable_sha256}`);
      }
      record.checks.push({ check: "uv-tool-install", installed: path.basename(installed.installed), executable_sha256: installedDigest });
      const result = smokeInstalled({ expected: installed.installed, installDirs: [installed.binDir], version, label: "uv tool install" });
      record.checks.push(...result.checks);
      record.problems.push(...result.problems);
    }
  } catch (error) {
    record.problems.push(error.message);
  } finally {
    rmSync(work, { recursive: true, force: true });
  }
  record.status = record.problems.length === 0 ? "passed" : "failed";
  return finish();
}

// ---------------------------------------------------------------------------------------
// crates

function crates(argv) {
  const options = parseOptions(argv, ["out", "work", "allow-dirty"]);
  const out = required(options, "out");
  const version = workspaceVersion(readText("Cargo.toml"));
  const commit = git(["rev-parse", "HEAD"]);
  const work = options.work ?? mkdtempSync(path.join(process.env.RUNNER_TEMP ?? os.tmpdir(), "urollup-crates-"));
  const { record, problems } = rehearseCrates({
    root: ROOT,
    commit,
    version,
    work,
    out,
    allowDirty: options.allowDirty === "true",
    smoke: (executable, installDirs) => smokeInstalled({ expected: executable, installDirs, version, label: "cargo install" }),
  });
  if (problems.length > 0) {
    fail(`the crates.io rehearsal failed:\n${problems.map((problem) => `- ${problem}`).join("\n")}`);
  }
  summary([
    "### Cargo packages",
    "",
    ...record.packages.map((item) => `- ${item.name}: ${item.files.length} files, ${item.size} bytes, sha256 \`${item.sha256}\``),
    ...record.checks.filter((check) => typeof check.check === "string" && !check.case).map((check) => `- ${check.check}`),
  ]);
  rmSync(work, { recursive: true, force: true });
}

// ---------------------------------------------------------------------------------------
// benchmark (informational: evidence for the allocator decision)

function median(values) {
  const sorted = [...values].sort((a, b) => a - b);
  return sorted[Math.floor(sorted.length / 2)];
}

function benchmark(argv) {
  const options = parseOptions(argv, ["input", "out", "runs", "max-bytes"]);
  const input = required(options, "input");
  const out = required(options, "out");
  const runs = Number(options.runs ?? 5);
  const version = workspaceVersion(readText("Cargo.toml"));
  const archive = artifactById(version, "archive-linux-x86_64");
  const wheel = artifactById(version, "wheel-linux-x86_64");
  const work = mkdtempSync(path.join(process.env.RUNNER_TEMP ?? os.tmpdir(), "urollup-benchmark-"));
  const musl = extractArchiveTo(archive, readFileSync(path.join(input, archive.name)), path.join(work, "musl"));
  const glibcCheck = checkWheel(wheel.name, readFileSync(path.join(input, wheel.name)), wheel, { version });
  const glibc = path.join(work, "glibc", PRODUCT);
  mkdirSync(path.dirname(glibc), { recursive: true });
  writeFileSync(glibc, glibcCheck.executable);
  chmodSync(glibc, 0o755);
  const corpus = path.join(work, "corpus");
  exec("uv", ["--config-file", "uv.toml", "run", "--frozen", "python", "scripts/generate-synthetic-corpus.py", "--out", corpus, "--max-bytes", options.maxBytes ?? String(256 * 1024 * 1024), "--seed", "1"], {
    label: "generate the synthetic corpus",
  });
  // One root per agent, as the scale gate passes them (scripts/measure-scale.py).
  const args = ["report", "--all", "--source", path.join(corpus, "claude", "projects"), "--source", path.join(corpus, "codex"), "--no-default-sources", "--format", "json", "--timezone", "UTC"];
  const time = existsSync("/usr/bin/time") ? "/usr/bin/time" : null;
  const samples = { musl: [], glibc: [] };
  const rss = { musl: [], glibc: [] };
  for (let index = 0; index < runs; index += 1) {
    for (const [name, binary] of index % 2 === 0 ? [["musl", musl], ["glibc", glibc]] : [["glibc", glibc], ["musl", musl]]) {
      const started = process.hrtime.bigint();
      const result = time
        ? spawnSync(time, ["-f", "%M", binary, ...args], { encoding: "utf8", maxBuffer: 512 * 1024 * 1024, env: { ...process.env, NO_COLOR: "1" } })
        : spawnSync(binary, args, { encoding: "utf8", maxBuffer: 512 * 1024 * 1024, env: { ...process.env, NO_COLOR: "1" } });
      const seconds = Number(process.hrtime.bigint() - started) / 1e9;
      if (result.status !== 0) {
        fail(`${name} report exited ${result.status}: ${result.stderr}`);
      }
      samples[name].push(seconds);
      const kib = Number(result.stderr.trim().split("\n").pop());
      if (Number.isFinite(kib)) {
        rss[name].push(kib);
      }
    }
  }
  const record = {
    schema: "urollup-release-benchmark/v1",
    host: hostId(),
    runner_image: runnerImage(),
    command: `urollup ${args.join(" ").split(corpus).join("<corpus>")}`,
    corpus: { generator: "scripts/generate-synthetic-corpus.py", seed: 1, max_bytes: Number(options.maxBytes ?? 256 * 1024 * 1024) },
    runs,
    musl: { median_seconds: median(samples.musl), samples: samples.musl, median_peak_rss_kib: rss.musl.length ? median(rss.musl) : null },
    glibc: { median_seconds: median(samples.glibc), samples: samples.glibc, median_peak_rss_kib: rss.glibc.length ? median(rss.glibc) : null },
  };
  record.musl_over_glibc = record.musl.median_seconds / record.glibc.median_seconds;
  writeJson(path.join(out, "benchmark.json"), record);
  rmSync(work, { recursive: true, force: true });
  summary([
    "### musl and glibc executables on one synthetic corpus",
    "",
    "| Build | Median seconds | Median peak RSS (KiB) |",
    "| --- | --- | --- |",
    `| static musl archive | ${record.musl.median_seconds.toFixed(2)} | ${record.musl.median_peak_rss_kib ?? "n/a"} |`,
    `| manylinux wheel | ${record.glibc.median_seconds.toFixed(2)} | ${record.glibc.median_peak_rss_kib ?? "n/a"} |`,
    "",
    `musl takes ${record.musl_over_glibc.toFixed(2)}x the glibc time over ${runs} alternating runs.`,
  ]);
}

// ---------------------------------------------------------------------------------------
// assemble

function findFile(directory, name) {
  if (!existsSync(directory)) {
    return null;
  }
  for (const entry of readdirSync(directory, { withFileTypes: true })) {
    const full = path.join(directory, entry.name);
    if (entry.isDirectory()) {
      const found = findFile(full, name);
      if (found) {
        return found;
      }
    } else if (entry.name === name) {
      return full;
    }
  }
  return null;
}

/** The GitHub Release assets a bundle stages: archives, wheels, SHA256SUMS and the manifest. */
function bundleAssets(bundle) {
  const assets = readdirSync(path.join(bundle, "assets"))
    .sort()
    .map((name) => ({ name, file: path.join(bundle, "assets", name) }));
  return [...assets, { name: "SHA256SUMS", file: path.join(bundle, "SHA256SUMS") }, { name: "release-manifest.json", file: path.join(bundle, "release-manifest.json") }].map((asset) => {
    const data = readFileSync(asset.file);
    return { ...asset, data, sha256: sha256(data) };
  });
}

/** Channel decisions for a bundle against current registry state. */
async function channelDecisions({ manifest, assets, repository, token, includeDrafts }) {
  const observed = await observeAll({ version: manifest.version, pep440: manifest.pep440_version, tag: manifest.tag, repository, token, includeDrafts });
  const wheels = manifest.artifacts.filter((artifact) => artifact.kind === "wheel").map((artifact) => ({ name: artifact.name, sha256: artifact.sha256 }));
  const crates = manifest.cargo_packages.map((item) => ({ name: item.name, sha256: item.sha256 }));
  return {
    observed,
    decisions: [
      decideCrates(crates, observed.crates),
      decidePypi(wheels, observed.pypi),
      decideGithubRelease(
        assets.map((asset) => ({ name: asset.name, sha256: asset.sha256 })),
        observed.github,
      ),
    ],
  };
}

async function assemble(argv) {
  const options = parseOptions(argv, ["input", "out"]);
  const input = required(options, "input");
  const out = required(options, "out");
  const planFile = findFile(input, "release-plan.json");
  if (!planFile) {
    fail(`no release-plan.json under ${input}`);
  }
  const planned = readJson(planFile);
  const identity = { mode: planned.mode, version: planned.version, tag: planned.tag, pep440: planned.pep440, commit: planned.commit };
  const problems = [];
  if (git(["rev-parse", "HEAD"]) !== identity.commit) {
    problems.push(`the assemble checkout is not the planned commit ${identity.commit}`);
  }
  const artifacts = [];
  const builds = [];
  const validations = [];
  for (const artifact of plannedArtifacts(identity.version)) {
    const file = findFile(path.join(input, artifact.id), artifact.name);
    const buildFile = findFile(path.join(input, artifact.id), `build-${artifact.id}.json`);
    const validationFile = findFile(path.join(input, `validation-${artifact.id}`), `validation-${artifact.id}.json`);
    if (file) {
      const data = readFileSync(file);
      artifacts.push({ name: artifact.name, file, sha256: sha256(data), size: data.length });
    }
    if (buildFile) {
      const build = readJson(buildFile);
      builds.push(build);
      if (build.commit !== identity.commit) {
        problems.push(`${artifact.name} was built from ${build.commit}, not ${identity.commit}`);
      }
      const assembled = artifacts.find((candidate) => candidate.name === artifact.name);
      if (assembled && build.sha256 !== assembled.sha256) {
        problems.push(`${artifact.name} differs from the bytes its build job recorded`);
      }
    } else {
      problems.push(`${artifact.name} has no build record`);
    }
    if (validationFile) {
      validations.push(readJson(validationFile));
    }
  }
  const cratesRecordFile = findFile(path.join(input, "crates"), "crates.json");
  const crateFiles = [];
  if (!cratesRecordFile) {
    problems.push("the crates.io rehearsal left no record");
  } else {
    const record = readJson(cratesRecordFile);
    if (record.status !== "passed" || record.commit !== identity.commit) {
      problems.push(`the crates.io rehearsal record is ${record.status} for ${record.commit}`);
    }
    for (const item of plannedCrates(identity.version)) {
      const file = findFile(path.join(input, "crates"), item.name);
      if (file) {
        const data = readFileSync(file);
        const recorded = record.packages.find((candidate) => candidate.name === item.name);
        if (recorded?.sha256 !== sha256(data)) {
          problems.push(`${item.name} differs from the crates.io rehearsal record`);
        }
        crateFiles.push({ name: item.name, crate: item.crate, file, sha256: sha256(data), size: data.length });
      }
    }
  }
  problems.push(...checkCompleteness({ version: identity.version, artifacts, validations, crates: crateFiles }));
  const section = changelogSection(readText("CHANGELOG.md"), identity.version);
  problems.push(...checkReleaseNotes(section, { mode: identity.mode }));
  if (problems.length > 0) {
    fail(`the release cannot be assembled:\n${problems.map((problem) => `- ${problem}`).join("\n")}`);
  }

  rmSync(out, { recursive: true, force: true });
  mkdirSync(path.join(out, "assets"), { recursive: true });
  mkdirSync(path.join(out, "crates"), { recursive: true });
  mkdirSync(path.join(out, "validation"), { recursive: true });
  for (const artifact of artifacts) {
    copyFileSync(artifact.file, path.join(out, "assets", artifact.name));
  }
  for (const crate of crateFiles) {
    copyFileSync(crate.file, path.join(out, "crates", crate.name));
  }
  for (const validation of validations) {
    writeJson(path.join(out, "validation", `validation-${validation.id}.json`), validation);
  }
  copyFileSync(planFile, path.join(out, "release-plan.json"));
  const checksums = formatChecksums(artifacts);
  writeFileSync(path.join(out, "SHA256SUMS"), checksums);
  const checkProblems = compareChecksums(parseChecksums(checksums), artifacts);
  if (checkProblems.length > 0) {
    fail(checkProblems.join("\n"));
  }
  const run = workflowRun();
  const manifest = buildManifest({
    identity,
    toolchain: planned.rust_toolchain,
    sourceDateEpoch: planned.source_date_epoch,
    workflowRun: run,
    artifacts,
    builds,
    validations,
    crates: crateFiles,
  });
  writeJson(path.join(out, "release-manifest.json"), manifest);
  const repository = process.env.GITHUB_REPOSITORY ?? "jlevy/urollup";
  writeFileSync(path.join(out, "release-notes.md"), renderReleaseNotes({ identity, section, artifacts: manifest.artifacts, repository }));

  const assets = bundleAssets(out);
  const { observed, decisions } = await channelDecisions({ manifest, assets, repository: process.env.GITHUB_REPOSITORY, token: readToken(), includeDrafts: false });
  const actions = {
    schema: "urollup-release-channel-actions/v1",
    mode: identity.mode,
    tag: identity.tag,
    observed_at: new Date().toISOString(),
    note: "Drafts are visible only to the publish jobs' write token; stage-github repeats this decision with drafts included.",
    observed,
    decisions,
  };
  writeJson(path.join(out, "channel-actions.json"), actions);
  // Unknown registry state stops a rehearsal too: a rehearsal must show the exact actions a
  // publication would take, and it cannot while a registry is unreadable.
  const blocking = decisions.filter((decision) => decision.blocking);
  summary([
    `## Release bundle: ${identity.tag} (${identity.mode})`,
    "",
    `All ${artifacts.length} archives and wheels passed native validation, and both Cargo packages passed the crates.io rehearsal.`,
    "",
    "| Artifact | Bytes | SHA-256 |",
    "| --- | --- | --- |",
    ...manifest.artifacts.map((artifact) => `| \`${artifact.name}\` | ${artifact.size} | \`${artifact.sha256}\` |`),
    ...manifest.cargo_packages.map((item) => `| \`${item.file}\` | ${item.size} | \`${item.sha256}\` |`),
    "",
    "### Channel actions",
    "",
    ...decisions.map((decision) => `- ${describeDecision(decision)}`),
    ...(identity.mode === "rehearsal" ? ["", "This rehearsal performs none of these actions."] : []),
  ]);
  if (blocking.length > 0) {
    fail(`publication would stop:\n${blocking.map((decision) => `- ${describeDecision(decision)}`).join("\n")}`);
  }
}

// ---------------------------------------------------------------------------------------
// Publication (tag runs only, inside the protected release environment)

function requirePublishRun(manifest) {
  if (manifest.mode !== "publish" || process.env.GITHUB_REF !== `refs/tags/${manifest.tag}`) {
    fail(`publication runs only from the pushed tag ${manifest.tag}, in a publish-mode run`);
  }
  if (git(["rev-parse", "HEAD"]) !== manifest.commit) {
    fail(`the checkout is not the release commit ${manifest.commit}`);
  }
}

function loadBundle(bundle) {
  const manifest = readJson(path.join(bundle, "release-manifest.json"));
  const assets = bundleAssets(bundle);
  const sums = parseChecksums(readFileSync(path.join(bundle, "SHA256SUMS"), "utf8"));
  const problems = compareChecksums(
    sums,
    assets.filter((asset) => asset.name !== "SHA256SUMS" && asset.name !== "release-manifest.json"),
  );
  if (problems.length > 0) {
    fail(`the release bundle does not match its SHA256SUMS:\n${problems.join("\n")}`);
  }
  return { manifest, assets };
}

function writeToken() {
  const token = process.env.RELEASE_WRITE_TOKEN;
  if (!token) {
    fail("RELEASE_WRITE_TOKEN is not set");
  }
  return token;
}

async function stageGithub(argv) {
  const bundle = required(parseOptions(argv, ["bundle"]), "bundle");
  const { manifest, assets } = loadBundle(bundle);
  requirePublishRun(manifest);
  const result = await stageGithubRelease({
    repository: process.env.GITHUB_REPOSITORY,
    tag: manifest.tag,
    commit: manifest.commit,
    title: `${PRODUCT} ${manifest.version}`,
    notes: readFileSync(path.join(bundle, "release-notes.md"), "utf8"),
    prerelease: parseReleaseVersion(manifest.version).prerelease !== null,
    files: assets,
    token: writeToken(),
  });
  summary([`- GitHub Release draft ${result.releaseId}: ${describeDecision(result.staged)}`]);
}

async function waitFor(check, { attempts = 20, delayMs = 15_000 } = {}) {
  let last;
  for (let attempt = 0; attempt < attempts; attempt += 1) {
    last = await check();
    if (last.state === "identical-complete" || last.state === "conflict") {
      return last;
    }
    await new Promise((resolve) => setTimeout(resolve, delayMs));
  }
  return last;
}

async function publishCrates(argv) {
  const bundle = required(parseOptions(argv, ["bundle"]), "bundle");
  const { manifest } = loadBundle(bundle);
  requirePublishRun(manifest);
  const planned = manifest.cargo_packages.map((item) => ({ name: item.name, sha256: item.sha256 }));
  const decide = async () => {
    const observed = {};
    for (const item of planned) {
      observed[item.name] = await observeCrate(item.name, manifest.version);
    }
    return decideCrates(planned, observed);
  };
  const before = await decide();
  if (before.blocking) {
    fail(`refusing to publish to crates.io: ${describeDecision(before)}`);
  }
  const command = cratesPublishCommand(before);
  if (command) {
    // `cargo publish` packages again; prove first that this commit packages to exactly the
    // rehearsed bytes, so an upload can never differ from the manifest.
    const target = mkdtempSync(path.join(process.env.RUNNER_TEMP ?? os.tmpdir(), "urollup-publish-package-"));
    exec("cargo", ["package", "--locked", "--workspace", "--no-verify"], { env: { ...process.env, CARGO_TARGET_DIR: target }, label: "cargo package (pre-publish digest check)" });
    for (const item of manifest.cargo_packages) {
      const digest = sha256(readFileSync(path.join(target, "package", item.file)));
      if (digest !== item.sha256) {
        fail(`${item.file} packages to ${digest}, not the rehearsed ${item.sha256}; refusing to publish`);
      }
    }
    await recordingChannelState("crates.io", decide, () => exec(command[0], command.slice(1), { label: command.join(" ") }));
  }
  const after = await waitFor(decide);
  if (after.state !== "identical-complete") {
    fail(`crates.io does not show the planned packages after publishing: ${describeDecision(after)}`);
  }
  summary([`- crates.io: ${describeDecision(before)}; now ${after.state}`]);
}

/**
 * Run a publishing command; if it fails, observe the registry again and record exactly
 * what it now holds before failing, so a partial publication is never left unrecorded.
 */
async function recordingChannelState(channel, decide, publish) {
  try {
    publish();
  } catch (error) {
    let state;
    try {
      state = describeDecision(await decide());
    } catch (observeError) {
      state = `unknown (${observeError.message})`;
    }
    summary([`- ${channel} publishing failed; the registry now reads: ${state}`]);
    throw error;
  }
}

async function publishPypi(argv) {
  const bundle = required(parseOptions(argv, ["bundle"]), "bundle");
  const { manifest } = loadBundle(bundle);
  requirePublishRun(manifest);
  const planned = manifest.artifacts.filter((artifact) => artifact.kind === "wheel").map((artifact) => ({ name: artifact.name, sha256: artifact.sha256 }));
  const decide = async () => decidePypi(planned, await observePypi(PRODUCT, manifest.pep440_version));
  const before = await decide();
  if (before.blocking) {
    fail(`refusing to publish to PyPI: ${describeDecision(before)}`);
  }
  if (before.publish.length > 0) {
    const files = before.publish.map((name) => path.join(bundle, "assets", name));
    // No configuration file: uv.toml's resolution cool-off would hide the just-uploaded
    // files from --check-url, and publishing must not read user configuration either.
    await recordingChannelState("PyPI", decide, () =>
      exec("uv", ["publish", "--trusted-publishing", "always", "--check-url", "https://pypi.org/simple/", ...files], {
        env: { ...process.env, UV_NO_CONFIG: "1" },
        label: "uv publish",
      }),
    );
  }
  const after = await waitFor(decide);
  if (after.state !== "identical-complete") {
    fail(`PyPI does not show the planned wheels after publishing: ${describeDecision(after)}`);
  }
  summary([`- PyPI: ${describeDecision(before)}; now ${after.state}`]);
}

async function publishGithub(argv) {
  const bundle = required(parseOptions(argv, ["bundle"]), "bundle");
  const { manifest, assets } = loadBundle(bundle);
  requirePublishRun(manifest);
  const { decisions } = await channelDecisions({ manifest, assets, repository: process.env.GITHUB_REPOSITORY, token: writeToken(), includeDrafts: true });
  const registries = decisions.filter((decision) => decision.channel !== "github");
  if (registries.some((decision) => decision.state !== "identical-complete")) {
    fail(`both registries must verify before the release is public:\n${registries.map(describeDecision).join("\n")}`);
  }
  const result = await publishGithubRelease({ repository: process.env.GITHUB_REPOSITORY, tag: manifest.tag, files: assets, token: writeToken() });
  summary([`- GitHub Release ${result.releaseId}: ${result.alreadyPublic ? "already public" : "published"}`]);
}

// ---------------------------------------------------------------------------------------
// Post-publication probes and evidence

/** Retry an installation while a registry's index catches up with a new publication. */
async function retrying(attempt, { attempts = 10, delayMs = 30_000 } = {}) {
  for (let index = 1; ; index += 1) {
    try {
      return attempt();
    } catch (error) {
      if (index >= attempts) {
        throw error;
      }
      await new Promise((resolve) => setTimeout(resolve, delayMs));
    }
  }
}

async function download(url, file) {
  for (let attempt = 0; attempt < 10; attempt += 1) {
    const response = await fetch(url, { headers: { "User-Agent": USER_AGENT } });
    if (response.ok) {
      writeFileSync(file, Buffer.from(await response.arrayBuffer()));
      return;
    }
    await new Promise((resolve) => setTimeout(resolve, 15_000));
  }
  fail(`could not download ${url}`);
}

async function probe(argv) {
  const options = parseOptions(argv, ["host", "bundle", "out"]);
  const host = required(options, "host");
  const { manifest } = loadBundle(required(options, "bundle"));
  const out = required(options, "out");
  if (hostId() !== host) {
    fail(`probe ${host} must run on its native host, not ${hostId()}`);
  }
  const repository = process.env.GITHUB_REPOSITORY;
  const work = mkdtempSync(path.join(process.env.RUNNER_TEMP ?? os.tmpdir(), "urollup-probe-"));
  const record = { schema: "urollup-release-probe/v1", host, version: manifest.version, channels: {}, problems: [] };
  const archive = manifest.artifacts.find((artifact) => artifact.kind === "archive" && artifact.platform === host);
  const wheel = manifest.artifacts.find((artifact) => artifact.kind === "wheel" && artifact.platform === host);
  const targetArchive = artifactById(manifest.version, `archive-${host}`);
  const channel = async (name, body) => {
    const problems = [];
    let checks = [];
    try {
      const result = await body();
      checks = result.checks;
      problems.push(...result.problems);
    } catch (error) {
      problems.push(error.message);
    }
    record.channels[name] = { status: problems.length === 0 ? "passed" : "failed", checks, problems };
    record.problems.push(...problems.map((problem) => `${name}: ${problem}`));
  };
  await channel("github", async () => {
    const dir = path.join(work, "github");
    mkdirSync(dir, { recursive: true });
    const base = `https://github.com/${repository}/releases/download/${manifest.tag}`;
    const file = path.join(dir, archive.name);
    await download(`${base}/${archive.name}`, file);
    await download(`${base}/SHA256SUMS`, path.join(dir, "SHA256SUMS"));
    const data = readFileSync(file);
    const sums = parseChecksums(readFileSync(path.join(dir, "SHA256SUMS"), "utf8"));
    const problems = compareChecksums(new Map([[archive.name, sums.get(archive.name)]]), [{ name: archive.name, sha256: sha256(data) }]);
    if (sha256(data) !== archive.sha256) {
      problems.push(`the downloaded ${archive.name} is not the manifest's ${archive.sha256}`);
    }
    exec("gh", ["attestation", "verify", file, "--repo", repository], { label: `gh attestation verify ${archive.name}`, capture: true });
    const installDir = path.join(dir, "install");
    const executable = extractArchiveTo(targetArchive, data, installDir);
    const smoked = smokeInstalled({ expected: executable, installDirs: [installDir], version: manifest.version, label: "GitHub archive" });
    return { checks: [{ check: "sha256-and-attestation", file: archive.name }, ...smoked.checks], problems: [...problems, ...smoked.problems] };
  });
  await channel("pypi", async () => {
    const dir = path.join(work, "pypi");
    const ephemeral = ephemeralUv({ from: `${PRODUCT}@${manifest.pep440_version}`, dir: path.join(dir, "uvx"), version: manifest.version, index: true });
    const installed = await retrying(() => installWheel({ dir: path.join(dir, "install"), index: true, requirement: `${PRODUCT}==${manifest.pep440_version}` }));
    const smoked = smokeInstalled({ expected: installed.installed, installDirs: [installed.binDir], version: manifest.version, label: "uv tool install" });
    const digest = sha256(readFileSync(realpathSync(installed.installed)));
    const problems = [...ephemeral.problems, ...smoked.problems];
    if (digest !== wheel.executable_sha256) {
      problems.push(`the PyPI-installed urollup is ${digest}, not the validated wheel payload ${wheel.executable_sha256}`);
    }
    return { checks: [...ephemeral.checks, ...smoked.checks], problems };
  });
  await channel("crates", async () => {
    const root = path.join(work, "cargo-root");
    await retrying(() =>
      exec("cargo", ["install", "--locked", `${PRODUCT}@${manifest.version}`, "--root", root], {
        env: { ...process.env, CARGO_TARGET_DIR: path.join(work, "cargo-target") },
        label: "cargo install --locked",
        capture: true,
      }),
    );
    const binDir = path.join(root, "bin");
    return smokeInstalled({ expected: path.join(binDir, executableName(targetArchive.os)), installDirs: [binDir], version: manifest.version, label: "cargo install" });
  });
  record.status = record.problems.length === 0 ? "passed" : "failed";
  writeJson(path.join(out, `probe-${host}.json`), record);
  rmSync(work, { recursive: true, force: true });
  summary([`- ${host}: ${record.status}`, ...record.problems.map((problem) => `  - ${problem}`)]);
  if (record.status !== "passed") {
    fail(`installation probes failed on ${host}`);
  }
}

async function evidence(argv) {
  const options = parseOptions(argv, ["input", "out"]);
  const input = required(options, "input");
  const out = required(options, "out");
  const bundle = path.dirname(findFile(input, "release-manifest.json") ?? fail("no release bundle under the input"));
  const { manifest, assets } = loadBundle(bundle);
  const probes = RELEASE_TARGETS.map((target) => {
    const file = findFile(input, `probe-${target.id}.json`);
    return file ? readJson(file) : { host: target.id, status: "missing" };
  });
  const { decisions } = await channelDecisions({ manifest, assets, repository: process.env.GITHUB_REPOSITORY, token: readToken(), includeDrafts: false });
  const incomplete = [
    ...probes.filter((record) => record.status !== "passed").map((record) => `probe ${record.host} is ${record.status}`),
    ...decisions.filter((decision) => decision.state !== "identical-complete").map(describeDecision),
  ];
  if (incomplete.length > 0) {
    fail(`release evidence is incomplete, so none is written:\n${incomplete.join("\n")}`);
  }
  const record = {
    schema: "urollup-release-evidence/v1",
    version: manifest.version,
    tag: manifest.tag,
    commit: manifest.commit,
    manifest_sha256: assets.find((asset) => asset.name === "release-manifest.json").sha256,
    workflow_run: workflowRun(),
    channels: Object.fromEntries(decisions.map((decision) => [decision.channel, { state: decision.state, verified: decision.skip }])),
    probes,
    complete: true,
  };
  writeJson(path.join(out, "release-evidence.json"), record);
  summary([`- Release evidence written for ${manifest.tag}: every channel and probe passed`]);
}

async function attachEvidence(argv) {
  const input = required(parseOptions(argv, ["input"]), "input");
  const bundle = path.dirname(findFile(input, "release-manifest.json") ?? fail("no release bundle under the input"));
  const { manifest } = loadBundle(bundle);
  requirePublishRun(manifest);
  const file = findFile(input, "release-evidence.json") ?? fail("no release-evidence.json under the input");
  const data = readFileSync(file);
  const observed = await observeGithubRelease(process.env.GITHUB_REPOSITORY, manifest.tag, { token: writeToken(), includeDrafts: true });
  if (observed.status !== "present" || observed.draft) {
    fail(`the ${manifest.tag} release must be public before its evidence is attached`);
  }
  const existing = observed.assets.find((asset) => asset.name === "release-evidence.json");
  if (existing) {
    if (existing.sha256 !== sha256(data)) {
      fail("a different release-evidence.json is already attached; evidence is never replaced");
    }
    summary(["- release-evidence.json is already attached and identical"]);
    return;
  }
  const response = await fetch(`https://uploads.github.com/repos/${process.env.GITHUB_REPOSITORY}/releases/${observed.id}/assets?name=release-evidence.json`, {
    method: "POST",
    headers: { "User-Agent": USER_AGENT, Authorization: `Bearer ${writeToken()}`, "Content-Type": "application/json", Accept: "application/vnd.github+json" },
    body: data,
  });
  if (!response.ok) {
    fail(`attaching release-evidence.json returned HTTP ${response.status}`);
  }
  summary(["- Attached release-evidence.json"]);
}

// ---------------------------------------------------------------------------------------
// rehearse-host (make release-rehearsal) and dispatch (make release-dispatch)

/**
 * The local single-host rehearsal: this host's archive and wheel, built, inspected,
 * installed into empty environments and smoke-tested, then the crates.io rehearsal. A
 * dirty tree is allowed and recorded, since a local rehearsal is never a release.
 */
async function rehearseHost(argv) {
  const out = path.resolve(required(parseOptions(argv, ["out"]), "out"));
  const version = workspaceVersion(readText("Cargo.toml"));
  const host = hostId();
  const artifacts = plannedArtifacts(version).filter((artifact) => artifact.host === host);
  if (artifacts.length === 0) {
    fail(`${host} is not a release host; the release hosts are ${RELEASE_TARGETS.map((target) => target.id).join(", ")}`);
  }
  const dirty = git(["status", "--porcelain", "--untracked-files=no"]) !== "";
  rmSync(out, { recursive: true, force: true });
  await plan(["--out", path.join(out, "plan"), "--event", "workflow_dispatch", "--commit", git(["rev-parse", "HEAD"]), "--version", version, "--offline", "true"]);
  for (const artifact of artifacts) {
    const dir = path.join(out, artifact.id);
    build(["--artifact", artifact.id, "--out", dir]);
    smoke(["--artifact", artifact.id, "--input", dir, "--out", path.join(out, `validation-${artifact.id}`)]);
  }
  crates(["--out", path.join(out, "crates"), "--work", path.join(out, "crates-work"), "--allow-dirty", String(dirty)]);
  summary([
    "",
    `Local rehearsal of ${host} passed: ${artifacts.map((artifact) => artifact.name).join(", ")} and both Cargo packages${dirty ? " (from a dirty tree, so not a release candidate)" : ""}.`,
    "The other hosts and the assemble step run only in CI: make release-dispatch.",
  ]);
}

/** The `gh workflow run` arguments for a rehearsal of a pushed commit. */
export function dispatchArgs({ branch, commit, version, allowUnmerged }) {
  return ["workflow", "run", "release.yml", "--ref", branch, "-f", `commit=${commit}`, "-f", `version=${version}`, "-f", `allow-unmerged=${allowUnmerged === "true"}`];
}

function dispatch(argv) {
  const options = parseOptions(argv, ["allow-unmerged"]);
  const branch = git(["symbolic-ref", "--short", "HEAD"]);
  const commit = git(["rev-parse", "HEAD"]);
  const remote = git(["ls-remote", "origin", `refs/heads/${branch}`]).split(/\s+/)[0];
  if (remote !== commit) {
    fail(`origin/${branch} is at ${remote || "nothing"}, not HEAD ${commit}; push the commit before rehearsing it`);
  }
  const args = dispatchArgs({ branch, commit, version: workspaceVersion(readText("Cargo.toml")), allowUnmerged: options.allowUnmerged });
  exec("gh", args, { label: `gh ${args.join(" ")}` });
  console.log(`Started the release rehearsal of ${commit} on ${branch}; follow it with: gh run list --workflow release.yml --branch ${branch} --limit 1`);
}

// ---------------------------------------------------------------------------------------
// check (make release-check)

/** The Maturin the wheel's build backend names must be the one the release jobs lock. */
export function maturinPinProblems(packageToml, developmentToml) {
  const backend = packageToml.match(/"maturin==([0-9.]+)"/)?.[1];
  const locked = developmentToml.match(/^release = \["maturin==([0-9.]+)"\]/m)?.[1];
  return backend && backend === locked ? [] : [`crates/urollup/pyproject.toml builds with maturin ${backend ?? "(unpinned)"}, but the release group locks ${locked ?? "(nothing)"}`];
}

function check() {
  const repo = repositoryState();
  const problems = [...repo.problems, ...maturinPinProblems(readText("crates", "urollup", "pyproject.toml"), readText("pyproject.toml"))];
  try {
    tagForVersion(repo.version);
    pep440Version(repo.version);
    const section = changelogSection(readText("CHANGELOG.md"), repo.version);
    problems.push(...checkReleaseNotes(section, { mode: "rehearsal" }));
  } catch (error) {
    problems.push(error.message);
  }
  if (problems.length > 0) {
    fail(`release invariants do not hold:\n${problems.map((problem) => `- ${problem}`).join("\n")}`);
  }
  console.log(`release-check: version ${repo.version} (tag ${tagForVersion(repo.version)}, PyPI ${pep440Version(repo.version)}), Rust ${repo.toolchain}, ${plannedArtifacts(repo.version).length} artifacts across ${RELEASE_TARGETS.length} hosts`);
}

// ---------------------------------------------------------------------------------------

export const COMMANDS = {
  plan,
  "gate-git": gateGit,
  "assert-no-authority": assertNoAuthority,
  build,
  inspect,
  smoke,
  crates,
  benchmark,
  assemble,
  "stage-github": stageGithub,
  "publish-crates": publishCrates,
  "publish-pypi": publishPypi,
  "publish-github": publishGithub,
  probe,
  evidence,
  "attach-evidence": attachEvidence,
  "rehearse-host": rehearseHost,
  dispatch,
  check,
};

async function main() {
  const [command, ...argv] = process.argv.slice(2);
  const handler = COMMANDS[command];
  if (!handler) {
    fail(`usage: release.mjs <${Object.keys(COMMANDS).join("|")}> [options]`);
  }
  await handler(argv);
}

if (import.meta.url === pathToFileURL(process.argv[1] ?? "").href) {
  main().catch((error) => {
    console.error(`release: ${error.message}`);
    process.exitCode = 1;
  });
}
