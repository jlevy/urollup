// The tag-only release path: the checks that guard publication and the jobs that publish,
// probe and record evidence (docs/project/specs/active/plan-2026-09-16-first-release-publishing.md,
// Build, Validate, and Publish Flow, steps 6–11; runbook in docs/publishing.md).
//
// A rehearsal never runs these jobs, so every function here takes its effects through a
// context object: `ctx.run` spawns processes, `ctx.fetchImpl` talks to GitHub, crates.io
// and PyPI, `ctx.sleep` waits, `ctx.smoke` smoke-tests an installed executable, and
// `ctx.env` holds the job's environment. scripts/release.mjs passes the real ones; the
// tests pass fakes and drive every branch.
//
// Three rules hold throughout:
// - No build or test code runs while a publishing credential is in its environment. The
//   crates.io token is read once, removed from the environment, and handed only to
//   `cargo publish --no-verify`, which compiles nothing.
// - Immediately before every write, the remote tag is peeled again and must still name the
//   release commit.
// - Release files come from named artifacts at fixed paths, and a bundle must agree with
//   itself (manifest, SHA256SUMS, files, crates, plan) before anything is uploaded.

import { createHash } from "node:crypto";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, realpathSync, rmSync, writeFileSync } from "node:fs";
import path from "node:path";

import { parseReleaseVersion } from "./release-identity.mjs";
import { compareChecksums, parseChecksums } from "./release-manifest.mjs";
import {
  USER_AGENT,
  cratesInCommand,
  cratesPackageArgs,
  cratesPublishCommand,
  decideCrates,
  decideGithubRelease,
  decidePypi,
  describeDecision,
  observeCrate,
  observeCrateProject,
  observeGithubRelease,
  observePypi,
  observePypiProject,
  publishGithubRelease,
  stageGithubRelease,
} from "./release-registry.mjs";
import { ephemeralUv, extractArchiveTo, installWheel } from "./release-smoke.mjs";
import { CARGO_PACKAGES, PRODUCT, RELEASE_TARGETS, artifactById, executableName } from "./release-targets.mjs";

/** Where `plan` reads the SSH keys allowed to sign a release tag (an allowed-signers file). */
export const RELEASE_SIGNERS_FILE = ".github/release-allowed-signers";

/** The protected environment every publish job enters. */
export const RELEASE_ENVIRONMENT = "release";

/** The workflow whose runs may attest release artifacts. */
export const RELEASE_WORKFLOW = ".github/workflows/release.yml";

/**
 * Variables that carry a publishing or GitHub credential, or the OIDC token endpoint. The
 * one list serves `withoutCredentials` here, `release.mjs assert-no-authority` and the
 * workflow gate, so a renamed credential cannot drop out of one of them.
 */
export const CREDENTIAL_VARIABLES = [
  "CARGO_REGISTRY_TOKEN",
  "RELEASE_CRATES_TOKEN",
  "RELEASE_WRITE_TOKEN",
  "RELEASE_READ_TOKEN",
  "GITHUB_TOKEN",
  "GH_TOKEN",
  "ACTIONS_ID_TOKEN_REQUEST_URL",
  "ACTIONS_ID_TOKEN_REQUEST_TOKEN",
  "UV_PUBLISH_TOKEN",
  "UV_PUBLISH_PASSWORD",
  "TWINE_PASSWORD",
  "PYPI_API_TOKEN",
  "MATURIN_PYPI_TOKEN",
];

/** True for a credential variable name, including Cargo's per-registry token variables. */
export function isCredentialVariable(name) {
  return CREDENTIAL_VARIABLES.includes(name) || /^CARGO_REGISTRIES_.+_TOKEN$/.test(name);
}

function fail(message) {
  throw new Error(message);
}

function sha256(data) {
  return createHash("sha256").update(data).digest("hex");
}

function readJson(file) {
  return JSON.parse(readFileSync(file, "utf8"));
}

/** A copy of `env` with every credential and the OIDC endpoint removed. */
export function withoutCredentials(env) {
  const clean = { ...env };
  for (const name of Object.keys(clean)) {
    if (isCredentialVariable(name)) {
      delete clean[name];
    }
  }
  return clean;
}

/** Run a process through the context; throw with its output tail on failure. */
export function execIn(ctx, program, args, { env = withoutCredentials(ctx.env), cwd = ctx.root, label, timeout } = {}) {
  const result = ctx.run(program, args, { env, cwd, timeout });
  if (result.error || result.status !== 0) {
    const tail = `${result.stdout ?? ""}${result.stderr ?? ""}`.trim().split("\n").slice(-20).join("\n");
    fail(`${label ?? `${program} ${args.join(" ")}`} failed (${result.error?.message ?? `exit ${result.status ?? result.signal}`})${tail ? `:\n${tail}` : ""}`);
  }
  return result.stdout ?? "";
}

/** Retry `attempt` (which throws to ask for a retry) while a registry catches up. */
export async function retrying(ctx, attempt, { attempts = 10, delayMs = 30_000 } = {}) {
  for (let index = 1; ; index += 1) {
    try {
      return await attempt();
    } catch (error) {
      if (index >= attempts) {
        throw error;
      }
      await ctx.sleep(delayMs);
    }
  }
}

/** Repeat a registry decision until it is final (identical-complete or conflict). */
async function waitFor(ctx, decide, { attempts = 20, delayMs = 15_000 } = {}) {
  let last;
  for (let attempt = 1; attempt <= attempts; attempt += 1) {
    last = await decide();
    if (last.state === "identical-complete" || last.state === "conflict") {
      return last;
    }
    await ctx.sleep(delayMs);
  }
  return last;
}

// ---------------------------------------------------------------------------------------
// The tag: its target, its type and its signature

/** Parse `git ls-remote` output for one tag into its object and its peeled commit. */
export function parseLsRemote(text, tag) {
  let object = null;
  let peeled = null;
  for (const line of text.split(/\r?\n/)) {
    const [sha, ref] = line.trim().split(/\s+/);
    if (ref === `refs/tags/${tag}`) {
      object = sha;
    } else if (ref === `refs/tags/${tag}^{}`) {
      peeled = sha;
    }
  }
  if (!object) {
    return null;
  }
  return { object, commit: peeled ?? object, annotated: peeled !== null };
}

/**
 * Peel the release tag on the remote. `GITHUB_SHA` is not trusted to be the peeled commit
 * of an annotated tag; this asks the remote for `refs/tags/<tag>^{}` instead.
 */
export function peelRemoteTag(ctx, tag) {
  const text = execIn(ctx, "git", ["ls-remote", "origin", `refs/tags/${tag}`, `refs/tags/${tag}^{}`], { label: `git ls-remote origin ${tag}` });
  const parsed = parseLsRemote(text, tag);
  if (!parsed) {
    fail(`the remote has no tag ${tag}`);
  }
  return parsed;
}

/** Require the remote tag to be annotated and to name `commit`, now. */
export function verifyTagTarget(ctx, { tag, commit }) {
  const peeled = peelRemoteTag(ctx, tag);
  if (!peeled.annotated) {
    fail(`${tag} is a lightweight tag; a release tag must be annotated (and signed)`);
  }
  if (peeled.commit !== commit) {
    fail(`${tag} now names ${peeled.commit}, not the release commit ${commit}; refusing to write`);
  }
  return peeled;
}

/**
 * Verify the local tag's SSH signature against the committed allowed-signers file. Git
 * selects the signer by key, so the file needs no e-mail address. Throws when the file is
 * missing, the tag is unsigned, or no allowed key made the signature.
 */
export function verifyTagSignature(ctx, { tag, signersFile }) {
  if (!existsSync(signersFile)) {
    fail(`${path.relative(ctx.root, signersFile)} does not exist; add the maintainer's SSH signing key before tagging (docs/publishing.md)`);
  }
  const result = ctx.run("git", ["-c", `gpg.ssh.allowedSignersFile=${signersFile}`, "verify-tag", tag], { env: withoutCredentials(ctx.env), cwd: ctx.root });
  const output = `${result.stdout ?? ""}${result.stderr ?? ""}`.trim();
  if (result.error || result.status !== 0) {
    fail(`${tag} does not carry a signature from an allowed release signer: ${output || result.error?.message || `exit ${result.status}`}`);
  }
  return output;
}

/** The `tag` header of an annotated tag object: the name it was signed under. */
export function tagObjectName(text) {
  return text.match(/^tag (.+)$/m)?.[1] ?? null;
}

/**
 * Verify that the local tag is exactly the release's signed tag: the object `ls-remote`
 * reported (`object`), naming the release commit (`commit`), signed under the release
 * tag's own name, with a signature from an allowed signer. Without the name check, a
 * signed tag made for something else could be re-pointed as the release ref (review B1).
 * Returns the signature line Git printed.
 */
export function verifyReleaseTag(ctx, { tag, object, commit, signersFile }) {
  const git = (...args) => execIn(ctx, "git", args, { label: `git ${args.join(" ")}` }).trim();
  const local = git("rev-parse", `refs/tags/${tag}`);
  if (local !== object) {
    fail(`the local ${tag} is object ${local}, not the ${object} the remote reported`);
  }
  const target = git("rev-parse", `${tag}^{commit}`);
  if (target !== commit) {
    fail(`${tag} points at ${target}, not the release commit ${commit}`);
  }
  if (git("cat-file", "-t", tag) !== "tag") {
    fail(`${tag} is a lightweight tag; a release tag must be annotated and signed`);
  }
  const name = tagObjectName(git("cat-file", "tag", tag));
  if (name !== tag) {
    fail(`the signed object at ${tag} was made for the tag ${JSON.stringify(name)}, not ${tag}`);
  }
  return verifyTagSignature(ctx, { tag, signersFile });
}

/**
 * The maintainer's pre-push check of a local release tag: the same binding and signature
 * checks `plan` makes on the pushed tag, against the signers file and workspace version
 * at the tagged commit, plus the gate's rule that the commit is on main. Run it before
 * `git push`, because a pushed tag cannot be moved once the ruleset protects it.
 */
export function verifyLocalReleaseTag(ctx, { tag, version }) {
  const git = (...args) => execIn(ctx, "git", args, { label: `git ${args.join(" ")}` });
  const object = git("rev-parse", `refs/tags/${tag}`).trim();
  const commit = git("rev-parse", `${tag}^{commit}`).trim();
  if (`v${version(git("show", `${commit}:Cargo.toml`))}` !== tag) {
    fail(`${tag} names a commit whose workspace version is not ${tag.slice(1)}`);
  }
  const notes = [];
  const main = ctx.run("git", ["rev-parse", "--verify", "--quiet", "refs/remotes/origin/main"], { env: withoutCredentials(ctx.env), cwd: ctx.root });
  if (main.status === 0) {
    if (ctx.run("git", ["merge-base", "--is-ancestor", commit, "refs/remotes/origin/main"], { env: withoutCredentials(ctx.env), cwd: ctx.root }).status !== 0) {
      fail(`${commit} is not on origin/main; fetch main, or tag a merged commit`);
    }
  } else {
    notes.push("origin/main is not fetched here, so the main check was skipped");
  }
  const dir = mkdtempSync(path.join(ctx.tmpdir, "urollup-signers-"));
  try {
    const signersFile = path.join(dir, "allowed-signers");
    const signers = ctx.run("git", ["show", `${commit}:${RELEASE_SIGNERS_FILE}`], { env: withoutCredentials(ctx.env), cwd: ctx.root });
    if (signers.status !== 0) {
      fail(`the tagged commit has no ${RELEASE_SIGNERS_FILE}; commit the maintainer's signing key first`);
    }
    writeFileSync(signersFile, signers.stdout);
    const signature = verifyReleaseTag(ctx, { tag, object, commit, signersFile });
    return { tag, object, commit, signature, notes };
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
}

// ---------------------------------------------------------------------------------------
// Repository settings the approval boundary depends on

async function getJson(ctx, url, token) {
  let response;
  try {
    response = await ctx.fetchImpl(url, {
      headers: { "User-Agent": USER_AGENT, Accept: "application/vnd.github+json", "X-GitHub-Api-Version": "2022-11-28", ...(token ? { Authorization: `Bearer ${token}` } : {}) },
    });
  } catch (error) {
    return { status: "unavailable", reason: `${url}: ${error.message}` };
  }
  // A job token without a permission the endpoint wants is refused with 403 even where the
  // public data is readable; read this public repository's settings anonymously instead.
  if (response.status === 403 && token) {
    return getJson(ctx, url, undefined);
  }
  if (response.status === 404) {
    return { status: "absent" };
  }
  if (!response.ok) {
    return { status: "unavailable", reason: `${url} returned HTTP ${response.status}` };
  }
  return { status: "present", body: await response.json() };
}

/** Read the `release` environment, its deployment policies and the tag rulesets. */
export async function observeReleaseSettings(ctx, { repository, token }) {
  const base = `https://api.github.com/repos/${repository}`;
  const environment = await getJson(ctx, `${base}/environments/${RELEASE_ENVIRONMENT}`, token);
  const policies = environment.status === "present" ? await getJson(ctx, `${base}/environments/${RELEASE_ENVIRONMENT}/deployment-branch-policies`, token) : { status: "absent" };
  const listed = await getJson(ctx, `${base}/rulesets?includes_parents=true&per_page=100`, token);
  const rulesets = [];
  if (listed.status === "present") {
    for (const summary of listed.body.filter((ruleset) => ruleset.target === "tag")) {
      const detail = await getJson(ctx, `${base}/rulesets/${summary.id}`, token);
      rulesets.push(detail.status === "present" ? detail.body : { ...summary, unreadable: detail.reason ?? detail.status });
    }
  }
  return { environment, policies, rulesets: listed.status === "present" ? { status: "present", list: rulesets } : listed };
}

/**
 * The problems with the settings publication relies on: an existing `release`
 * environment with a required reviewer and a deployment policy that admits only `v*`
 * tags, no administrator bypass where GitHub reports it, and an active tag ruleset on
 * `refs/tags/v*` that blocks updates and deletions.
 */
export function judgeReleaseSettings({ environment, policies, rulesets }) {
  const problems = [];
  if (environment.status !== "present") {
    problems.push(environment.status === "absent" ? `the ${RELEASE_ENVIRONMENT} environment does not exist; GitHub would create it unprotected` : `the ${RELEASE_ENVIRONMENT} environment could not be read: ${environment.reason}`);
  } else {
    const body = environment.body;
    const reviewers = (body.protection_rules ?? []).filter((rule) => rule.type === "required_reviewers").flatMap((rule) => rule.reviewers ?? []);
    if (reviewers.length === 0) {
      problems.push(`the ${RELEASE_ENVIRONMENT} environment has no required reviewer`);
    }
    if (body.can_admins_bypass === true) {
      problems.push(`administrators can bypass the ${RELEASE_ENVIRONMENT} environment's protection rules`);
    }
    const policy = body.deployment_branch_policy;
    if (!policy || policy.protected_branches !== false || policy.custom_branch_policies !== true) {
      problems.push(`the ${RELEASE_ENVIRONMENT} environment must deploy only from selected tags`);
    }
    const listed = policies.status === "present" ? (policies.body.branch_policies ?? []) : null;
    if (listed === null) {
      problems.push(`the ${RELEASE_ENVIRONMENT} environment's deployment policies could not be read`);
    } else if (listed.length !== 1 || listed[0].type !== "tag" || listed[0].name !== "v*") {
      problems.push(`the ${RELEASE_ENVIRONMENT} environment's only deployment policy must be the tag pattern v*, found ${listed.map((item) => `${item.type ?? "branch"} ${item.name}`).join(", ") || "none"}`);
    }
  }
  if (rulesets.status !== "present") {
    problems.push(`the repository's rulesets could not be read: ${rulesets.reason ?? rulesets.status}`);
  } else {
    const guarding = guardingRulesets(rulesets.list);
    if (guarding.length === 0) {
      problems.push("no active tag ruleset on refs/tags/v*, with nothing excluded, blocks tag updates and deletions");
    }
    const bypassed = guarding.filter((ruleset) => Array.isArray(ruleset.bypass_actors) && ruleset.bypass_actors.length > 0);
    if (guarding.length > 0 && bypassed.length === guarding.length) {
      problems.push(`the tag ruleset blocking updates and deletions lets ${bypassed[0].bypass_actors.length} actor(s) bypass it; that ruleset needs an empty bypass list`);
    }
  }
  return problems;
}

/** Ref patterns that include every `v*` tag. */
const TAG_PATTERNS = ["refs/tags/v*", "refs/tags/*", "~ALL"];

/**
 * Active tag rulesets that block updates and deletions of every `v*` tag: an include
 * pattern covering `refs/tags/v*` and nothing excluded, since any exclusion could remove
 * a release tag.
 */
export function guardingRulesets(list) {
  return list.filter((ruleset) => {
    const types = new Set((ruleset.rules ?? []).map((rule) => rule.type));
    const include = ruleset.conditions?.ref_name?.include ?? [];
    const exclude = ruleset.conditions?.ref_name?.exclude ?? [];
    return ruleset.target === "tag" && ruleset.enforcement === "active" && include.some((pattern) => TAG_PATTERNS.includes(pattern)) && exclude.length === 0 && types.has("update") && types.has("deletion");
  });
}

/**
 * What the preflight cannot see, so the maintainer confirms it at setup: GitHub returns a
 * ruleset's bypass list (`bypass_actors: null`) only to repository administrators, and the
 * job token is not one.
 */
export function unverifiableSettings({ rulesets }) {
  if (rulesets.status !== "present") {
    return [];
  }
  const hidden = guardingRulesets(rulesets.list).filter((ruleset) => !Array.isArray(ruleset.bypass_actors));
  return hidden.length > 0 ? ["the tag ruleset's bypass list is hidden from the job token; confirm at setup that it is empty"] : [];
}

/** The `gh attestation verify` arguments that bind a file to this release's own build. */
export function attestationVerifyArgs({ file, repository, tag, commit }) {
  return [
    "attestation",
    "verify",
    file,
    "--repo",
    repository,
    "--signer-workflow",
    `${repository}/${RELEASE_WORKFLOW}`,
    "--source-ref",
    `refs/tags/${tag}`,
    "--source-digest",
    commit,
    "--deny-self-hosted-runners",
  ];
}

// ---------------------------------------------------------------------------------------
// The release bundle

/** The GitHub Release assets a bundle stages: archives, wheels, SHA256SUMS and the manifest. */
export function bundleAssets(bundle) {
  const assets = readdirSync(path.join(bundle, "assets"))
    .sort()
    .map((name) => ({ name, file: path.join(bundle, "assets", name) }));
  return [...assets, { name: "SHA256SUMS", file: path.join(bundle, "SHA256SUMS") }, { name: "release-manifest.json", file: path.join(bundle, "release-manifest.json") }].map((asset) => {
    const data = readFileSync(asset.file);
    return { ...asset, data, sha256: sha256(data) };
  });
}

/**
 * Load a release bundle and prove it agrees with itself before anything is promoted:
 * SHA256SUMS matches the asset files, every manifest artifact matches SHA256SUMS in name,
 * digest and size, every Cargo package matches its `.crate`, and the manifest's identity
 * is the plan's.
 */
export function loadBundle(bundle) {
  const manifest = readJson(path.join(bundle, "release-manifest.json"));
  const plan = readJson(path.join(bundle, "release-plan.json"));
  const assets = bundleAssets(bundle);
  const files = assets.filter((asset) => asset.name !== "SHA256SUMS" && asset.name !== "release-manifest.json");
  const sums = parseChecksums(readFileSync(path.join(bundle, "SHA256SUMS"), "utf8"));
  const problems = compareChecksums(sums, files);
  const names = new Set(manifest.artifacts.map((artifact) => artifact.name));
  for (const artifact of manifest.artifacts) {
    const file = files.find((candidate) => candidate.name === artifact.name);
    if (!file) {
      problems.push(`the manifest names ${artifact.name}, which the bundle does not hold`);
    } else if (artifact.sha256 !== file.sha256 || artifact.size !== file.data.length || sums.get(artifact.name) !== artifact.sha256) {
      problems.push(`the manifest's digest or size for ${artifact.name} differs from the file and SHA256SUMS`);
    }
  }
  for (const file of files) {
    if (!names.has(file.name)) {
      problems.push(`${file.name} is in the bundle but not in the manifest`);
    }
  }
  const crates = (manifest.cargo_packages ?? []).map((item) => {
    const file = path.join(bundle, "crates", item.file);
    if (!existsSync(file)) {
      problems.push(`the manifest names ${item.file}, which the bundle does not hold`);
      return { ...item, path: file };
    }
    const data = readFileSync(file);
    if (sha256(data) !== item.sha256 || data.length !== item.size) {
      problems.push(`${item.file} differs from the manifest's digest or size`);
    }
    return { ...item, path: file };
  });
  if (CARGO_PACKAGES.some((crate) => !crates.some((item) => item.name === crate))) {
    problems.push(`the manifest must name every Cargo package: ${CARGO_PACKAGES.join(", ")}`);
  }
  for (const field of ["version", "tag", "commit", "mode"]) {
    if (manifest[field] !== plan[field]) {
      problems.push(`the manifest's ${field} ${manifest[field]} is not the plan's ${plan[field]}`);
    }
  }
  if (problems.length > 0) {
    fail(`the release bundle is inconsistent, so nothing is promoted:\n${problems.join("\n")}`);
  }
  return { manifest, plan, assets, crates };
}

/** Refuse unless this is the tag run for this bundle, at its commit, with the tag still there. */
export function requirePublishRun(ctx, manifest) {
  if (manifest.mode !== "publish" || ctx.env.GITHUB_REF !== `refs/tags/${manifest.tag}`) {
    fail(`publication runs only from the pushed tag ${manifest.tag}, in a publish-mode run`);
  }
  const head = execIn(ctx, "git", ["rev-parse", "HEAD"], { label: "git rev-parse HEAD" }).trim();
  if (head !== manifest.commit) {
    fail(`the checkout is not the release commit ${manifest.commit}`);
  }
  verifyTagTarget(ctx, { tag: manifest.tag, commit: manifest.commit });
}

// ---------------------------------------------------------------------------------------
// Registry observation shared by plan, assemble and the publish jobs

/** Observe every channel for a version; project existence is read for a first release. */
export async function observeAll(ctx, { version, pep440, tag, repository, token, includeDrafts = false, firstRelease = false }) {
  const crates = {};
  const projects = {};
  for (const crate of CARGO_PACKAGES) {
    crates[crate] = await observeCrate(crate, version, ctx.fetchImpl);
    if (firstRelease) {
      projects[crate] = await observeCrateProject(crate, ctx.fetchImpl);
    }
  }
  const pypi = await observePypi(PRODUCT, pep440, ctx.fetchImpl);
  const pypiProject = firstRelease ? await observePypiProject(PRODUCT, ctx.fetchImpl) : undefined;
  const github = repository
    ? await observeGithubRelease(repository, tag, { token, includeDrafts, fetchImpl: ctx.fetchImpl })
    : { status: "unavailable", reason: "no repository is known outside GitHub Actions" };
  return { crates, projects, pypi, pypiProject, github, firstRelease };
}

/** Channel decisions for a bundle against current registry state. */
export async function channelDecisions(ctx, { manifest, assets, firstRelease, repository, token, includeDrafts }) {
  const observed = await observeAll(ctx, { version: manifest.version, pep440: manifest.pep440_version, tag: manifest.tag, repository, token, includeDrafts, firstRelease });
  const wheels = manifest.artifacts.filter((artifact) => artifact.kind === "wheel").map((artifact) => ({ name: artifact.name, sha256: artifact.sha256 }));
  const crates = manifest.cargo_packages.map((item) => ({ name: item.name, sha256: item.sha256 }));
  return {
    observed,
    decisions: [
      decideCrates(crates, observed.crates, { firstRelease, projects: observed.projects }),
      decidePypi(wheels, observed.pypi, { firstRelease, project: observed.pypiProject }),
      decideGithubRelease(
        assets.map((asset) => ({ name: asset.name, sha256: asset.sha256 })),
        observed.github,
      ),
    ],
  };
}

/**
 * Run a publishing command; if it fails, observe the registry again and record exactly
 * what it now holds before failing, so a partial publication is never left unrecorded.
 */
export async function recordingChannelState(ctx, channel, decide, publish) {
  try {
    return await publish();
  } catch (error) {
    let state;
    try {
      state = describeDecision(await decide());
    } catch (observeError) {
      state = `unknown (${observeError.message})`;
    }
    ctx.log([`- ${channel} publishing failed; the registry now reads: ${state}`]);
    throw error;
  }
}

function releaseWriteToken(ctx) {
  const token = ctx.env.RELEASE_WRITE_TOKEN;
  if (!token) {
    fail("RELEASE_WRITE_TOKEN is not set");
  }
  return token;
}

function releaseDetails(bundle, manifest) {
  return {
    tag: manifest.tag,
    commit: manifest.commit,
    title: `${PRODUCT} ${manifest.version}`,
    notes: readFileSync(path.join(bundle, "release-notes.md"), "utf8"),
    prerelease: parseReleaseVersion(manifest.version).prerelease !== null,
  };
}

// ---------------------------------------------------------------------------------------
// Publication (tag runs only, inside the protected release environment)

/** Stage the draft GitHub Release with the bundle's assets. */
export async function stageGithub(ctx, { bundle }) {
  const { manifest, assets } = loadBundle(bundle);
  requirePublishRun(ctx, manifest);
  const result = await stageGithubRelease({
    repository: ctx.env.GITHUB_REPOSITORY,
    ...releaseDetails(bundle, manifest),
    files: assets,
    token: releaseWriteToken(ctx),
    fetchImpl: ctx.fetchImpl,
    sleep: ctx.sleep,
    beforeWrite: async () => verifyTagTarget(ctx, manifest),
  });
  ctx.log([`- GitHub Release draft ${result.releaseId}: ${describeDecision(result.staged)}${result.reconciled.length ? `; reset ${result.reconciled.join(", ")}` : ""}${result.repaired.length ? `; re-uploaded ${result.repaired.join(", ")}` : ""}`]);
  return result;
}

/**
 * Publish both crates, or resume a partial publication, with the rehearsed bytes. The
 * crates.io token is taken out of the environment first; `cargo package` runs without it
 * to prove this commit packages to the rehearsed digests, and only `cargo publish
 * --no-verify`, which compiles nothing, receives it.
 */
export async function publishCrates(ctx, { bundle }) {
  const token = ctx.env.RELEASE_CRATES_TOKEN;
  delete ctx.env.RELEASE_CRATES_TOKEN;
  delete ctx.env.CARGO_REGISTRY_TOKEN;
  if (!token) {
    fail("RELEASE_CRATES_TOKEN is not set; the release environment holds the crates.io token");
  }
  const { manifest, plan } = loadBundle(bundle);
  requirePublishRun(ctx, manifest);
  const planned = manifest.cargo_packages.map((item) => ({ name: item.name, sha256: item.sha256 }));
  const decide = async () => {
    const observed = {};
    const projects = {};
    for (const item of planned) {
      observed[item.name] = await observeCrate(item.name, manifest.version, ctx.fetchImpl);
      if (plan.first_release) {
        projects[item.name] = await observeCrateProject(item.name, ctx.fetchImpl);
      }
    }
    return decideCrates(planned, observed, { firstRelease: Boolean(plan.first_release), projects });
  };
  const before = await decide();
  if (before.blocking) {
    fail(`refusing to publish to crates.io: ${describeDecision(before)}`);
  }
  const command = cratesPublishCommand(before);
  if (command) {
    const clean = withoutCredentials(ctx.env);
    const target = mkdtempSync(path.join(ctx.tmpdir, "urollup-publish-package-"));
    // Package with the same selection the upload uses: --workspace, or -p for a resumed
    // publication whose dependency is already on crates.io.
    execIn(ctx, "cargo", cratesPackageArgs(command), { env: { ...clean, CARGO_TARGET_DIR: target }, label: "cargo package (pre-publish digest check)" });
    const uploading = cratesInCommand(command, manifest.cargo_packages.map((item) => item.name));
    for (const item of manifest.cargo_packages.filter((candidate) => uploading.includes(candidate.name))) {
      const digest = sha256(readFileSync(path.join(target, "package", item.file)));
      if (digest !== item.sha256) {
        fail(`${item.file} packages to ${digest}, not the rehearsed ${item.sha256}; refusing to publish`);
      }
    }
    verifyTagTarget(ctx, manifest);
    await recordingChannelState(ctx, "crates.io", decide, () =>
      execIn(ctx, command[0], command.slice(1), { env: { ...clean, CARGO_TARGET_DIR: target, CARGO_REGISTRY_TOKEN: token }, label: command.join(" ") }),
    );
  }
  const after = await waitFor(ctx, decide);
  if (after.state !== "identical-complete") {
    fail(`crates.io does not show the planned packages after publishing: ${describeDecision(after)}`);
  }
  ctx.log([`- crates.io: ${describeDecision(before)}; now ${after.state}`]);
  return { before, after, command };
}

/** Upload the missing wheels through trusted publishing, or resume a partial upload. */
export async function publishPypi(ctx, { bundle }) {
  const { manifest, plan } = loadBundle(bundle);
  requirePublishRun(ctx, manifest);
  const planned = manifest.artifacts.filter((artifact) => artifact.kind === "wheel").map((artifact) => ({ name: artifact.name, sha256: artifact.sha256 }));
  const decide = async () =>
    decidePypi(planned, await observePypi(PRODUCT, manifest.pep440_version, ctx.fetchImpl), {
      firstRelease: Boolean(plan.first_release),
      project: plan.first_release ? await observePypiProject(PRODUCT, ctx.fetchImpl) : undefined,
    });
  const before = await decide();
  if (before.blocking) {
    fail(`refusing to publish to PyPI: ${describeDecision(before)}`);
  }
  let files = [];
  if (before.publish.length > 0) {
    files = before.publish.map((name) => path.join(bundle, "assets", name));
    verifyTagTarget(ctx, manifest);
    // No configuration file: uv.toml's resolution cool-off would hide the just-uploaded
    // files from --check-url, and publishing must not read user configuration either. The
    // OIDC variables stay in this environment, which runs no build code.
    await recordingChannelState(ctx, "PyPI", decide, () =>
      execIn(ctx, "uv", ["publish", "--trusted-publishing", "always", "--check-url", "https://pypi.org/simple/", ...files], {
        env: { ...ctx.env, UV_NO_CONFIG: "1" },
        label: "uv publish",
      }),
    );
  }
  const after = await waitFor(ctx, decide);
  if (after.state !== "identical-complete") {
    fail(`PyPI does not show the planned wheels after publishing: ${describeDecision(after)}`);
  }
  ctx.log([`- PyPI: ${describeDecision(before)}; now ${after.state}`]);
  return { before, after, files };
}

/** Make the staged draft public once both registries hold exactly the planned files. */
export async function publishGithub(ctx, { bundle }) {
  const { manifest, assets } = loadBundle(bundle);
  requirePublishRun(ctx, manifest);
  const token = releaseWriteToken(ctx);
  const { decisions } = await channelDecisions(ctx, { manifest, assets, firstRelease: false, repository: ctx.env.GITHUB_REPOSITORY, token, includeDrafts: true });
  const registries = decisions.filter((decision) => decision.channel !== "github");
  if (registries.some((decision) => decision.state !== "identical-complete")) {
    fail(`both registries must verify before the release is public:\n${registries.map(describeDecision).join("\n")}`);
  }
  const result = await publishGithubRelease({
    repository: ctx.env.GITHUB_REPOSITORY,
    ...releaseDetails(bundle, manifest),
    files: assets,
    token,
    fetchImpl: ctx.fetchImpl,
    beforeWrite: async () => verifyTagTarget(ctx, manifest),
  });
  ctx.log([`- GitHub Release ${result.releaseId}: ${result.alreadyPublic ? "already public" : "published"}`]);
  return result;
}

// ---------------------------------------------------------------------------------------
// Post-publication probes and evidence

async function download(ctx, url, file) {
  await retrying(
    ctx,
    async () => {
      const response = await ctx.fetchImpl(url, { headers: { "User-Agent": USER_AGENT } });
      if (!response.ok) {
        fail(`${url} returned HTTP ${response.status}`);
      }
      writeFileSync(file, Buffer.from(await response.arrayBuffer()));
    },
    { attempts: 10, delayMs: 15_000 },
  );
}

/** Wait until PyPI's simple index lists every planned wheel, so the probes see them. */
export async function waitForPypiFiles(ctx, { project, files, attempts = 20, delayMs = 15_000 }) {
  return retrying(
    ctx,
    async () => {
      const response = await ctx.fetchImpl(`https://pypi.org/simple/${project}/`, { headers: { "User-Agent": USER_AGENT, Accept: "application/vnd.pypi.simple.v1+json" } });
      if (!response.ok) {
        fail(`the PyPI simple index for ${project} returned HTTP ${response.status}`);
      }
      const listed = new Set(((await response.json()).files ?? []).map((file) => file.filename));
      const missing = files.filter((name) => !listed.has(name));
      if (missing.length > 0) {
        fail(`the PyPI simple index does not list ${missing.join(", ")} yet`);
      }
      return true;
    },
    { attempts, delayMs },
  );
}

/**
 * Install this host's release from GitHub, PyPI and crates.io into clean environments and
 * smoke-test each. Writes `probe-<host>.json` and throws unless every channel passed.
 */
export async function probe(ctx, { host, bundle, out }) {
  const { manifest } = loadBundle(bundle);
  if (ctx.host !== host) {
    fail(`probe ${host} must run on its native host, not ${ctx.host}`);
  }
  const readToken = ctx.env.RELEASE_READ_TOKEN;
  const repository = ctx.env.GITHUB_REPOSITORY;
  const clean = withoutCredentials(ctx.env);
  const work = mkdtempSync(path.join(ctx.tmpdir, "urollup-probe-"));
  const record = { schema: "urollup-release-probe/v1", host, version: manifest.version, channels: {}, problems: [] };
  const archive = manifest.artifacts.find((artifact) => artifact.kind === "archive" && artifact.platform === host);
  const wheel = manifest.artifacts.find((artifact) => artifact.kind === "wheel" && artifact.platform === host);
  const target = artifactById(manifest.version, `archive-${host}`);
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
  try {
    await channel("github", async () => {
      const dir = path.join(work, "github");
      mkdirSync(dir, { recursive: true });
      const base = `https://github.com/${repository}/releases/download/${manifest.tag}`;
      const file = path.join(dir, archive.name);
      await download(ctx, `${base}/${archive.name}`, file);
      await download(ctx, `${base}/SHA256SUMS`, path.join(dir, "SHA256SUMS"));
      const data = readFileSync(file);
      const sums = parseChecksums(readFileSync(path.join(dir, "SHA256SUMS"), "utf8"));
      const problems = compareChecksums(new Map([[archive.name, sums.get(archive.name)]]), [{ name: archive.name, sha256: sha256(data) }]);
      if (sha256(data) !== archive.sha256) {
        problems.push(`the downloaded ${archive.name} is not the manifest's ${archive.sha256}`);
      }
      if (problems.length > 0) {
        return { checks: [], problems };
      }
      // Only gh receives the read token; it is bound to this release's workflow, tag and commit.
      execIn(ctx, "gh", attestationVerifyArgs({ file, repository, tag: manifest.tag, commit: manifest.commit }), {
        env: { ...clean, GH_TOKEN: readToken ?? "" },
        label: `gh attestation verify ${archive.name}`,
      });
      const installDir = path.join(dir, "install");
      const executable = extractArchiveTo(target, data, installDir);
      const smoked = ctx.smoke({ expected: executable, installDirs: [installDir], version: manifest.version, label: "GitHub archive" });
      return { checks: [{ check: "sha256-and-attestation", file: archive.name }, ...smoked.checks], problems: smoked.problems };
    });
    await channel("pypi", async () => {
      const dir = path.join(work, "pypi");
      const wheels = manifest.artifacts.filter((artifact) => artifact.kind === "wheel").map((artifact) => artifact.name);
      await waitForPypiFiles(ctx, { project: PRODUCT, files: wheels });
      let attempt = 0;
      const ephemeral = await retrying(ctx, async () => {
        attempt += 1;
        const result = ephemeralUv({ from: `${PRODUCT}@${manifest.pep440_version}`, dir: path.join(dir, `uvx-${attempt}`), version: manifest.version, index: true, run: ctx.run, baseEnv: clean });
        if (result.problems.length > 0) {
          fail(result.problems.join("; "));
        }
        return result;
      }, { attempts: 6, delayMs: 30_000 });
      const installed = await retrying(ctx, async () => installWheel({ dir: path.join(dir, "install"), index: true, requirement: `${PRODUCT}==${manifest.pep440_version}`, run: ctx.run, baseEnv: clean }), { attempts: 6, delayMs: 30_000 });
      const smoked = ctx.smoke({ expected: installed.installed, installDirs: [installed.binDir], version: manifest.version, label: "uv tool install" });
      const digest = sha256(readFileSync(realpathSync(installed.installed)));
      const problems = [...smoked.problems];
      if (digest !== wheel.executable_sha256) {
        problems.push(`the PyPI-installed urollup is ${digest}, not the validated wheel payload ${wheel.executable_sha256}`);
      }
      return { checks: [...ephemeral.checks, ...smoked.checks], problems };
    });
    await channel("crates", async () => {
      const root = path.join(work, "cargo-root");
      await retrying(
        ctx,
        async () =>
          execIn(ctx, "cargo", ["install", "--locked", `${PRODUCT}@${manifest.version}`, "--root", root], {
            env: { ...clean, CARGO_TARGET_DIR: path.join(work, "cargo-target") },
            label: "cargo install --locked",
          }),
        { attempts: 6, delayMs: 30_000 },
      );
      const binDir = path.join(root, "bin");
      return ctx.smoke({ expected: path.join(binDir, executableName(target.os)), installDirs: [binDir], version: manifest.version, label: "cargo install" });
    });
  } finally {
    rmSync(work, { recursive: true, force: true });
  }
  record.status = record.problems.length === 0 ? "passed" : "failed";
  mkdirSync(out, { recursive: true });
  writeFileSync(path.join(out, `probe-${host}.json`), `${JSON.stringify(record, null, 2)}\n`);
  ctx.log([`- ${host}: ${record.status}`, ...record.problems.map((problem) => `  - ${problem}`)]);
  if (record.status !== "passed") {
    fail(`installation probes failed on ${host}`);
  }
  return record;
}

/**
 * Write release-evidence.json only when every host's probe passed and every registry
 * holds exactly the planned files. `probes` holds one `probe-<host>/probe-<host>.json`
 * per release host, as the probe jobs upload them.
 */
export async function evidence(ctx, { bundle, probes, out }) {
  const { manifest, assets } = loadBundle(bundle);
  const records = RELEASE_TARGETS.map((target) => {
    const file = path.join(probes, `probe-${target.id}`, `probe-${target.id}.json`);
    if (!existsSync(file)) {
      return { host: target.id, status: "missing" };
    }
    const record = readJson(file);
    return record.host === target.id ? record : { host: target.id, status: `mislabeled (${record.host})` };
  });
  const { decisions } = await channelDecisions(ctx, {
    manifest,
    assets,
    firstRelease: false,
    repository: ctx.env.GITHUB_REPOSITORY,
    token: ctx.env.RELEASE_READ_TOKEN,
    includeDrafts: false,
  });
  const incomplete = [
    ...records.filter((record) => record.status !== "passed").map((record) => `probe ${record.host} is ${record.status}`),
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
    workflow_run: ctx.workflowRun ?? null,
    channels: Object.fromEntries(decisions.map((decision) => [decision.channel, { state: decision.state, verified: decision.skip }])),
    probes: records,
    complete: true,
  };
  mkdirSync(out, { recursive: true });
  writeFileSync(path.join(out, "release-evidence.json"), `${JSON.stringify(record, null, 2)}\n`);
  ctx.log([`- Release evidence written for ${manifest.tag}: every channel and probe passed`]);
  return record;
}

/**
 * Attach the evidence the `attest-evidence` job attested to the public release. The file
 * must describe this bundle's version, tag, commit and manifest, and is never replaced.
 */
export async function attachEvidence(ctx, { bundle, evidenceDir }) {
  const { manifest, assets } = loadBundle(bundle);
  requirePublishRun(ctx, manifest);
  const file = path.join(evidenceDir, "release-evidence.json");
  const data = readFileSync(file);
  const record = JSON.parse(data.toString("utf8"));
  const manifestDigest = assets.find((asset) => asset.name === "release-manifest.json").sha256;
  if (record.complete !== true || record.version !== manifest.version || record.tag !== manifest.tag || record.commit !== manifest.commit || record.manifest_sha256 !== manifestDigest) {
    fail("release-evidence.json does not describe this release's bundle");
  }
  const token = releaseWriteToken(ctx);
  const repository = ctx.env.GITHUB_REPOSITORY;
  const observed = await observeGithubRelease(repository, manifest.tag, { token, includeDrafts: true, fetchImpl: ctx.fetchImpl });
  if (observed.status !== "present" || observed.draft) {
    fail(`the ${manifest.tag} release must be public before its evidence is attached`);
  }
  const existing = observed.assets.find((asset) => asset.name === "release-evidence.json");
  if (existing) {
    if (existing.sha256 !== sha256(data)) {
      fail("a different release-evidence.json is already attached; evidence is never replaced");
    }
    ctx.log(["- release-evidence.json is already attached and identical"]);
    return { attached: false };
  }
  verifyTagTarget(ctx, manifest);
  const response = await ctx.fetchImpl(`https://uploads.github.com/repos/${repository}/releases/${observed.id}/assets?name=release-evidence.json`, {
    method: "POST",
    headers: { "User-Agent": USER_AGENT, Authorization: `Bearer ${token}`, "Content-Type": "application/json", Accept: "application/vnd.github+json" },
    body: data,
  });
  if (!response.ok) {
    fail(`attaching release-evidence.json returned HTTP ${response.status}`);
  }
  ctx.log(["- Attached release-evidence.json"]);
  return { attached: true };
}
