#!/usr/bin/env node
// Prove the release workflow's authority boundaries
// (docs/project/specs/active/plan-2026-09-16-first-release-publishing.md, Workflow and
// Permissions, and Rehearsal Acceptance).
//
// The supply-chain gate already pins actions and requires a read-only top level for every
// workflow. This gate reads .github/workflows/release.yml as YAML and checks, per job, that:
//
// - only a v* tag push or a workflow_dispatch with an explicit commit and version starts it;
// - plan, gate, build, crates, validate, benchmark and assemble hold no write or OIDC
//   permission, no environment and no secret (this gate is what proves the job token's
//   scope), and check at run time that their environment holds no credential or OIDC
//   endpoint (`release.mjs assert-no-authority`);
// - each job that can write anything runs only when the plan says this is a tag
//   publication, and holds exactly the permission its step needs: id-token and
//   attestations for attestation, id-token for PyPI, contents: write for the GitHub
//   Release, and the crates.io bootstrap secret for crates.io;
// - every publish job enters the protected `release` environment;
// - checkouts never persist credentials, nothing uses a reusable cache, and every uploaded
//   artifact is retained for inspection and fails when empty.

import { readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

import { parse } from "yaml";

import { isCredentialVariable } from "./release-publish.mjs";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
export const WORKFLOW = ".github/workflows/release.yml";

/** Jobs that run in a rehearsal: no authority beyond reading the repository. */
export const REHEARSAL_JOBS = ["plan", "gate", "build", "crates", "validate", "benchmark", "assemble"];

/** Jobs a rehearsal must run `assert-no-authority` in: everything that builds or handles artifacts. */
export const ASSERTING_JOBS = ["build", "crates", "validate", "benchmark", "assemble"];

/**
 * Tag-only jobs and the exact authority each holds. `permissions: null` inherits the
 * read-only top level. `environment` names the protected environment, `secrets` the
 * secrets the job may reference, and `needs` its exact predecessors, which fix the
 * plan's order: attest, stage the draft, both registries, make the release public, probe,
 * then write, attest and attach the evidence.
 */
export const PUBLICATION_JOBS = {
  attest: { permissions: { contents: "read", "id-token": "write", attestations: "write" }, environment: null, secrets: [], needs: ["plan", "assemble"] },
  "stage-github": { permissions: { contents: "write" }, environment: "release", secrets: [], needs: ["plan", "attest"] },
  "publish-crates": { permissions: null, environment: "release", secrets: ["CARGO_REGISTRY_TOKEN"], needs: ["plan", "stage-github"] },
  "publish-pypi": { permissions: { contents: "read", "id-token": "write" }, environment: "release", secrets: [], needs: ["plan", "stage-github"] },
  "publish-github": { permissions: { contents: "write" }, environment: "release", secrets: [], needs: ["plan", "publish-crates", "publish-pypi"] },
  probe: { permissions: { contents: "read", attestations: "read" }, environment: null, secrets: [], needs: ["plan", "publish-github"] },
  evidence: { permissions: null, environment: null, secrets: [], needs: ["plan", "probe"] },
  "attest-evidence": { permissions: { contents: "read", "id-token": "write", attestations: "write" }, environment: null, secrets: [], needs: ["plan", "evidence"] },
  "attach-evidence": { permissions: { contents: "write" }, environment: "release", secrets: [], needs: ["plan", "attest-evidence"] },
};

/** Jobs whose steps may read the workflow token, all for reads except the release writers. */
export const TOKEN_JOBS = ["plan", "gate", "assemble", "stage-github", "publish-github", "probe", "evidence", "attach-evidence"];

/** The only job that may fail without failing the run: informational evidence. */
export const OPTIONAL_JOBS = ["benchmark"];

/** Plan outputs every later job trusts, which must come from the tested plan step. */
export const PLAN_OUTPUTS = ["version", "tag", "commit", "mode", "publish", "matrix", "hosts", "source-date-epoch", "allow-unmerged"];

const EXPRESSION = /\$\{\{[\s\S]*?\}\}/g;
const TOKEN_EXPRESSION = /\bgithub\s*(?:\.\s*token\b|\[\s*['"]token['"]\s*\])|\bsecrets\s*(?:\.\s*GITHUB_TOKEN\b|\[\s*['"]GITHUB_TOKEN['"]\s*\])/;

/**
 * The secrets an expression text references. A reference this cannot name, such as
 * `toJSON(secrets)` or a computed index, is reported as `*`, which no rule allows.
 */
export function secretReferences(text) {
  const names = [];
  for (const [expression] of [...text.matchAll(EXPRESSION)].filter(([candidate]) => /\bsecrets\b/.test(candidate))) {
    let rest = expression;
    for (const match of expression.matchAll(/\bsecrets\s*(?:\.\s*([A-Za-z0-9_]+)|\[\s*['"]([A-Za-z0-9_]+)['"]\s*\])/g)) {
      names.push(match[1] ?? match[2]);
      rest = rest.replace(match[0], "");
    }
    if (/\bsecrets\b/.test(rest)) {
      names.push("*");
    }
  }
  return names;
}

export const PUBLISH_CONDITION = "needs.plan.outputs.publish == 'true'";

function isObject(value) {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function needsList(job) {
  return Array.isArray(job.needs) ? job.needs : typeof job.needs === "string" ? [job.needs] : [];
}

/** Every job a job depends on, directly or not. */
function ancestors(jobs, name, seen = new Set()) {
  for (const parent of needsList(jobs[name] ?? {})) {
    if (!seen.has(parent)) {
      seen.add(parent);
      ancestors(jobs, parent, seen);
    }
  }
  return seen;
}

function samePermissions(actual, expected) {
  if (expected === null) {
    return actual === undefined;
  }
  if (!isObject(actual)) {
    return false;
  }
  const keys = new Set([...Object.keys(actual), ...Object.keys(expected)]);
  return [...keys].every((key) => actual[key] === expected[key]);
}

function describePermissions(value) {
  return value === undefined || value === null ? "none (read-only default)" : JSON.stringify(value);
}

/** The one command the crates.io publish job may run: the script that confines the token. */
export const PUBLISH_CRATES_RUN = 'node scripts/release.mjs publish-crates --bundle "$RUNNER_TEMP/release-bundle"';

/** The one place the crates.io secret may appear: this value of the publish step's env. */
export const CRATES_SECRET = { name: "RELEASE_CRATES_TOKEN", value: "${{ secrets.CARGO_REGISTRY_TOKEN }}" };

/** Every string in a parsed YAML value, with its path. */
function stringsIn(value, where, found = []) {
  if (typeof value === "string") {
    found.push({ where, value });
  } else if (Array.isArray(value)) {
    value.forEach((item, index) => stringsIn(item, `${where}[${index}]`, found));
  } else if (isObject(value)) {
    for (const [key, item] of Object.entries(value)) {
      stringsIn(item, `${where}.${key}`, found);
    }
  }
  return found;
}

/**
 * The crates.io secret is confined to the publish step (review C2): it is referenced
 * exactly once in the whole workflow, as `RELEASE_CRATES_TOKEN: ${{ secrets.CARGO_REGISTRY_TOKEN }}`
 * in the env of the publish-crates step that runs the publish script. publish-crates sets
 * no job-level env and no env on any other step, and the publish step's env holds nothing
 * else, so no action input, action step, other name or other job ever sees the token.
 */
export function crateSecretProblems(workflow, jobs) {
  const problems = [];
  const job = isObject(jobs["publish-crates"]) ? jobs["publish-crates"] : {};
  const steps = Array.isArray(job.steps) ? job.steps : [];
  const index = steps.findIndex((step) => isObject(step) && typeof step.run === "string" && step.run.trim() === PUBLISH_CRATES_RUN);
  const allowed = `workflow.jobs.publish-crates.steps[${index}].env.${CRATES_SECRET.name}`;
  for (const place of stringsIn(workflow, "workflow").filter(({ value }) => secretReferences(value).includes("CARGO_REGISTRY_TOKEN"))) {
    if (index < 0 || place.where !== allowed) {
      problems.push(`${place.where} references secrets.CARGO_REGISTRY_TOKEN; only the env of the publish-crates step that runs the publish script may, as ${CRATES_SECRET.name}`);
    }
  }
  const step = index >= 0 ? steps[index] : null;
  if (!step || typeof step.env?.[CRATES_SECRET.name] !== "string" || step.env[CRATES_SECRET.name].trim() !== CRATES_SECRET.value) {
    problems.push(`the publish-crates step must receive the crates.io secret exactly as ${CRATES_SECRET.name}: ${CRATES_SECRET.value}`);
  }
  const extra = Object.keys(isObject(step?.env) ? step.env : {}).filter((key) => key !== CRATES_SECRET.name);
  if (extra.length > 0) {
    problems.push(`the publish-crates step's env may hold only ${CRATES_SECRET.name}, found ${extra.join(", ")}`);
  }
  if (job.env !== undefined) {
    problems.push("job publish-crates must not set a job-level env, which every step, third-party actions included, would inherit");
  }
  steps.forEach((other, position) => {
    if (position !== index && isObject(other) && other.env !== undefined) {
      problems.push(`job publish-crates step ${position + 1}: only the publish step may set env in the job that holds the crates.io token`);
    }
  });
  return problems;
}

/** Each environment block of the workflow, with where it is, read from the parsed YAML. */
function environmentBlocks(workflow, jobs) {
  const blocks = [{ where: "the workflow", job: null, env: workflow.env }];
  for (const [name, job] of Object.entries(jobs)) {
    if (!isObject(job)) {
      continue;
    }
    blocks.push({ where: `job ${name}`, job: name, env: job.env });
    for (const [index, step] of (job.steps ?? []).entries()) {
      blocks.push({ where: `job ${name} step ${index + 1}`, job: name, env: step.env });
    }
  }
  return blocks.filter((block) => isObject(block.env));
}

/**
 * Credentials reach a process only through named environment entries, which this reads
 * from the parsed workflow, so quoting a key changes nothing:
 * - no environment anywhere exports CARGO_REGISTRY_TOKEN, which Cargo hands to every build
 *   script and proc macro;
 * - a credential-named entry appears only in a job that may hold that credential, never
 *   at the workflow level, and the crates.io secret only where `crateSecretProblems`
 *   allows it;
 * - no step writes to $GITHUB_ENV, which would export a value to every later step; and
 * - publish-crates runs nothing but the publish script.
 */
export function credentialEnvironmentProblems(workflow, jobs) {
  const problems = [];
  for (const block of environmentBlocks(workflow, jobs)) {
    for (const key of Object.keys(block.env)) {
      if (key === "CARGO_REGISTRY_TOKEN") {
        problems.push(`${block.where} exports CARGO_REGISTRY_TOKEN: Cargo hands it to every build script and proc macro; pass the secret under another name to the publish script`);
      } else if (isCredentialVariable(key)) {
        const allowed = key === "RELEASE_CRATES_TOKEN" ? block.job === "publish-crates" : block.job !== null && TOKEN_JOBS.includes(block.job);
        if (!allowed) {
          problems.push(`${block.where} must not set the credential ${key}`);
        }
      }
    }
  }
  for (const [name, job] of Object.entries(jobs)) {
    for (const [index, step] of (isObject(job) ? (job.steps ?? []) : []).entries()) {
      if (typeof step.run !== "string") {
        continue;
      }
      if (/GITHUB_ENV/.test(step.run)) {
        problems.push(`job ${name} step ${index + 1}: release jobs must not write to $GITHUB_ENV, which exports a value to every later step`);
      }
      if (/CARGO_REGISTRY_TOKEN/.test(step.run)) {
        problems.push(`job ${name} step ${index + 1}: run must not name CARGO_REGISTRY_TOKEN`);
      }
      if (name === "publish-crates" && step.run.trim() !== PUBLISH_CRATES_RUN) {
        problems.push(`job publish-crates step ${index + 1}: the job that holds the crates.io token runs only ${PUBLISH_CRATES_RUN}`);
      }
    }
  }
  return problems;
}

/** Validate a parsed release workflow; returns a list of problems. */
export function checkReleaseWorkflow(workflow, text = JSON.stringify(workflow)) {
  const problems = [];
  if (!isObject(workflow)) {
    return [`${WORKFLOW} is not a YAML mapping`];
  }

  const on = workflow.on;
  if (!isObject(on) || JSON.stringify(Object.keys(on).sort()) !== JSON.stringify(["push", "workflow_dispatch"])) {
    problems.push("the release workflow must start only from workflow_dispatch and a tag push");
  } else {
    const push = on.push;
    if (!isObject(push) || JSON.stringify(Object.keys(push)) !== JSON.stringify(["tags"]) || JSON.stringify(push.tags) !== JSON.stringify(["v*"])) {
      problems.push('a release push must be exactly tags: ["v*"], with no branches or paths');
    }
    const inputs = on.workflow_dispatch?.inputs ?? {};
    for (const name of ["commit", "version"]) {
      if (inputs[name]?.required !== true) {
        problems.push(`the rehearsal must require an explicit ${name} input`);
      }
    }
  }
  if (!samePermissions(workflow.permissions, { contents: "read" })) {
    problems.push(`the workflow's top-level permissions must be exactly contents: read, found ${describePermissions(workflow.permissions)}`);
  }
  const concurrency = workflow.concurrency;
  if (!isObject(concurrency) || !/inputs\.version/.test(concurrency.group ?? "") || !/github\.ref_name/.test(concurrency.group ?? "") || concurrency["cancel-in-progress"] !== false) {
    problems.push("the workflow needs a per-version concurrency group that never cancels a run in progress");
  }
  if (/pull_request_target|\bsecrets\s*:\s*inherit/.test(text)) {
    problems.push("the release workflow must not use pull_request_target or inherit secrets");
  }
  for (const key of ["env", "defaults"]) {
    const body = JSON.stringify(workflow[key] ?? {});
    if (secretReferences(body).length > 0 || TOKEN_EXPRESSION.test(body)) {
      problems.push(`the workflow-level ${key} must not carry a secret or the workflow token, which every job would inherit`);
    }
  }
  const planOutputs = workflow.jobs?.plan?.outputs ?? {};
  for (const name of PLAN_OUTPUTS) {
    if (planOutputs[name] !== `\${{ steps.plan.outputs.${name} }}`) {
      problems.push(`plan output ${name} must come from the tested plan step (\${{ steps.plan.outputs.${name} }})`);
    }
  }

  const jobs = isObject(workflow.jobs) ? workflow.jobs : {};
  const known = [...REHEARSAL_JOBS, ...Object.keys(PUBLICATION_JOBS)];
  for (const name of known) {
    if (!isObject(jobs[name])) {
      problems.push(`the release workflow has no ${name} job`);
    }
  }
  for (const name of Object.keys(jobs)) {
    if (!known.includes(name)) {
      problems.push(`job ${name} is not a known release job; give it an authority rule in scripts/check-release-workflow.mjs`);
    }
  }

  problems.push(...credentialEnvironmentProblems(workflow, jobs));
  problems.push(...crateSecretProblems(workflow, jobs));
  for (const [name, job] of Object.entries(jobs)) {
    if (!isObject(job)) {
      continue;
    }
    if (job["continue-on-error"] !== undefined && !OPTIONAL_JOBS.includes(name)) {
      problems.push(`job ${name} must not continue on error: a later job would treat its failure as success`);
    }
    const body = JSON.stringify(job);
    const rule = PUBLICATION_JOBS[name];
    const secrets = secretReferences(body);
    const usesToken = TOKEN_EXPRESSION.test(body);
    if (REHEARSAL_JOBS.includes(name)) {
      if (job.permissions !== undefined) {
        problems.push(`job ${name} runs in rehearsals and must keep the read-only default, but grants ${describePermissions(job.permissions)}`);
      }
      if (job.environment !== undefined) {
        problems.push(`job ${name} runs in rehearsals and must not enter an environment`);
      }
      if (secrets.length > 0) {
        problems.push(`job ${name} runs in rehearsals and must not read secrets (${secrets.join(", ")})`);
      }
      if (job.if !== undefined) {
        problems.push(`job ${name} must run in every release run, rehearsal or publication`);
      }
    } else if (rule) {
      if (!samePermissions(job.permissions, rule.permissions)) {
        problems.push(`job ${name} must hold exactly ${describePermissions(rule.permissions)}, found ${describePermissions(job.permissions)}`);
      }
      if ((job.environment ?? null) !== rule.environment) {
        problems.push(`job ${name} must ${rule.environment ? `enter the ${rule.environment} environment` : "not enter an environment"}`);
      }
      const unexpected = secrets.filter((secret) => !rule.secrets.includes(secret));
      if (unexpected.length > 0) {
        problems.push(`job ${name} must not read secrets ${unexpected.join(", ")}`);
      }
      if (job.if !== PUBLISH_CONDITION) {
        problems.push(`job ${name} must run only for a tag publication: if: ${PUBLISH_CONDITION}`);
      }
      if (!needsList(job).includes("plan")) {
        problems.push(`job ${name} must need plan directly, so its condition reads the plan`);
      }
      if (JSON.stringify([...needsList(job)].sort()) !== JSON.stringify([...rule.needs].sort())) {
        problems.push(`job ${name} must need exactly ${rule.needs.join(", ")}, found ${needsList(job).join(", ") || "nothing"}`);
      }
      if (!ancestors(jobs, name).has("assemble")) {
        problems.push(`job ${name} must come after assemble, so nothing publishes before every artifact is validated`);
      }
    }
    if (usesToken && !TOKEN_JOBS.includes(name)) {
      problems.push(`job ${name} must not read the workflow token`);
    }
    if (ASSERTING_JOBS.includes(name) && !(job.steps ?? []).some((step) => /release\.mjs assert-no-authority/.test(step.run ?? ""))) {
      problems.push(`job ${name} must check at run time that its environment holds no credential or OIDC endpoint (release.mjs assert-no-authority)`);
    }
    for (const [index, step] of (job.steps ?? []).entries()) {
      const uses = typeof step.uses === "string" ? step.uses : "";
      const where = `job ${name} step ${index + 1}`;
      if (step["continue-on-error"] !== undefined) {
        problems.push(`${where}: steps must not continue on error`);
      }
      if (typeof step.run === "string" && step.run.includes("${{")) {
        problems.push(`${where}: run must not expand \${{ }} expressions into the shell; pass values through env`);
      }
      if (uses.startsWith("actions/download-artifact@") && !step.with?.name && !step.with?.pattern) {
        problems.push(`${where}: download artifacts by name or pattern, never every artifact of the run`);
      }
      if (uses.startsWith("actions/checkout@") && step.with?.["persist-credentials"] !== false) {
        problems.push(`${where}: checkout must set persist-credentials: false`);
      }
      if (/^(actions\/cache|Swatinem\/rust-cache)[@/]/.test(uses) || step.with?.cache === true || step.with?.["enable-cache"] === true) {
        problems.push(`${where}: release jobs must not use a reusable cache`);
      }
      if (uses.startsWith("astral-sh/setup-uv@") && step.with?.["enable-cache"] !== false) {
        problems.push(`${where}: setup-uv must set enable-cache: false`);
      }
      if (uses.startsWith("actions/upload-artifact@")) {
        const days = step.with?.["retention-days"];
        if (!Number.isInteger(days) || days < 14) {
          problems.push(`${where}: uploaded release artifacts must be retained at least 14 days for review`);
        }
        if (step.with?.["if-no-files-found"] !== "error") {
          problems.push(`${where}: an artifact upload must fail when there is nothing to upload`);
        }
      }
    }
  }

  for (const name of ["build", "validate"]) {
    if (jobs[name] && !/needs\.plan\.outputs\.matrix/.test(JSON.stringify(jobs[name].strategy ?? {}))) {
      problems.push(`job ${name} must take its matrix from the plan, so no target can drop out of the workflow alone`);
    }
  }
  const assembleNeeds = ancestors(jobs, "assemble");
  for (const name of ["gate", "crates", "validate", "build", "plan"]) {
    if (!assembleNeeds.has(name)) {
      problems.push(`assemble must come after ${name}`);
    }
  }
  return problems;
}

function main() {
  const text = readFileSync(path.join(ROOT, WORKFLOW), "utf8");
  const problems = checkReleaseWorkflow(parse(text), text);
  if (problems.length > 0) {
    console.error(`check-release-workflow: ${WORKFLOW} breaks its authority boundaries:`);
    for (const problem of problems) {
      console.error(`  - ${problem}`);
    }
    process.exitCode = 1;
    return;
  }
  console.log(`check-release-workflow: ${WORKFLOW} keeps every build and validation job read-only and every write behind the tag and the release environment`);
}

if (import.meta.url === pathToFileURL(process.argv[1] ?? "").href) {
  main();
}
