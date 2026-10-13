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
 * PyPI: `observed` is `{status: "absent"}`, `{status: "unavailable", reason}` or
 * `{status: "present", files: [{name, sha256, packagetype, yanked}]}`. An sdist, a yanked
 * file or an unplanned wheel under this version is a conflict.
 */
export function decidePypi(planned, observed) {
  if (observed.status === "unavailable") {
    return decision("pypi", "unavailable", { publish: [], skip: [], conflicts: [observed.reason ?? "PyPI could not be read"] });
  }
  if (observed.status === "absent") {
    return decision("pypi", "absent", { publish: planned.map((item) => item.name), skip: [], conflicts: [] });
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
 * a conflict, since Cargo never orders a workspace publication that way.
 */
export function decideCrates(planned, observed) {
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

/** The command a crates.io decision runs, or null when there is nothing to publish. */
export function cratesPublishCommand(result) {
  if (result.blocking || result.publish.length === 0) {
    return null;
  }
  if (result.state === "absent") {
    return ["cargo", "publish", "--locked", "--workspace"];
  }
  return ["cargo", "publish", "--locked", ...result.publish.flatMap((name) => ["-p", name])];
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

/**
 * Stage the draft release: create it if absent, upload only the planned assets it lacks,
 * and then re-read it to prove every planned asset is present with its planned digest.
 * `files` is `[{name, sha256, data}]`. Refuses any blocking state.
 */
export async function stageGithubRelease({ repository, tag, commit, title, notes, prerelease, files, token, fetchImpl = fetch }) {
  const planned = files.map((file) => ({ name: file.name, sha256: file.sha256 }));
  let observed = await observeGithubRelease(repository, tag, { token, includeDrafts: true, fetchImpl });
  // An upload interrupted in an earlier attempt leaves a planned asset with no digest or an
  // unfinished state. While the release is still a private draft, remove it so this run can
  // upload it again; on a public release it stays a conflict.
  const repaired = [];
  if (observed.status === "present" && observed.draft) {
    for (const asset of observed.assets.filter((candidate) => planned.some((item) => item.name === candidate.name))) {
      if (asset.state !== "uploaded" || asset.sha256 === null) {
        await githubWrite(fetchImpl, `https://api.github.com/repos/${repository}/releases/assets/${asset.id}`, { method: "DELETE", token });
        repaired.push(asset.name);
      }
    }
    if (repaired.length > 0) {
      observed = await observeGithubRelease(repository, tag, { token, includeDrafts: true, fetchImpl });
    }
  }
  const result = decideGithubRelease(planned, observed);
  if (result.blocking) {
    throw new Error(describeDecision(result));
  }
  let releaseId = observed.id;
  if (observed.status === "absent") {
    const created = await githubWrite(fetchImpl, `https://api.github.com/repos/${repository}/releases`, {
      method: "POST",
      token,
      body: JSON.stringify({ tag_name: tag, target_commitish: commit, name: title, body: notes, draft: true, prerelease }),
    });
    releaseId = created.id;
  }
  for (const name of result.publish) {
    const file = files.find((candidate) => candidate.name === name);
    await githubWrite(fetchImpl, `https://uploads.github.com/repos/${repository}/releases/${releaseId}/assets?name=${encodeURIComponent(name)}`, {
      method: "POST",
      token,
      body: file.data,
      contentType: "application/octet-stream",
    });
  }
  const after = decideGithubRelease(planned, await observeGithubRelease(repository, tag, { token, includeDrafts: true, fetchImpl }));
  if (after.state !== "identical-complete") {
    throw new Error(`after staging, ${describeDecision(after)}`);
  }
  return { releaseId, staged: result, repaired };
}

/** Make the staged draft public, only when it is identical and complete. */
export async function publishGithubRelease({ repository, tag, files, token, fetchImpl = fetch }) {
  const planned = files.map((file) => ({ name: file.name, sha256: file.sha256 }));
  const observed = await observeGithubRelease(repository, tag, { token, includeDrafts: true, fetchImpl });
  const result = decideGithubRelease(planned, observed);
  if (result.state !== "identical-complete") {
    throw new Error(`refusing to publish: ${describeDecision(result)}`);
  }
  if (observed.draft) {
    await githubWrite(fetchImpl, `https://api.github.com/repos/${repository}/releases/${observed.id}`, {
      method: "PATCH",
      token,
      body: JSON.stringify({ draft: false }),
    });
  }
  return { releaseId: observed.id, alreadyPublic: !observed.draft };
}
