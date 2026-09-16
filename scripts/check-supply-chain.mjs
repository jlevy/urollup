#!/usr/bin/env node
// Supply-chain provenance gate: release age, registry provenance, pins and CI trust
// controls for every dependency surface in this repository.
//
// Adapted from fdu `scripts/check-supply-chain.mjs` at afbb2ee; see PROVENANCE.md.
// urollup changes: all workspace npm lockfiles, one root uv.lock, workflow-level and
// job-level permission checks for every workflow, and a policy that fails on any expired
// exception even when unused.

import { readFile, readdir } from "node:fs/promises";
import { createHash } from "node:crypto";
import { execFileSync } from "node:child_process";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const DAY_MS = 24 * 60 * 60 * 1000;
const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

//: Every npm lockfile in the repository, relative to ROOT.
//
// The parity project is deliberately isolated from the root development tools, but its
// native executable packages need the same integrity, provenance and cool-off checks.
const NPM_LOCKS = [
  ["package-lock.json"],
  ["tests", "parity", "ccusage", "package-lock.json"],
];

//: Every uv lockfile in the repository, relative to ROOT.
//
// Auditing one of them is worse than auditing none, because the summary line then
// reports a clean Python surface while a whole environment goes unchecked. Add any new
// uv project here in the same change that adds its lockfile.
const UV_LOCKS = [["uv.lock"]];

function fail(message) {
  throw new Error(message);
}

function requiredString(value, label) {
  if (typeof value !== "string" || value.trim() === "") {
    fail(`${label} must be a non-empty string`);
  }
  return value;
}

function validDate(value, label) {
  const date = new Date(requiredString(value, label));
  if (Number.isNaN(date.getTime())) {
    fail(`${label} is not a valid timestamp`);
  }
  return date;
}

export function validateExceptions(exceptions) {
  if (!Array.isArray(exceptions)) {
    fail("exceptions must be an array");
  }
  const ids = new Set();
  for (const exception of exceptions) {
    for (const field of [
      "id",
      "ecosystem",
      "name",
      "version",
      "reason",
      "approvedBy",
      "approvedAt",
      "expiresAt",
      "followUp",
    ]) {
      requiredString(exception?.[field], `exception ${exception?.id ?? "<unknown>"}.${field}`);
    }
    if (ids.has(exception.id)) {
      fail(`duplicate exception id ${exception.id}`);
    }
    ids.add(exception.id);
    const approvedAt = validDate(exception.approvedAt, `exception ${exception.id}.approvedAt`);
    const expiresAt = validDate(exception.expiresAt, `exception ${exception.id}.expiresAt`);
    if (expiresAt <= approvedAt) {
      fail(`exception ${exception.id} must expire after approval`);
    }
  }
}

/**
 * Fail on any expired exception, whether or not a locked package still uses it.
 *
 * An expired waiver that nothing matches is harmless today, but it is the record rotting:
 * the next dependency change that happens to match it would be told the waiver expired,
 * rather than the maintainer being told when it did. Removing or renewing it is the
 * follow-up the exception already names.
 */
export function assertNoExpiredExceptions(exceptions, now) {
  validateExceptions(exceptions);
  for (const exception of exceptions) {
    if (validDate(exception.expiresAt, `exception ${exception.id}.expiresAt`) <= now) {
      fail(
        `exception ${exception.id} expired at ${exception.expiresAt}; ` +
          `remove or renew it (follow-up: ${exception.followUp})`,
      );
    }
  }
}

export function validateFirstParty(firstParty) {
  if (!Array.isArray(firstParty)) {
    fail("firstParty must be an array");
  }
  for (const entry of firstParty) {
    for (const field of ["ecosystem", "name", "reason", "approvedBy"]) {
      requiredString(entry?.[field], `firstParty ${entry?.name ?? "<unknown>"}.${field}`);
    }
    // Deliberately no version field: pinning one here would recreate the per-release
    // churn this list exists to remove.
    if ("version" in entry) {
      fail(`firstParty ${entry.name} must not pin a version`);
    }
  }
}

export function assertAged(item, now, minimumAgeDays, exceptions, firstParty = []) {
  const publishedAt = new Date(item.publishedAt ?? "");
  if (Number.isNaN(publishedAt.getTime())) {
    fail(`${item.ecosystem} ${item.name}@${item.version} is missing a valid publication time`);
  }
  const cutoff = now.getTime() - minimumAgeDays * DAY_MS;
  if (publishedAt.getTime() <= cutoff) {
    return;
  }

  // A first-party package is exempt by identity rather than by version. The cool-off
  // exists so that a compromised upstream release is noticed by somebody else before
  // this repository takes it, and that argument does not apply to a package this
  // project's own authors publish — so requiring a fresh dated exception for every
  // patch release bought no safety and guaranteed the record would rot. This waives
  // release age only; integrity, tarball, and publication-time checks still run, which
  // is what SUPPLY-CHAIN-SECURITY.md means by never waiving provenance.
  validateFirstParty(firstParty);
  if (
    firstParty.some(
      (candidate) => candidate.ecosystem === item.ecosystem && candidate.name === item.name,
    )
  ) {
    return;
  }

  validateExceptions(exceptions);
  const exception = exceptions.find(
    (candidate) =>
      candidate.ecosystem === item.ecosystem &&
      candidate.name === item.name &&
      candidate.version === item.version,
  );
  if (exception && validDate(exception.expiresAt, `exception ${exception.id}.expiresAt`) > now) {
    return;
  }
  fail(
    `${item.ecosystem} ${item.name}@${item.version} has not cleared the ${minimumAgeDays}-day cool-off` +
      (exception ? `; exception ${exception.id} is expired` : ""),
  );
}

export function assertEqualProvenance(label, expected, actual) {
  requiredString(expected, `${label} expected value`);
  requiredString(actual, `${label} authoritative value`);
  if (expected !== actual) {
    fail(`${label} mismatch: locked ${expected}, authoritative ${actual}`);
  }
}

function assertEqualTimestamp(label, expected, actual) {
  const expectedDate = validDate(expected, `${label} locked value`);
  const actualDate = validDate(actual, `${label} authoritative value`);
  if (expectedDate.getTime() !== actualDate.getTime()) {
    fail(`${label} mismatch: locked ${expected}, authoritative ${actual}`);
  }
}

function packageBlocks(text) {
  return [...text.matchAll(/\[\[package\]\]\s*\n([\s\S]*?)(?=\n\[\[package\]\]|\s*$)/g)].map(
    (match) => match[1],
  );
}

function quotedField(block, name) {
  return block.match(new RegExp(`^${name} = "([^"]+)"`, "m"))?.[1];
}

export function parseCargoLock(text) {
  if (!/^version = 4$/m.test(text)) {
    fail("Cargo.lock must use the reviewed version 4 format");
  }
  const packages = [];
  for (const block of packageBlocks(text)) {
    if (!/^source = "registry\+/m.test(block)) {
      continue;
    }
    const name = requiredString(quotedField(block, "name"), "Cargo package name");
    const version = requiredString(quotedField(block, "version"), `Cargo package ${name} version`);
    const checksum = quotedField(block, "checksum");
    if (!checksum) {
      fail(`Cargo package ${name}@${version} is missing a checksum`);
    }
    if (!/^[0-9a-f]{64}$/.test(checksum)) {
      fail(`Cargo package ${name}@${version} has an invalid checksum`);
    }
    packages.push({ name, version, checksum });
  }
  if (packages.length === 0) {
    fail("Cargo.lock contains no registry packages");
  }
  return packages;
}

export function parseNpmLock(text) {
  let lock;
  try {
    lock = JSON.parse(text);
  } catch (error) {
    fail(`package-lock.json is not valid JSON: ${error.message}`);
  }
  if (lock.lockfileVersion !== 3 || typeof lock.packages !== "object" || lock.packages === null) {
    fail("package-lock.json must use lockfileVersion 3 with a packages inventory");
  }
  const packages = [];
  for (const [packagePath, record] of Object.entries(lock.packages)) {
    if (!packagePath || !record.resolved?.startsWith("https://registry.npmjs.org/")) {
      continue;
    }
    const name = packagePath.split("node_modules/").at(-1);
    const version = requiredString(record.version, `npm package ${name} version`);
    const integrity = requiredString(record.integrity, `npm package ${name}@${version} integrity`);
    if (!/^sha512-[A-Za-z0-9+/]+={0,2}$/.test(integrity)) {
      fail(`npm package ${name}@${version} has an invalid sha512 integrity`);
    }
    packages.push({ name, version, integrity, resolved: record.resolved });
  }
  if (packages.length === 0) {
    fail("package-lock.json contains no registry packages");
  }
  return packages;
}

export function parseUvLock(text) {
  if (!/^version = 1$/m.test(text) || !/^revision = 3$/m.test(text)) {
    fail("uv.lock must use the reviewed version 1 revision 3 format");
  }
  const packages = [];
  for (const block of packageBlocks(text)) {
    if (!/^source = \{ registry = "https:\/\/pypi\.org\/simple" \}$/m.test(block)) {
      continue;
    }
    const name = requiredString(quotedField(block, "name"), "Python package name");
    const version = requiredString(quotedField(block, "version"), `Python package ${name} version`);
    const artifacts = [];
    const artifactPattern =
      /\{\s*url = "([^"]+)"\s*,\s*hash = "([^"]+)"(?:\s*,\s*size = \d+)?(?:\s*,\s*upload-time = "([^"]+)")?\s*\}/g;
    for (const match of block.matchAll(artifactPattern)) {
      const [, url, hash, uploadTime] = match;
      if (!uploadTime) {
        fail(`Python package ${name}@${version} artifact is missing an upload time`);
      }
      if (!/^sha256:[0-9a-f]{64}$/.test(hash)) {
        fail(`Python package ${name}@${version} artifact has an invalid sha256 hash`);
      }
      if (!url.startsWith("https://files.pythonhosted.org/")) {
        fail(`Python package ${name}@${version} artifact is not hosted by PyPI`);
      }
      validDate(uploadTime, `Python package ${name}@${version} upload time`);
      artifacts.push({ url, hash, uploadTime });
    }
    if (artifacts.length === 0) {
      fail(`Python package ${name}@${version} has no hashed artifacts`);
    }
    packages.push({ name, version, artifacts });
  }
  if (packages.length === 0) {
    fail("uv.lock contains no PyPI packages");
  }
  return packages;
}

export function parseActionUses(workflows) {
  const actions = [];
  for (const workflow of workflows) {
    for (const match of workflow.text.matchAll(/^\s*-?\s*uses:\s*([^\s#]+).*$/gm)) {
      const target = match[1];
      if (target.startsWith("./")) {
        continue;
      }
      const at = target.lastIndexOf("@");
      const actionPath = at >= 0 ? target.slice(0, at) : target;
      const revision = at >= 0 ? target.slice(at + 1) : "";
      if (!/^[0-9a-f]{40}$/.test(revision)) {
        fail(`${workflow.path}: ${target} must pin an exact 40-character commit`);
      }
      const segments = actionPath.split("/");
      if (segments.length < 2) {
        fail(`${workflow.path}: ${target} is not a repository-backed action`);
      }
      actions.push({ path: workflow.path, repository: `${segments[0]}/${segments[1]}`, revision });
    }
  }
  return actions;
}

export function validateWorkflowSecurity(workflows) {
  for (const workflow of workflows) {
    const { text } = workflow;
    // Every workflow, not only pull-request ones, starts from read-only authority; a job
    // that needs more (a release upload) asks for it in its own job-level block.
    const permissionBlock = text.match(/^permissions:\s*\n((?: {2}[^\n]+\n)+)/m)?.[1];
    const permissions = permissionBlock
      ?.trim()
      .split("\n")
      .map((line) => line.trim())
      .filter(Boolean);
    if (!permissions || permissions.length !== 1 || permissions[0] !== "contents: read") {
      fail(`${workflow.path}: workflows must grant only contents: read at the top level`);
    }
    if (/^\s*permissions:\s*(?:write-all|read-all)\s*$/m.test(text)) {
      fail(`${workflow.path}: blanket write-all or read-all permissions are forbidden`);
    }
    if (!/^\s{2}pull_request:\s*$/m.test(text)) {
      continue;
    }
    // A pull-request job runs code from the pull request, so no job there may widen the
    // read-only default.
    for (const block of text.matchAll(/^( {4,})permissions:\s*\n((?:\1 {2}[^\n]+\n)+)/gm)) {
      if (/:\s*write\s*$/m.test(block[2])) {
        fail(`${workflow.path}: pull-request jobs must not grant write permissions`);
      }
    }
    if (
      /uses:\s+(?:Swatinem\/rust-cache|actions\/cache(?:\/|@))/.test(text) ||
      /^\s+cache:\s*/m.test(text) ||
      /^\s+enable-cache:\s*true\s*$/m.test(text)
    ) {
      fail(`${workflow.path}: reusable cache writes are forbidden in pull-request jobs`);
    }
    for (const line of text.matchAll(/^\s*- run:\s*npm ci(.*)$/gm)) {
      if (!line[1].includes("--ignore-scripts")) {
        fail(`${workflow.path}: npm ci must explicitly disable lifecycle scripts`);
      }
    }

    const lines = text.split("\n");
    for (let index = 0; index < lines.length; index += 1) {
      if (!lines[index].includes("uses: actions/checkout@")) {
        continue;
      }
      const indent = lines[index].match(/^\s*/)[0].length;
      const block = [lines[index]];
      for (let next = index + 1; next < lines.length; next += 1) {
        const nextIndent = lines[next].match(/^\s*/)[0].length;
        if (lines[next].trim() && nextIndent <= indent) {
          break;
        }
        block.push(lines[next]);
      }
      if (!block.join("\n").includes("persist-credentials: false")) {
        fail(`${workflow.path}: checkout must set persist-credentials: false`);
      }
    }

    const jobsText = text.slice(text.indexOf("\njobs:\n") + "\njobs:\n".length);
    const jobs = [];
    let currentJob;
    for (const line of jobsText.split("\n")) {
      const header = line.match(/^ {2}([A-Za-z0-9_-]+):\s*$/);
      if (header) {
        currentJob = { name: header[1], body: "" };
        jobs.push(currentJob);
      } else if (currentJob) {
        currentJob.body += `${line}\n`;
      }
    }
    const supplyChain = jobs.find((job) => job.name === "supply-chain")?.body;
    if (
      !supplyChain?.includes("node --test scripts/check-supply-chain.test.mjs") ||
      !supplyChain.includes("node scripts/check-supply-chain.mjs")
    ) {
      fail(`${workflow.path}: supply-chain job must test and run the provenance gate`);
    }
    if (
      !/GITHUB_TOKEN:\s*\$\{\{\s*(?:github\.token|secrets\.GITHUB_TOKEN)\s*\}\}/.test(
        supplyChain,
      )
    ) {
      fail(`${workflow.path}: supply-chain job must authenticate with the least-privilege workflow token`);
    }
    for (const job of jobs) {
      if (job.name !== "supply-chain" && !/^ {4}needs:\s*supply-chain\s*$/m.test(job.body)) {
        fail(`${workflow.path}: job ${job.name} must wait for the supply-chain gate`);
      }
    }
  }
}

const delay = (milliseconds) => new Promise((resolve) => setTimeout(resolve, milliseconds));

export async function fetchWithRetry(url, headers, fetchImpl = fetch, wait = delay) {
  let lastError;
  for (let attempt = 1; attempt <= 3; attempt += 1) {
    let response;
    try {
      response = await fetchImpl(url, { headers, signal: AbortSignal.timeout(20_000) });
    } catch (error) {
      lastError = error;
    }

    if (response?.ok) {
      return response;
    }
    if (response && response.status !== 429 && response.status < 500) {
      fail(`provenance request failed (${response.status}): ${url}`);
    }
    if (response) {
      lastError = new Error(`HTTP ${response.status}`);
    }
    if (attempt < 3) {
      await wait(250 * attempt);
    }
  }
  fail(`provenance request failed after 3 attempts (${lastError?.message ?? "unknown error"}): ${url}`);
}

export function selectGithubToken(environment, localCredential = "") {
  for (const candidate of [environment.GITHUB_TOKEN, environment.GH_TOKEN, localCredential]) {
    if (typeof candidate === "string" && candidate.trim() !== "") {
      return candidate.trim();
    }
  }
  return undefined;
}

let discoveredGithubToken;

function githubToken() {
  if (discoveredGithubToken !== undefined) {
    return discoveredGithubToken || undefined;
  }

  let localCredential = "";
  if (!selectGithubToken(process.env)) {
    try {
      // Use the already authenticated GitHub CLI for local gates. execFileSync does
      // not invoke a shell, and neither the credential nor stderr is ever printed.
      localCredential = execFileSync("gh", ["auth", "token"], {
        encoding: "utf8",
        stdio: ["ignore", "pipe", "ignore"],
        timeout: 5_000,
      });
    } catch {
      // An unauthenticated request remains a valid fallback while GitHub's public
      // quota is available; exhaustion still fails closed in fetchWithRetry.
    }
  }
  discoveredGithubToken = selectGithubToken(process.env, localCredential) ?? null;
  return discoveredGithubToken || undefined;
}

async function fetchJson(url) {
  const headers = { Accept: "application/json", "User-Agent": "urollup-supply-chain-check" };
  const token = url.startsWith("https://api.github.com/") ? githubToken() : undefined;
  if (token) {
    headers.Authorization = `Bearer ${token}`;
    headers["X-GitHub-Api-Version"] = "2022-11-28";
  }
  const response = await fetchWithRetry(url, headers);
  return response.json();
}

async function fetchBuffer(url) {
  const response = await fetchWithRetry(url, { "User-Agent": "urollup-supply-chain-check" });
  return Buffer.from(await response.arrayBuffer());
}

async function mapLimit(items, limit, callback) {
  const results = new Array(items.length);
  let next = 0;
  async function worker() {
    while (next < items.length) {
      const index = next;
      next += 1;
      results[index] = await callback(items[index]);
    }
  }
  await Promise.all(Array.from({ length: Math.min(limit, items.length) }, () => worker()));
  return results;
}

async function verifyCargo(packages, context) {
  await mapLimit(packages, 8, async (item) => {
    const metadata = await fetchJson(
      `https://crates.io/api/v1/crates/${encodeURIComponent(item.name)}/${encodeURIComponent(item.version)}`,
    );
    const version = metadata.version;
    if (!version || version.num !== item.version || version.yanked) {
      fail(`Cargo package ${item.name}@${item.version} is absent or yanked`);
    }
    assertEqualProvenance(`Cargo ${item.name}@${item.version} checksum`, item.checksum, version.checksum);
    assertAged(
      { ecosystem: "cargo", name: item.name, version: item.version, publishedAt: version.created_at },
      context.now,
      context.minimumAgeDays,
      context.exceptions,
      context.firstParty,
    );
  });
}

async function verifyNpm(packages, context) {
  await mapLimit(packages, 8, async (item) => {
    const metadata = await fetchJson(`https://registry.npmjs.org/${encodeURIComponent(item.name)}`);
    const version = metadata.versions?.[item.version];
    if (!version) {
      fail(`npm package ${item.name}@${item.version} is absent from the registry`);
    }
    assertEqualProvenance(`npm ${item.name}@${item.version} integrity`, item.integrity, version.dist?.integrity);
    assertEqualProvenance(`npm ${item.name}@${item.version} tarball`, item.resolved, version.dist?.tarball);
    assertAged(
      { ecosystem: "npm", name: item.name, version: item.version, publishedAt: metadata.time?.[item.version] },
      context.now,
      context.minimumAgeDays,
      context.exceptions,
      context.firstParty,
    );
  });
}

async function verifyPython(packages, context) {
  await mapLimit(packages, 4, async (item) => {
    const metadata = await fetchJson(
      `https://pypi.org/pypi/${encodeURIComponent(item.name)}/${encodeURIComponent(item.version)}/json`,
    );
    for (const artifact of item.artifacts) {
      const remote = metadata.urls?.find((candidate) => candidate.url === artifact.url);
      if (!remote || remote.yanked) {
        fail(`Python artifact is absent or yanked: ${artifact.url}`);
      }
      assertEqualProvenance(
        `Python ${item.name}@${item.version} artifact hash`,
        artifact.hash,
        `sha256:${remote.digests?.sha256 ?? ""}`,
      );
      assertEqualTimestamp(
        `Python ${item.name}@${item.version} upload time`,
        artifact.uploadTime,
        remote.upload_time_iso_8601,
      );
      assertAged(
        {
          ecosystem: "pypi",
          name: item.name,
          version: item.version,
          publishedAt: remote.upload_time_iso_8601,
        },
        context.now,
        context.minimumAgeDays,
        context.exceptions,
        context.firstParty,
      );
    }
  });
}

async function workflowFiles(root) {
  const directory = path.join(root, ".github", "workflows");
  const names = await readdir(directory);
  return Promise.all(
    names
      .filter((name) => name.endsWith(".yml") || name.endsWith(".yaml"))
      .map(async (name) => ({
        path: path.posix.join(".github", "workflows", name),
        text: await readFile(path.join(directory, name), "utf8"),
      })),
  );
}

/**
 * True for a directory holding another working copy, such as an agent worktree under
 * `.claude/worktrees/`. Its scripts belong to that checkout's own gate, not this one, and
 * a mid-edit copy of this repository would otherwise be reported as this repository's
 * uninventoried downloader.
 */
export function isNestedWorkingCopy(entryNames) {
  return entryNames.includes(".git");
}

async function shellFilesUnder(root, relativeDirectory) {
  const files = [];
  async function walk(absoluteDirectory, relativePath) {
    for (const entry of await readdir(absoluteDirectory, { withFileTypes: true })) {
      const entryRelativePath = path.posix.join(relativePath, entry.name);
      const entryAbsolutePath = path.join(absoluteDirectory, entry.name);
      if (entry.isDirectory()) {
        if (isNestedWorkingCopy(await readdir(entryAbsolutePath))) {
          continue;
        }
        await walk(entryAbsolutePath, entryRelativePath);
      } else if (entry.isFile() && entry.name.endsWith(".sh")) {
        files.push({ path: entryRelativePath, text: await readFile(entryAbsolutePath, "utf8") });
      }
    }
  }
  await walk(path.join(root, relativeDirectory), relativeDirectory);
  return files;
}

export function validateDownloadScripts(files, inventoriedFiles) {
  for (const file of files) {
    if (/\b(?:curl|wget|npx|uvx)\b/.test(file.text) && !inventoriedFiles.has(file.path)) {
      fail(`${file.path} uses a download or package-execution primitive but is absent from the bootstrap inventory`);
    }
  }
}

/**
 * Verify one file pins `item` to its reviewed bootstrap version.
 *
 * The rule is that the exact version is declared in the repository and matches the
 * policy — not that it is spelled inside this particular file. tbd 0.7.x reads its
 * fallback from `.tbd/config.yml` at run time, so a literal-only rule would reject a
 * pin that is in fact tighter than before: the version now lives in one checked place
 * instead of being copied into four shell scripts that could drift apart.
 *
 * Indirection is therefore allowed only in the single shape the policy names, and only
 * after this function has confirmed the named source declares the pinned version. An
 * `npx` line that resolves through anything else, and `@latest` anywhere in the file,
 * still fail closed.
 */
export function validateBootstrapPin(filePath, text, item, versionSourceText = null) {
  const pinned = `${item.name}@${item.version}`;
  if (/@latest\b/.test(text)) {
    fail(`${filePath} must not resolve ${item.name} through @latest`);
  }

  // The version may be written inline, or read at run time from a committed file the
  // policy names. Indirection is only acceptable because that file is itself pinned and
  // checked here: what must stay true is that the exact version is declared in the
  // repository and matches the policy, not that it appears in this particular file.
  const source = item.versionSource;
  let indirect = null;
  if (source) {
    if (versionSourceText === null) {
      fail(`${filePath} declares a version source ${source.file} that could not be read`);
    }
    const declared = new RegExp(`^\\s*${source.key}\\s*:\\s*(\\S+)\\s*$`, "m").exec(versionSourceText);
    if (!declared) {
      fail(`${source.file} does not declare ${source.key} for bootstrap ${item.name}`);
    }
    if (declared[1] !== item.version) {
      fail(
        `${source.file} declares ${source.key} ${declared[1]}, but the bootstrap pin is ${pinned}`,
      );
    }
    indirect = new RegExp(`${escapeRegExp(item.name)}@\\$\\{?${escapeRegExp(source.variable)}\\}?`);
  }

  const hasLiteral = text.includes(pinned);
  const hasIndirect = indirect !== null && indirect.test(text);
  if (!hasLiteral && !hasIndirect) {
    fail(`${filePath} must use only the exact bootstrap ${pinned}`);
  }

  for (const line of text.split("\n").filter((candidate) => /^\s*npx\s/.test(candidate))) {
    const literalLine = line.includes(pinned);
    const indirectLine = indirect !== null && indirect.test(line);
    if (!literalLine && !indirectLine) {
      fail(`${filePath} contains an unreviewed npx execution: ${line.trim()}`);
    }
  }
}

function escapeRegExp(value) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

async function verifyActions(actions, context) {
  const unique = [
    ...new Map(actions.map((item) => [`${item.repository}@${item.revision}`, item])).values(),
  ];
  await mapLimit(unique, 4, async (item) => {
    const commit = await fetchJson(
      `https://api.github.com/repos/${item.repository}/commits/${item.revision}`,
    );
    assertEqualProvenance(`GitHub action ${item.repository} commit`, item.revision, commit.sha);
    if (commit.commit?.verification?.verified !== true) {
      fail(`GitHub action ${item.repository}@${item.revision} commit is not verified`);
    }
    assertAged(
      {
        ecosystem: "github-action",
        name: item.repository,
        version: item.revision,
        publishedAt: commit.commit?.committer?.date,
      },
      context.now,
      context.minimumAgeDays,
      context.exceptions,
      context.firstParty,
    );
  });
}

async function verifyBootstrap(policy, root, context) {
  const inventoriedFiles = new Set(
    [...policy.bootstrap.npm, ...policy.bootstrap.githubReleases].flatMap((item) => item.files),
  );
  const shellFiles = (
    await Promise.all([".claude", ".codex", "scripts"].map((directory) => shellFilesUnder(root, directory)))
  ).flat();
  validateDownloadScripts(shellFiles, inventoriedFiles);

  for (const item of policy.bootstrap.npm) {
    const metadata = await fetchJson(`https://registry.npmjs.org/${encodeURIComponent(item.name)}`);
    const version = metadata.versions?.[item.version];
    if (!version) {
      fail(`bootstrap npm package ${item.name}@${item.version} is absent`);
    }
    assertEqualProvenance(`bootstrap ${item.name}@${item.version} integrity`, item.integrity, version.dist?.integrity);
    assertEqualProvenance(`bootstrap ${item.name}@${item.version} tarball`, item.tarball, version.dist?.tarball);
    assertAged(
      { ecosystem: "npm", name: item.name, version: item.version, publishedAt: metadata.time?.[item.version] },
      context.now,
      context.minimumAgeDays,
      context.exceptions,
      context.firstParty,
    );
    const versionSourceText = item.versionSource
      ? await readFile(path.join(root, item.versionSource.file), "utf8").catch(() => null)
      : null;
    for (const file of item.files) {
      const text = await readFile(path.join(root, file), "utf8");
      validateBootstrapPin(file, text, item, versionSourceText);
    }
  }

  for (const item of policy.bootstrap.githubReleases) {
    const release = await fetchJson(
      `https://api.github.com/repos/${item.repository}/releases/tags/${encodeURIComponent(item.tag)}`,
    );
    if (release.draft || release.prerelease) {
      fail(`bootstrap release ${item.repository}@${item.tag} is not final`);
    }
    assertAged(
      {
        ecosystem: "github-release",
        name: item.repository,
        version: item.tag,
        publishedAt: release.published_at,
      },
      context.now,
      context.minimumAgeDays,
      context.exceptions,
      context.firstParty,
    );
    const fileTexts = await Promise.all(
      item.files.map(async (file) => [file, await readFile(path.join(root, file), "utf8")]),
    );
    for (const [file, text] of fileTexts) {
      const missingDirectDownload =
        item.requireAssetDigestsInFiles &&
        !text.includes(`github.com/${item.repository}/releases/download/`);
      if (!text.includes(item.version) || missingDirectDownload) {
        fail(`${file} does not pin bootstrap release ${item.repository}@${item.version}`);
      }
    }
    for (const [assetName, digest] of Object.entries(item.assets)) {
      const remote = release.assets?.find((asset) => asset.name === assetName);
      assertEqualProvenance(`bootstrap ${item.repository} ${assetName} digest`, digest, remote?.digest);
      if (item.requireAssetDigestsInFiles) {
        for (const [file, text] of fileTexts) {
          if (!text.includes(digest.replace(/^sha256:/, ""))) {
            fail(`${file} is missing the reviewed digest for ${assetName}`);
          }
        }
      }
    }
  }

  for (const item of policy.bootstrap.rustToolchains) {
    const manifest = await fetchBuffer(item.manifest);
    assertEqualProvenance(
      `Rust ${item.version} channel manifest digest`,
      item.sha256,
      createHash("sha256").update(manifest).digest("hex"),
    );
    const releaseDate = manifest.toString("utf8").match(/^date = "([^"]+)"$/m)?.[1];
    assertAged(
      {
        ecosystem: "rust-toolchain",
        name: "rust",
        version: item.version,
        publishedAt: releaseDate ? `${releaseDate}T00:00:00Z` : undefined,
      },
      context.now,
      context.minimumAgeDays,
      context.exceptions,
      context.firstParty,
    );
    for (const file of item.files) {
      const text = await readFile(path.join(root, file), "utf8");
      if (!text.includes(item.version)) {
        fail(`${file} does not pin Rust ${item.version}`);
      }
    }
  }

  for (const item of policy.bootstrap.nodeRuntimes) {
    const releases = await fetchJson("https://nodejs.org/dist/index.json");
    const release = releases.find((candidate) => candidate.version === `v${item.version}`);
    if (!release) {
      fail(`Node.js ${item.version} is absent from the official release index`);
    }
    assertAged(
      {
        ecosystem: "node-runtime",
        name: "node",
        version: item.version,
        publishedAt: `${release.date}T00:00:00Z`,
      },
      context.now,
      context.minimumAgeDays,
      context.exceptions,
      context.firstParty,
    );
    const shasums = await fetchBuffer(item.shasums);
    assertEqualProvenance(
      `Node.js ${item.version} SHASUMS256 digest`,
      item.sha256,
      createHash("sha256").update(shasums).digest("hex"),
    );
    for (const file of item.files) {
      const text = await readFile(path.join(root, file), "utf8");
      if (!text.includes(`node-version: ${item.version}`)) {
        fail(`${file} does not pin Node.js ${item.version}`);
      }
    }
  }

  for (const group of policy.bootstrap.identicalFileGroups) {
    const contents = await Promise.all(group.map((file) => readFile(path.join(root, file), "utf8")));
    if (!contents.every((content) => content === contents[0])) {
      fail(`bootstrap copies have diverged: ${group.join(", ")}`);
    }
  }
}

export function validatePolicy(policy, now) {
  if (policy.schema !== 1 || !Number.isInteger(policy.minimumAgeDays) || policy.minimumAgeDays < 14) {
    fail("supply-chain policy must use schema 1 and a minimum age of at least 14 days");
  }
  assertNoExpiredExceptions(policy.exceptions, now);
  validateFirstParty(policy.firstParty ?? []);
  if (
    !Array.isArray(policy.bootstrap?.npm) ||
    !Array.isArray(policy.bootstrap?.githubReleases) ||
    !Array.isArray(policy.bootstrap?.rustToolchains) ||
    !Array.isArray(policy.bootstrap?.nodeRuntimes) ||
    !Array.isArray(policy.bootstrap?.identicalFileGroups)
  ) {
    fail("supply-chain policy is missing bootstrap inventories");
  }
}

async function main() {
  const policy = JSON.parse(await readFile(path.join(ROOT, "supply-chain-policy.json"), "utf8"));
  const now = new Date();
  validatePolicy(policy, now);
  const context = {
    now,
    minimumAgeDays: policy.minimumAgeDays,
    exceptions: policy.exceptions,
    firstParty: policy.firstParty ?? [],
  };
  const [cargoText, npmTexts, uvTexts, workflows] = await Promise.all([
    readFile(path.join(ROOT, "Cargo.lock"), "utf8"),
    Promise.all(NPM_LOCKS.map((lock) => readFile(path.join(ROOT, ...lock), "utf8"))),
    Promise.all(UV_LOCKS.map((lock) => readFile(path.join(ROOT, ...lock), "utf8"))),
    workflowFiles(ROOT),
  ]);
  const cargo = parseCargoLock(cargoText);
  const npm = npmTexts.flatMap((text) => parseNpmLock(text));
  const python = uvTexts.flatMap((text) => parseUvLock(text));
  const actions = parseActionUses(workflows);
  validateWorkflowSecurity(workflows);

  await Promise.all([
    verifyCargo(cargo, context),
    verifyNpm(npm, context),
    verifyPython(python, context),
    verifyActions(actions, context),
    verifyBootstrap(policy, ROOT, context),
  ]);
  console.log(
    `supply-chain: verified ${cargo.length} Cargo packages, ${npm.length} npm packages, ` +
      `${python.length} Python packages, ${actions.length} action uses, and all bootstrap pins`,
  );
}

if (import.meta.url === pathToFileURL(process.argv[1] ?? "").href) {
  main().catch((error) => {
    console.error(`supply-chain: ${error.message}`);
    process.exitCode = 1;
  });
}
