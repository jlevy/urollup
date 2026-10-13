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
//   assert-no-authority  Fail if this job's environment holds a credential or OIDC endpoint
//   build              Build and package one archive or wheel
//   inspect            Check one packaged artifact's contents
//   smoke              Install one artifact into an empty environment and test it
//   crates             The crates.io packaging rehearsal
//   benchmark          Time the static musl and manylinux executables (informational)
//   assemble           Check the complete matrix; write SHA256SUMS, manifest, notes, actions
//   stage-github, publish-crates, publish-pypi, publish-github, probe, evidence,
//   attach-evidence    The tag-only publication path
//   verify-tag         The maintainer's check of a signed local tag before pushing it
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

import { writeTarGz, writeZip } from "./release-archive.mjs";
import { rehearseCrates } from "./release-crates.mjs";
import {
  checkCargoManifests,
  checkPyproject,
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
import { describeDecision, LATE_ASSETS } from "./release-registry.mjs";
import {
  CREDENTIAL_VARIABLES,
  RELEASE_SIGNERS_FILE,
  attachEvidence,
  bundleAssets,
  channelDecisions,
  evidence,
  execIn,
  isCredentialVariable,
  judgeReleaseSettings,
  loadBundle,
  observeAll,
  observeReleaseSettings,
  peelRemoteTag,
  probe,
  publishCrates,
  publishGithub,
  publishPypi,
  stageGithub,
  unverifiableSettings,
  verifyLocalReleaseTag,
  verifyReleaseTag,
} from "./release-publish.mjs";
import { ephemeralUv, extractArchiveTo, installWheel, smokeInstalled, spawnRun } from "./release-smoke.mjs";
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
/** Credentials and the OIDC endpoint, from the one shared list in release-publish.mjs. */
export const AUTHORITY_VARIABLES = CREDENTIAL_VARIABLES;

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
// Context for the tag-only path (scripts/release-publish.mjs takes its effects from it)

function readToken() {
  return process.env.RELEASE_READ_TOKEN || process.env.GITHUB_TOKEN || undefined;
}

/** The real effects: processes, network, time, smoke tests and the run summary. */
export function defaultContext() {
  return {
    env: process.env,
    run: spawnRun,
    fetchImpl: fetch,
    sleep: (milliseconds) => new Promise((resolve) => setTimeout(resolve, milliseconds)),
    smoke: smokeInstalled,
    log: summary,
    root: ROOT,
    tmpdir: process.env.RUNNER_TEMP ?? os.tmpdir(),
    host: hostId(),
    workflowRun: workflowRun(),
  };
}

function observationSummary(observed) {
  const status = (value) => (value === undefined ? "not read" : value.status + (value.status === "unavailable" ? ` (${value.reason})` : ""));
  return [
    ...CARGO_PACKAGES.map((crate) => `- crates.io ${crate}: ${status(observed.crates[crate])}${observed.firstRelease ? `; crate ${status(observed.projects[crate])}` : ""}`),
    `- PyPI ${PRODUCT}: ${status(observed.pypi)}${observed.firstRelease ? `; project ${status(observed.pypiProject)}` : ""}`,
    `- GitHub Release: ${status(observed.github)}`,
  ];
}

// ---------------------------------------------------------------------------------------
// plan

/**
 * The commit a tag run releases. `GITHUB_SHA` is not trusted to be the peeled commit of an
 * annotated tag: the remote tag is peeled explicitly, must be annotated, and must be the
 * commit this job checked out.
 */
export function resolveTagCommit(ctx, tag) {
  const peeled = peelRemoteTag(ctx, tag);
  if (!peeled.annotated) {
    fail(`${tag} is a lightweight tag; create the release tag with git tag -s (annotated and signed)`);
  }
  const head = execIn(ctx, "git", ["rev-parse", "HEAD"], { label: "git rev-parse HEAD" }).trim();
  if (head !== peeled.commit) {
    fail(`the checkout is at ${head}, but ${tag} names ${peeled.commit}`);
  }
  return peeled;
}

/**
 * The guards a tag publication must pass before any job builds for it: the tag carries a
 * signature from an allowed release signer, and the repository's `release` environment
 * and tag ruleset exist as the runbook configures them. A rehearsal reports the same
 * settings without refusing.
 */
export async function publicationGuards(ctx, { tag, peeled, repository, token, publish }) {
  const problems = [];
  const facts = {};
  if (publish) {
    execIn(ctx, "git", ["fetch", "--no-tags", "--force", "origin", `refs/tags/${tag}:refs/tags/${tag}`], { label: `git fetch ${tag}` });
    try {
      facts.signature = verifyReleaseTag(ctx, { tag, object: peeled.object, commit: peeled.commit, signersFile: path.join(ctx.root, RELEASE_SIGNERS_FILE) });
    } catch (error) {
      problems.push(error.message);
    }
  } else {
    facts.signers_file = existsSync(path.join(ctx.root, RELEASE_SIGNERS_FILE)) ? "present" : "missing";
  }
  if (repository) {
    const settings = await observeReleaseSettings(ctx, { repository, token });
    facts.settings_problems = judgeReleaseSettings(settings);
    facts.settings_unverifiable = unverifiableSettings(settings);
    problems.push(...(publish ? facts.settings_problems : []));
  } else {
    facts.settings_problems = ["not read outside GitHub Actions"];
  }
  return { problems, facts };
}

async function plan(argv) {
  const options = parseOptions(argv, ["out", "event", "ref", "commit", "version", "offline"]);
  const out = required(options, "out");
  const ctx = defaultContext();
  const repo = repositoryState();
  if (repo.problems.length > 0) {
    fail(`the repository is not releasable:\n${repo.problems.map((problem) => `- ${problem}`).join("\n")}`);
  }
  const event = process.env.GITHUB_EVENT_NAME ?? options.event ?? "workflow_dispatch";
  const ref = process.env.GITHUB_REF ?? options.ref ?? git(["symbolic-ref", "-q", "HEAD"]);
  const peeled = event === "push" && ref.startsWith("refs/tags/") ? resolveTagCommit(ctx, ref.slice("refs/tags/".length)) : null;
  const sha = peeled ? peeled.commit : (process.env.GITHUB_SHA ?? git(["rev-parse", "HEAD"]));
  const identity = resolveRunIdentity({
    event,
    ref,
    sha,
    inputs: {
      commit: process.env.RELEASE_INPUT_COMMIT ?? options.commit,
      version: process.env.RELEASE_INPUT_VERSION ?? options.version,
    },
    workspaceToml: repo.workspaceToml,
  });
  const publish = identity.mode === "publish";
  const allowUnmerged = identity.mode === "rehearsal" && process.env.RELEASE_INPUT_ALLOW_UNMERGED === "true";
  const sourceDateEpoch = commitTime(identity.commit);
  const firstRelease = changelogSection(readText("CHANGELOG.md"), identity.version).first;
  const repository = process.env.GITHUB_REPOSITORY;
  const guards = options.offline === "true" ? { problems: [], facts: { offline: true } } : await publicationGuards(ctx, { tag: identity.tag, peeled, repository, token: readToken(), publish });
  if (guards.problems.length > 0) {
    fail(`${identity.tag} cannot be published:\n${guards.problems.map((problem) => `- ${problem}`).join("\n")}`);
  }
  const observed = options.offline === "true" ? null : await observeAll(ctx, { ...identity, repository, token: readToken(), firstRelease });
  const document = {
    schema: "urollup-release-plan/v1",
    ...identity,
    publish,
    allow_unmerged: allowUnmerged,
    first_release: firstRelease,
    source_date_epoch: sourceDateEpoch,
    rust_toolchain: repo.toolchain,
    node: process.version,
    macos_deployment_target: MACOS_DEPLOYMENT_TARGET,
    workflow_run: workflowRun(),
    artifacts: plannedArtifacts(identity.version),
    crates: plannedCrates(identity.version),
    publication_guards: guards.facts,
    prior_publication: observed,
  };
  writeJson(path.join(out, "release-plan.json"), document);
  setOutputs({
    version: identity.version,
    tag: identity.tag,
    commit: identity.commit,
    mode: identity.mode,
    publish: String(publish),
    "allow-unmerged": String(allowUnmerged),
    "source-date-epoch": String(sourceDateEpoch),
    matrix: buildMatrix(identity.version),
    hosts: { include: RELEASE_TARGETS.map((target) => ({ host: target.id, runner: target.runner })) },
  });
  const settings = guards.facts.settings_problems ?? [];
  summary([
    `## Release plan: ${identity.tag} (${identity.mode})`,
    "",
    `- Commit: \`${identity.commit}\``,
    `- Version: ${identity.version} (PyPI ${identity.pep440})${firstRelease ? "; first release" : ""}`,
    `- Rust toolchain: ${repo.toolchain}; Node ${process.version}; SOURCE_DATE_EPOCH ${sourceDateEpoch}`,
    `- Publication: ${publish ? "enabled after validation and approval" : "none; this is a rehearsal"}`,
    ...(allowUnmerged ? ["- The commit may be off main; this rehearsal is not releasable"] : []),
    `- Artifacts: ${document.artifacts.length} (${document.artifacts.map((artifact) => artifact.name).join(", ")})`,
    "",
    "### Publication settings",
    "",
    ...(guards.facts.offline ? ["- not read (--offline)"] : settings.length === 0 ? ["- The release environment and the v* tag ruleset are configured as the runbook requires"] : settings.map((problem) => `- ${publish ? "" : "A tag publication would stop: "}${problem}`)),
    ...(guards.facts.settings_unverifiable ?? []).map((note) => `- Not verifiable by this job: ${note}`),
    ...(guards.facts.signers_file ? [`- Release tag signers file (${RELEASE_SIGNERS_FILE}): ${guards.facts.signers_file}`] : []),
    ...(guards.facts.signature ? [`- Tag signature: ${guards.facts.signature.split("\n")[0]}`] : []),
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
  return Object.keys(env).filter((name) => isCredentialVariable(name) && env[name]);
}

/**
 * What this can observe is the environment: no OIDC token endpoint and no exported
 * credential. The job token's scope is not in the environment; scripts/check-release-workflow.mjs
 * proves it statically, and each job log's GITHUB_TOKEN Permissions banner shows it.
 */
function assertNoAuthority() {
  const present = authorityProblems(process.env);
  if (present.length > 0) {
    fail(`this job's environment must hold no credential or OIDC endpoint, but it has ${present.join(", ")}`);
  }
  console.log("This job's environment holds no OIDC token endpoint and no GitHub, crates.io or PyPI credential.");
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
export function manylinuxCommand({ artifact, root, cargoHomeDir, sysroot, maturin, uid, gid, outDir, env, sourceDateEpoch }) {
  if (!Number.isInteger(sourceDateEpoch) || sourceDateEpoch <= 0) {
    fail(`SOURCE_DATE_EPOCH must be whole seconds, not ${JSON.stringify(sourceDateEpoch)}`);
  }
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
    // Maturin stamps wheel members with this time, as it does on macOS and Windows.
    "--env",
    `SOURCE_DATE_EPOCH=${sourceDateEpoch}`,
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

function buildWheel(artifact, env, workDir, sourceDateEpoch) {
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
    exec("docker", manylinuxCommand({ artifact, root: ROOT, cargoHomeDir: cargoHome(), sysroot, maturin, uid: process.getuid(), gid: process.getgid(), outDir, env, sourceDateEpoch }), {
      label: `maturin build in ${artifact.container}`,
    });
  } else {
    exec(
      "uv",
      ["--config-file", "uv.toml", "run", "--frozen", "--only-group", "release", "maturin", "build", "--release", "--locked", "--manifest-path", "crates/urollup/Cargo.toml", "--target", artifact.rustTarget, "--compatibility", "pypi", "--out", outDir],
      { env: { ...process.env, ...env, SOURCE_DATE_EPOCH: String(sourceDateEpoch), TZ: "UTC" }, label: "maturin build" },
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

/** The commit time every archive and wheel member must carry, when the job knows it. */
function plannedSourceDateEpoch(value = process.env.SOURCE_DATE_EPOCH) {
  if (value === undefined || value === "") {
    return undefined;
  }
  const seconds = Number(value);
  if (!Number.isInteger(seconds) || seconds <= 0) {
    fail(`SOURCE_DATE_EPOCH ${JSON.stringify(value)} is not whole seconds`);
  }
  return seconds;
}

function inspectArtifact(artifact, data, version, sourceDateEpoch) {
  return artifact.kind === "archive"
    ? checkArchive(artifact.name, data, artifact, { readme: readFileSync(path.join(ROOT, "README.md")), license: readFileSync(path.join(ROOT, "LICENSE")), sourceDateEpoch })
    : checkWheel(artifact.name, data, artifact, { version, sourceDateEpoch });
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
  const data = artifact.kind === "archive" ? buildArchive(artifact, env, sourceDateEpoch) : buildWheel(artifact, env, workDir, sourceDateEpoch);
  const inspected = inspectArtifact(artifact, data, version, sourceDateEpoch);
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
    // Archive bytes come from Node's bundled zlib, so a reproduction names this version too.
    node: process.version,
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
  const options = parseOptions(argv, ["artifact", "file", "source-date-epoch"]);
  const version = workspaceVersion(readText("Cargo.toml"));
  const artifact = artifactById(version, required(options, "artifact"));
  const result = inspectArtifact(artifact, readFileSync(required(options, "file")), version, plannedSourceDateEpoch(options.sourceDateEpoch));
  console.log(JSON.stringify(result.facts, null, 2));
  if (result.problems.length > 0) {
    fail(result.problems.join("\n"));
  }
}

// ---------------------------------------------------------------------------------------
// smoke

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
  const inspected = inspectArtifact(artifact, data, version, plannedSourceDateEpoch());
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

/** Read a JSON file a named artifact must contain, or record that it is missing. */
function namedJson(file, problems, what) {
  if (!existsSync(file)) {
    problems.push(`${what} is missing (${file})`);
    return null;
  }
  return readJson(file);
}

/**
 * Assemble the release from named artifacts at fixed paths: `--plan` holds
 * release-plan.json, `--artifacts` one directory per planned artifact id, `--validations`
 * one `validation-<id>` directory per artifact, and `--crates` the crates job's output.
 * Nothing is found by searching, so no other artifact of the run can stand in.
 */
async function assemble(argv) {
  const options = parseOptions(argv, ["plan", "artifacts", "validations", "crates", "out"]);
  const out = required(options, "out");
  const planFile = path.join(required(options, "plan"), "release-plan.json");
  const artifactsDir = required(options, "artifacts");
  const validationsDir = required(options, "validations");
  const cratesDir = required(options, "crates");
  if (!existsSync(planFile)) {
    fail(`no release-plan.json at ${planFile}`);
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
    const file = path.join(artifactsDir, artifact.id, artifact.name);
    if (existsSync(file)) {
      const data = readFileSync(file);
      artifacts.push({ name: artifact.name, file, sha256: sha256(data), size: data.length });
    }
    const build = namedJson(path.join(artifactsDir, artifact.id, `build-${artifact.id}.json`), problems, `the build record of ${artifact.name}`);
    if (build) {
      builds.push(build);
      if (build.commit !== identity.commit) {
        problems.push(`${artifact.name} was built from ${build.commit}, not ${identity.commit}`);
      }
      const assembled = artifacts.find((candidate) => candidate.name === artifact.name);
      if (assembled && build.sha256 !== assembled.sha256) {
        problems.push(`${artifact.name} differs from the bytes its build job recorded`);
      }
    }
    const validation = namedJson(path.join(validationsDir, `validation-${artifact.id}`, `validation-${artifact.id}.json`), problems, `the validation record of ${artifact.name}`);
    if (validation) {
      validations.push(validation);
    }
  }
  const crateFiles = [];
  const record = namedJson(path.join(cratesDir, "crates.json"), problems, "the crates.io rehearsal record");
  if (record) {
    if (record.status !== "passed" || record.commit !== identity.commit) {
      problems.push(`the crates.io rehearsal record is ${record.status} for ${record.commit}`);
    }
    for (const item of plannedCrates(identity.version)) {
      const file = path.join(cratesDir, item.name);
      if (existsSync(file)) {
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

  // The publish jobs' own loader reads the real bundle here, so a tag run can never be the
  // first time it sees one (review B8).
  loadBundle(out);
  const assets = bundleAssets(out);
  const { observed, decisions } = await channelDecisions(defaultContext(), {
    manifest,
    assets,
    firstRelease: Boolean(planned.first_release),
    repository: process.env.GITHUB_REPOSITORY,
    token: readToken(),
    includeDrafts: false,
  });
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
// Publication, probes and evidence (tag runs only; logic in scripts/release-publish.mjs)

async function stageGithubCommand(argv) {
  await stageGithub(defaultContext(), { bundle: required(parseOptions(argv, ["bundle"]), "bundle") });
}

async function publishCratesCommand(argv) {
  await publishCrates(defaultContext(), { bundle: required(parseOptions(argv, ["bundle"]), "bundle") });
}

async function publishPypiCommand(argv) {
  await publishPypi(defaultContext(), { bundle: required(parseOptions(argv, ["bundle"]), "bundle") });
}

async function publishGithubCommand(argv) {
  await publishGithub(defaultContext(), { bundle: required(parseOptions(argv, ["bundle"]), "bundle") });
}

async function probeCommand(argv) {
  const options = parseOptions(argv, ["host", "bundle", "out"]);
  await probe(defaultContext(), { host: required(options, "host"), bundle: required(options, "bundle"), out: required(options, "out") });
}

async function evidenceCommand(argv) {
  const options = parseOptions(argv, ["bundle", "probes", "out"]);
  await evidence(defaultContext(), { bundle: required(options, "bundle"), probes: required(options, "probes"), out: required(options, "out") });
}

async function attachEvidenceCommand(argv) {
  const options = parseOptions(argv, ["bundle", "evidence"]);
  await attachEvidence(defaultContext(), { bundle: required(options, "bundle"), evidenceDir: required(options, "evidence") });
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
  process.env.SOURCE_DATE_EPOCH ??= String(commitTime(git(["rev-parse", "HEAD"])));
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
// verify-tag (make release-verify-tag): the maintainer's check before pushing a tag

function verifyTag(argv) {
  const options = parseOptions(argv, ["tag", "repo"]);
  const ctx = { ...defaultContext(), root: options.repo ? path.resolve(options.repo) : ROOT };
  const result = verifyLocalReleaseTag(ctx, { tag: required(options, "tag"), version: workspaceVersion });
  console.log(`${result.tag} is an annotated tag of ${result.commit}, made for ${result.tag}, and ${result.signature.split("\n")[0]}`);
  for (const note of result.notes) {
    console.log(`note: ${note}`);
  }
  console.log(`Push only this tag: git push origin ${result.tag}`);
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
  "stage-github": stageGithubCommand,
  "publish-crates": publishCratesCommand,
  "publish-pypi": publishPypiCommand,
  "publish-github": publishGithubCommand,
  probe: probeCommand,
  evidence: evidenceCommand,
  "attach-evidence": attachEvidenceCommand,
  "rehearse-host": rehearseHost,
  "verify-tag": verifyTag,
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
