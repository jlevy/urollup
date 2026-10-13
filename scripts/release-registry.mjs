// Registry state and the channel actions a release run would take.
//
// Registry publication is not transactional, so a rerun must tell a partial publication it
// may resume from one it must not touch (plan, Failure and Recovery Contract). Each channel
// is observed read-only and classified as:
//
// - absent: nothing is published; publish everything planned.
// - identical-complete: every planned file is published with the planned digest; skip.
// - identical-partial: what is published matches, and some planned files are missing;
//   publish only those.
// - conflict: something published under this version differs from the plan, or is not in
//   it; stop. A checksum conflict is never success.
// - unavailable: the registry could not be read; stop, since unknown is not absent.
//
// The observers make anonymous reads with a generic User-Agent that names no person,
// account or repository. The GitHub write operations run only in the protected publish jobs.

export const USER_AGENT = "urollup-release-tooling";

export const STATES = ["absent", "identical-complete", "identical-partial", "conflict", "unavailable"];

/**
 * GitHub Release assets added after the release is public, never staged: every GitHub
 * decision ignores them, so a rerun after evidence is attached still sees the planned set.
 */
export const LATE_ASSETS = ["release-evidence.json"];

function decision(channel, state, details) {
  const blocking = state === "conflict" || state === "unavailable";
  return { ...details, channel, state, blocking };
}

function plannedMap(planned) {
  const map = new Map();
  for (const item of planned) {
    if (map.has(item.name)) {
      throw new Error(`planned ${item.name} twice`);
    }
    map.set(item.name, item.sha256);
  }
  if (map.size === 0) {
    throw new Error("there is nothing planned to publish");
  }
  return map;
}

/**
 * Classify published files against planned ones, both as `{name, sha256}`. An observed
 * file with no digest is a conflict too: it cannot be shown identical.
 */
function classifyFiles(channel, planned, observedFiles, extraFacts = {}) {
  const expected = plannedMap(planned);
  const conflicts = [];
  const present = [];
  for (const file of observedFiles) {
    if (!expected.has(file.name)) {
      conflicts.push(`${file.name} is published but not planned`);
    } else if (file.sha256 !== expected.get(file.name)) {
      conflicts.push(`${file.name} is published with digest ${file.sha256 ?? "unknown"}, planned ${expected.get(file.name)}`);
    } else {
      present.push(file.name);
    }
  }
  const missing = [...expected.keys()].filter((name) => !present.includes(name));
  if (conflicts.length > 0) {
    return decision(channel, "conflict", { publish: [], skip: present, conflicts, ...extraFacts });
  }
  if (present.length === 0) {
    return decision(channel, "absent", { publish: missing, skip: [], conflicts: [], ...extraFacts });
  }
  if (missing.length === 0) {
    return decision(channel, "identical-complete", { publish: [], skip: present, conflicts: [], ...extraFacts });
  }
  return decision(channel, "identical-partial", { publish: missing, skip: present, conflicts: [], ...extraFacts });
}

/**
 * GitHub Release: `observed` is `{status: "absent"}`, `{status: "unavailable", reason}` or
 * `{status: "present", draft, assets: [{name, sha256}]}`. A published (non-draft) release
 * that is not identical and complete is a conflict, because its assets are public. Assets
 * named in `late` are ignored.
 */
export function decideGithubRelease(planned, observed, { late = LATE_ASSETS } = {}) {
  if (observed.status === "present") {
    observed = { ...observed, assets: (observed.assets ?? []).filter((asset) => !late.includes(asset.name)) };
  }
  if (observed.status === "unavailable") {
    return decision("github", "unavailable", { publish: [], skip: [], conflicts: [observed.reason ?? "GitHub could not be read"] });
  }
  if (observed.status === "absent") {
    return decision("github", "absent", { publish: planned.map((item) => item.name), skip: [], conflicts: [], release: "create-draft" });
  }
  const result = classifyFiles("github", planned, observed.assets ?? [], { release: observed.draft ? "existing-draft" : "published" });
  if (!observed.draft && result.state === "identical-partial") {
    return decision("github", "conflict", {
      ...result,
      publish: [],
      conflicts: [`the release is already public but lacks ${result.publish.join(", ")}`],
    });
  }
  if (result.state === "absent" && !observed.draft) {
    return decision("github", "conflict", { ...result, publish: [], conflicts: ["the release is already public with none of the planned assets"] });
  }
  return result;
}

/**
 * A first release publishes into names nobody holds yet. A project that already exists
 * without the planned version belongs to someone else (a pending PyPI publisher does not
 * reserve a name), so it is a conflict rather than "absent". `project` is the project-level
 * observation; anything but a clean "absent" is refused.
 */
function firstReleaseConflict(channel, name, project) {
  if (project === undefined) {
    return null;
  }
  if (project.status === "unavailable") {
    return decision(channel, "unavailable", { publish: [], skip: [], conflicts: [`${name}: ${project.reason ?? "the project could not be read"}`] });
  }
  if (project.status === "present") {
    return decision(channel, "conflict", {
      publish: [],
      skip: [],
      conflicts: [`${name} already exists without this version; a first release never publishes into an existing project`],
    });
  }
  return null;
}

/**
 * PyPI: `observed` is `{status: "absent"}`, `{status: "unavailable", reason}` or
 * `{status: "present", files: [{name, sha256, packagetype, yanked}]}`. An sdist, a yanked
 * file or an unplanned wheel under this version is a conflict. With `firstRelease`, an
 * existing `project` without the version is a conflict too.
 */
export function decidePypi(planned, observed, { firstRelease = false, project } = {}) {
  if (observed.status === "unavailable") {
    return decision("pypi", "unavailable", { publish: [], skip: [], conflicts: [observed.reason ?? "PyPI could not be read"] });
  }
  if (observed.status === "absent") {
    const taken = firstRelease ? firstReleaseConflict("pypi", "PyPI project urollup", project) : null;
    return taken ?? decision("pypi", "absent", { publish: planned.map((item) => item.name), skip: [], conflicts: [] });
  }
  const files = observed.files ?? [];
  const result = classifyFiles("pypi", planned, files);
  const extra = [];
  for (const file of files) {
    if (file.packagetype && file.packagetype !== "bdist_wheel") {
      extra.push(`${file.name} is a ${file.packagetype}; urollup publishes wheels only`);
    }
    if (file.yanked) {
      extra.push(`${file.name} is yanked; a yanked version is never republished`);
    }
  }
  if (extra.length > 0) {
    return decision("pypi", "conflict", { ...result, publish: [], conflicts: [...result.conflicts, ...extra] });
  }
  return result;
}

/**
 * crates.io: `planned` is `[{name: crate, sha256}]` in publication order, and `observed`
 * maps each crate to `{status: "absent"}`, `{status: "unavailable", reason}` or
 * `{status: "present", checksum, yanked}`. A dependent published without its dependency is
 * a conflict, since Cargo never orders a workspace publication that way. With
 * `firstRelease`, a crate whose project exists without the version is a conflict.
 */
export function decideCrates(planned, observed, { firstRelease = false, projects = {} } = {}) {
  if (firstRelease) {
    for (const item of planned) {
      if (observed[item.name]?.status === "absent") {
        const taken = firstReleaseConflict("crates", `crates.io crate ${item.name}`, projects[item.name]);
        if (taken) {
          return taken;
        }
      }
    }
  }
  const unavailable = planned.filter((item) => observed[item.name]?.status === "unavailable" || observed[item.name] === undefined);
  if (unavailable.length > 0) {
    return decision("crates", "unavailable", {
      publish: [],
      skip: [],
      conflicts: unavailable.map((item) => `${item.name}: ${observed[item.name]?.reason ?? "crates.io could not be read"}`),
    });
  }
  const files = planned
    .filter((item) => observed[item.name].status === "present")
    .map((item) => ({ name: item.name, sha256: observed[item.name].checksum ?? null }));
  const result = classifyFiles("crates", planned, files);
  const extra = planned.filter((item) => observed[item.name].status === "present" && observed[item.name].yanked).map((item) => `${item.name} is yanked`);
  const order = planned.map((item) => item.name);
  const firstMissing = order.findIndex((name) => observed[name].status !== "present");
  if (firstMissing >= 0 && order.slice(firstMissing).some((name) => observed[name].status === "present")) {
    extra.push(`${order.slice(firstMissing + 1).find((name) => observed[name].status === "present")} is published before its dependency ${order[firstMissing]}`);
  }
  if (extra.length > 0) {
    return decision("crates", "conflict", { ...result, publish: [], conflicts: [...result.conflicts, ...extra] });
  }
  return result;
}

/**
 * The command a crates.io decision runs, or null when there is nothing to publish.
 * `--no-verify` is load-bearing: verification compiles every dependency's build script
 * and proc macro, and in the publish job the crates.io credential is in Cargo's
 * environment. The token-free `crates` job already verified these exact bytes, and the
 * publish job proves the digests match before it uploads.
 */
export function cratesPublishCommand(result) {
  if (result.blocking || result.publish.length === 0) {
    return null;
  }
  if (result.state === "absent") {
    return ["cargo", "publish", "--locked", "--no-verify", "--workspace"];
  }
  return ["cargo", "publish", "--locked", "--no-verify", ...result.publish.flatMap((name) => ["-p", name])];
}

/** One line per decision, for the run summary. */
export function describeDecision(result) {
  const parts = [`${result.channel}: ${result.state}`];
  if (result.publish.length > 0) {
    parts.push(`would publish ${result.publish.join(", ")}`);
  }
  if (result.skip.length > 0) {
    parts.push(`already identical: ${result.skip.join(", ")}`);
  }
  if (result.conflicts.length > 0) {
    parts.push(`stop: ${result.conflicts.join("; ")}`);
  }
  return parts.join("; ");
}

// ---------------------------------------------------------------------------------------
// Observation (read-only)

async function getJson(fetchImpl, url, headers = {}) {
  let response;
  try {
    response = await fetchImpl(url, { headers: { "User-Agent": USER_AGENT, Accept: "application/json", ...headers } });
  } catch (error) {
    return { status: "unavailable", reason: `${url}: ${error.message}` };
  }
  if (response.status === 404) {
    return { status: "absent" };
  }
  if (!response.ok) {
    return { status: "unavailable", reason: `${url} returned HTTP ${response.status}` };
  }
  try {
    return { status: "present", body: await response.json() };
  } catch (error) {
    return { status: "unavailable", reason: `${url} returned unreadable JSON: ${error.message}` };
  }
}

/** Observe one crate version on crates.io. */
export async function observeCrate(name, version, fetchImpl = fetch) {
  const result = await getJson(fetchImpl, `https://crates.io/api/v1/crates/${encodeURIComponent(name)}/${encodeURIComponent(version)}`);
  if (result.status !== "present") {
    return result;
  }
  const published = result.body?.version;
  if (published?.num !== version || typeof published.checksum !== "string") {
    return { status: "unavailable", reason: `crates.io returned an unexpected record for ${name} ${version}` };
  }
  return { status: "present", checksum: published.checksum, yanked: Boolean(published.yanked) };
}

/** Observe whether a crate exists at all on crates.io, whatever its versions. */
export async function observeCrateProject(name, fetchImpl = fetch) {
  const result = await getJson(fetchImpl, `https://crates.io/api/v1/crates/${encodeURIComponent(name)}`);
  return result.status === "present" ? { status: "present" } : result;
}

/** Observe whether a PyPI project exists at all, whatever its versions. */
export async function observePypiProject(project, fetchImpl = fetch) {
  const result = await getJson(fetchImpl, `https://pypi.org/pypi/${encodeURIComponent(project)}/json`);
  return result.status === "present" ? { status: "present" } : result;
}

/** Observe one release on PyPI. */
export async function observePypi(project, version, fetchImpl = fetch) {
  const result = await getJson(fetchImpl, `https://pypi.org/pypi/${encodeURIComponent(project)}/${encodeURIComponent(version)}/json`);
  if (result.status !== "present") {
    return result;
  }
  const urls = result.body?.urls;
  if (!Array.isArray(urls)) {
    return { status: "unavailable", reason: `PyPI returned an unexpected record for ${project} ${version}` };
  }
  return {
    status: "present",
    files: urls.map((file) => ({ name: file.filename, sha256: file.digests?.sha256 ?? null, packagetype: file.packagetype, yanked: Boolean(file.yanked) })),
  };
}

function githubHeaders(token) {
  return { Accept: "application/vnd.github+json", "X-GitHub-Api-Version": "2022-11-28", ...(token ? { Authorization: `Bearer ${token}` } : {}) };
}

function assetDigest(asset) {
  return typeof asset.digest === "string" && asset.digest.startsWith("sha256:") ? asset.digest.slice("sha256:".length) : null;
}

/**
 * Observe the GitHub Release for a tag. A read-only token sees published releases only;
 * the publish jobs' write token also sees drafts, which is when `includeDrafts` is set.
 */
export async function observeGithubRelease(repository, tag, { token, includeDrafts = false, fetchImpl = fetch } = {}) {
  if (includeDrafts) {
    const releases = [];
    for (let page = 1; page <= 10; page += 1) {
      const listed = await getJson(fetchImpl, `https://api.github.com/repos/${repository}/releases?per_page=100${page > 1 ? `&page=${page}` : ""}`, githubHeaders(token));
      if (listed.status !== "present") {
        return listed.status === "absent" ? { status: "unavailable", reason: `${repository} has no releases endpoint` } : listed;
      }
      releases.push(...listed.body);
      if (listed.body.length < 100) {
        break;
      }
    }
    const matches = releases.filter((release) => release.tag_name === tag);
    if (matches.length > 1) {
      return { status: "unavailable", reason: `${matches.length} releases name tag ${tag}` };
    }
    if (matches.length === 0) {
      return { status: "absent" };
    }
    return releaseObservation(matches[0]);
  }
  const result = await getJson(fetchImpl, `https://api.github.com/repos/${repository}/releases/tags/${encodeURIComponent(tag)}`, githubHeaders(token));
  return result.status === "present" ? releaseObservation(result.body) : result;
}

function releaseObservation(release) {
  return {
    status: "present",
    id: release.id,
    draft: Boolean(release.draft),
    name: release.name ?? null,
    body: release.body ?? null,
    prerelease: Boolean(release.prerelease),
    target: release.target_commitish ?? null,
    assets: (release.assets ?? []).map((asset) => ({ name: asset.name, sha256: assetDigest(asset), id: asset.id, state: asset.state ?? "uploaded" })),
  };
}

// ---------------------------------------------------------------------------------------
// GitHub Release writes (protected publish jobs only)

async function githubWrite(fetchImpl, url, { method, token, body, contentType }) {
  const response = await fetchImpl(url, {
    method,
    headers: { "User-Agent": USER_AGENT, ...githubHeaders(token), "Content-Type": contentType ?? "application/json" },
    body,
  });
  if (!response.ok) {
    throw new Error(`${method} ${url} returned HTTP ${response.status}`);
  }
  return response.status === 204 ? null : response.json();
}

/** Fields a draft must carry before it is published, as the plan writes them. */
export function releaseMetadata({ tag, commit, title, notes, prerelease }) {
  return { tag_name: tag, target_commitish: commit, name: title, body: notes, prerelease: Boolean(prerelease) };
}

/** The metadata fields where an observed release differs from the planned values. */
export function metadataDifferences(observed, planned) {
  const actual = { name: observed.name, body: observed.body, prerelease: observed.prerelease, target_commitish: observed.target };
  return ["name", "body", "prerelease", "target_commitish"].filter((field) => actual[field] !== planned[field]);
}

const realSleep = (milliseconds) => new Promise((resolve) => setTimeout(resolve, milliseconds));

/**
 * Re-read the release until every planned asset GitHub holds reports a digest. GitHub
 * computes an asset's SHA-256 at upload; this tolerates the digest appearing a moment
 * later, and still fails closed when it never does.
 */
async function observeWithDigests({ repository, tag, token, fetchImpl, sleep, planned, attempts = 6, delayMs = 5_000 }) {
  let observed;
  for (let attempt = 1; attempt <= attempts; attempt += 1) {
    observed = await observeGithubRelease(repository, tag, { token, includeDrafts: true, fetchImpl });
    const pending = observed.status === "present" && observed.assets.some((asset) => planned.some((item) => item.name === asset.name) && asset.sha256 === null);
    if (!pending) {
      return observed;
    }
    await sleep(delayMs);
  }
  return observed;
}

/**
 * Stage the draft release: create it if absent, bring an existing draft's name, notes,
 * prerelease flag and target to the planned values, upload only the planned assets it
 * lacks, and then re-read it to prove every planned asset is present with its planned
 * digest. `files` is `[{name, sha256, data}]`. `beforeWrite()` runs immediately before
 * every write, so the caller can re-check the tag's target. Refuses any blocking state.
 */
export async function stageGithubRelease({ repository, tag, commit, title, notes, prerelease, files, token, fetchImpl = fetch, beforeWrite = async () => {}, sleep = realSleep }) {
  const planned = files.map((file) => ({ name: file.name, sha256: file.sha256 }));
  const metadata = releaseMetadata({ tag, commit, title, notes, prerelease });
  const write = async (url, options) => {
    await beforeWrite();
    return githubWrite(fetchImpl, url, { ...options, token });
  };
  let observed = await observeGithubRelease(repository, tag, { token, includeDrafts: true, fetchImpl });
  // An upload interrupted in an earlier attempt leaves a planned asset with no digest or an
  // unfinished state. While the release is still a private draft, remove it so this run can
  // upload it again; on a public release it stays a conflict.
  const repaired = [];
  let reconciled = [];
  if (observed.status === "present" && observed.draft) {
    for (const asset of observed.assets.filter((candidate) => planned.some((item) => item.name === candidate.name))) {
      if (asset.state !== "uploaded" || asset.sha256 === null) {
        await write(`https://api.github.com/repos/${repository}/releases/assets/${asset.id}`, { method: "DELETE" });
        repaired.push(asset.name);
      }
    }
    reconciled = metadataDifferences(observed, metadata);
    if (reconciled.length > 0) {
      await write(`https://api.github.com/repos/${repository}/releases/${observed.id}`, {
        method: "PATCH",
        body: JSON.stringify(Object.fromEntries(reconciled.map((field) => [field, metadata[field]]))),
      });
    }
    if (repaired.length > 0 || reconciled.length > 0) {
      observed = await observeGithubRelease(repository, tag, { token, includeDrafts: true, fetchImpl });
    }
  }
  const result = decideGithubRelease(planned, observed);
  if (result.blocking) {
    throw new Error(describeDecision(result));
  }
  if (observed.status === "present" && !observed.draft && metadataDifferences(observed, metadata).length > 0) {
    throw new Error(`the public ${tag} release differs from the plan in ${metadataDifferences(observed, metadata).join(", ")}`);
  }
  let releaseId = observed.id;
  if (observed.status === "absent") {
    const created = await write(`https://api.github.com/repos/${repository}/releases`, {
      method: "POST",
      body: JSON.stringify({ ...metadata, draft: true }),
    });
    releaseId = created.id;
  }
  for (const name of result.publish) {
    const file = files.find((candidate) => candidate.name === name);
    await write(`https://uploads.github.com/repos/${repository}/releases/${releaseId}/assets?name=${encodeURIComponent(name)}`, {
      method: "POST",
      body: file.data,
      contentType: "application/octet-stream",
    });
  }
  const after = decideGithubRelease(planned, await observeWithDigests({ repository, tag, token, fetchImpl, sleep, planned }));
  if (after.state !== "identical-complete") {
    throw new Error(`after staging, ${describeDecision(after)}`);
  }
  return { releaseId, staged: result, repaired, reconciled };
}

/**
 * Make the staged draft public, only when its assets are identical and complete and its
 * name, notes, prerelease flag and target are the planned ones. `beforeWrite()` runs
 * immediately before the write.
 */
export async function publishGithubRelease({ repository, tag, commit, title, notes, prerelease, files, token, fetchImpl = fetch, beforeWrite = async () => {} }) {
  const planned = files.map((file) => ({ name: file.name, sha256: file.sha256 }));
  const observed = await observeGithubRelease(repository, tag, { token, includeDrafts: true, fetchImpl });
  const result = decideGithubRelease(planned, observed);
  if (result.state !== "identical-complete") {
    throw new Error(`refusing to publish: ${describeDecision(result)}`);
  }
  const differences = metadataDifferences(observed, releaseMetadata({ tag, commit, title, notes, prerelease }));
  if (differences.length > 0) {
    throw new Error(`refusing to publish: the release's ${differences.join(", ")} differ from the plan; rerun stage-github`);
  }
  if (observed.draft) {
    await beforeWrite();
    await githubWrite(fetchImpl, `https://api.github.com/repos/${repository}/releases/${observed.id}`, {
      method: "PATCH",
      token,
      body: JSON.stringify({ draft: false }),
    });
  }
  return { releaseId: observed.id, alreadyPublic: !observed.draft };
}
