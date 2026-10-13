import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

import { parse } from "yaml";

import { PUBLICATION_JOBS, PUBLISH_CONDITION, PUBLISH_CRATES_RUN, REHEARSAL_JOBS, WORKFLOW, checkReleaseWorkflow, crateSecretProblems, secretReferences } from "./check-release-workflow.mjs";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const TEXT = readFileSync(path.join(ROOT, WORKFLOW), "utf8");

function workflow() {
  return parse(TEXT);
}

function problemsAfter(change) {
  const copy = workflow();
  change(copy);
  return checkReleaseWorkflow(copy).join("\n");
}

test("the committed release workflow keeps its authority boundaries", () => {
  assert.deepEqual(checkReleaseWorkflow(workflow(), TEXT), []);
});

test("rehearsal jobs can never gain write, OIDC, attestation, environment or secret authority", () => {
  for (const job of REHEARSAL_JOBS) {
    assert.match(problemsAfter((copy) => (copy.jobs[job].permissions = { contents: "read", "id-token": "write" })), new RegExp(`job ${job} runs in rehearsals and must keep the read-only default`));
  }
  assert.match(problemsAfter((copy) => (copy.jobs.build.permissions = { attestations: "write" })), /job build runs in rehearsals/);
  assert.match(problemsAfter((copy) => (copy.jobs.validate.permissions = { contents: "write" })), /job validate runs in rehearsals/);
  assert.match(problemsAfter((copy) => (copy.jobs.build.environment = "release")), /job build runs in rehearsals and must not enter an environment/);
  assert.match(
    problemsAfter((copy) => copy.jobs.crates.steps.push({ run: "cargo publish", env: { CARGO_REGISTRY_TOKEN: "${{ secrets.CARGO_REGISTRY_TOKEN }}" } })),
    /job crates runs in rehearsals and must not read secrets \(CARGO_REGISTRY_TOKEN\)/,
  );
  assert.match(problemsAfter((copy) => copy.jobs.build.steps.push({ run: "x", env: { GITHUB_TOKEN: "${{ github.token }}" } })), /job build must not read the workflow token/);
  assert.match(
    problemsAfter((copy) => (copy.jobs.build.steps = copy.jobs.build.steps.filter((step) => !/assert-no-authority/.test(step.run ?? "")))),
    /job build must check at run time that its environment holds no credential/,
  );
  assert.match(problemsAfter((copy) => (copy.jobs.assemble.if = "always()")), /job assemble must run in every release run/);
});

test("publication jobs hold exactly their authority, only for a tag, inside the release environment", () => {
  for (const name of Object.keys(PUBLICATION_JOBS)) {
    assert.match(problemsAfter((copy) => delete copy.jobs[name].if), new RegExp(`job ${name} must run only for a tag publication: if: ${PUBLISH_CONDITION.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}`));
  }
  assert.match(problemsAfter((copy) => (copy.jobs["publish-pypi"].permissions = { contents: "write", "id-token": "write" })), /job publish-pypi must hold exactly/);
  assert.match(problemsAfter((copy) => (copy.jobs["publish-crates"].permissions = { "id-token": "write" })), /job publish-crates must hold exactly none/);
  assert.match(problemsAfter((copy) => (copy.jobs.attest.permissions = { "id-token": "write", attestations: "write", contents: "write" })), /job attest must hold exactly/);
  assert.match(problemsAfter((copy) => delete copy.jobs["stage-github"].environment), /job stage-github must enter the release environment/);
  assert.match(problemsAfter((copy) => delete copy.jobs["publish-pypi"].environment), /job publish-pypi must enter the release environment/);
  assert.match(problemsAfter((copy) => (copy.jobs.probe.environment = "release")), /job probe must not enter an environment/);
  assert.match(
    problemsAfter((copy) => copy.jobs["publish-pypi"].steps.push({ run: "x", env: { TOKEN: "${{ secrets.PYPI_API_TOKEN }}" } })),
    /job publish-pypi must not read secrets PYPI_API_TOKEN/,
  );
  assert.match(problemsAfter((copy) => (copy.jobs["stage-github"].needs = ["attest"])), /job stage-github must need plan directly/);
  assert.match(problemsAfter((copy) => (copy.jobs.attest.needs = ["plan", "build"])), /job attest must come after assemble/);
  assert.match(problemsAfter((copy) => (copy.jobs["publish-sideways"] = { "runs-on": "ubuntu-24.04", steps: [] })), /job publish-sideways is not a known release job/);
  assert.match(problemsAfter((copy) => delete copy.jobs.attest), /has no attest job/);
});

test("triggers are a v* tag push and a dispatch with an explicit commit and version", () => {
  assert.match(problemsAfter((copy) => (copy.on.pull_request = {})), /only from workflow_dispatch and a tag push/);
  assert.match(problemsAfter((copy) => (copy.on.push.branches = ["main"])), /exactly tags: \["v\*"\]/);
  assert.match(problemsAfter((copy) => (copy.on.push.tags = ["*"])), /exactly tags: \["v\*"\]/);
  assert.match(problemsAfter((copy) => (copy.on.workflow_dispatch.inputs.commit.required = false)), /explicit commit input/);
  assert.match(problemsAfter((copy) => delete copy.on.workflow_dispatch.inputs.version), /explicit version input/);
  assert.match(problemsAfter((copy) => (copy.permissions = { contents: "write" })), /top-level permissions must be exactly contents: read/);
  assert.match(problemsAfter((copy) => (copy.concurrency["cancel-in-progress"] = true)), /never cancels a run in progress/);
});

test("checkouts, caches and artifact retention follow the release rules", () => {
  assert.match(problemsAfter((copy) => (copy.jobs.validate.steps[0].with["persist-credentials"] = true)), /job validate step 1: checkout must set persist-credentials: false/);
  assert.match(problemsAfter((copy) => copy.jobs.build.steps.push({ uses: "actions/cache@0123456789012345678901234567890123456789" })), /must not use a reusable cache/);
  assert.match(
    problemsAfter((copy) => {
      const step = copy.jobs.validate.steps.find((candidate) => String(candidate.uses).startsWith("astral-sh/setup-uv@"));
      step.with["enable-cache"] = true;
    }),
    /must not use a reusable cache|setup-uv must set enable-cache: false/,
  );
  assert.match(
    problemsAfter((copy) => {
      const step = copy.jobs.build.steps.find((candidate) => String(candidate.uses).startsWith("actions/upload-artifact@"));
      step.with["retention-days"] = 1;
    }),
    /retained at least 14 days/,
  );
  assert.match(
    problemsAfter((copy) => {
      const step = copy.jobs.assemble.steps.find((candidate) => String(candidate.uses).startsWith("actions/upload-artifact@"));
      delete step.with["if-no-files-found"];
    }),
    /fail when there is nothing to upload/,
  );
  assert.match(problemsAfter((copy) => (copy.jobs.build.strategy.matrix = { include: [{ id: "archive-linux-x86_64" }] })), /job build must take its matrix from the plan/);
  assert.match(problemsAfter((copy) => (copy.jobs.assemble.needs = ["plan", "validate", "crates"])), /assemble must come after gate/);
  assert.match(checkReleaseWorkflow(workflow(), `${TEXT}\non:\n  pull_request_target: {}\n`).join("\n"), /pull_request_target/);
});

test("secrets and the workflow token are found however an expression spells them", () => {
  assert.deepEqual(secretReferences("${{ secrets.CARGO_REGISTRY_TOKEN }}"), ["CARGO_REGISTRY_TOKEN"]);
  assert.deepEqual(secretReferences("${{ secrets['PYPI_TOKEN'] }}"), ["PYPI_TOKEN"]);
  assert.deepEqual(secretReferences("${{ toJSON(secrets) }}"), ["*"]);
  assert.deepEqual(secretReferences("${{ secrets[format('{0}', 'X')] }}"), ["*"]);
  assert.deepEqual(secretReferences("no expression mentions secrets here"), []);
  assert.match(problemsAfter((copy) => copy.jobs.build.steps.push({ run: "x", env: { ALL: "${{ toJSON(secrets) }}" } })), /job build runs in rehearsals and must not read secrets \(\*\)/);
  assert.match(problemsAfter((copy) => copy.jobs.validate.steps.push({ run: "x", env: { T: "${{ secrets['CARGO_REGISTRY_TOKEN'] }}" } })), /job validate runs in rehearsals and must not read secrets/);
  assert.match(problemsAfter((copy) => copy.jobs.benchmark.steps.push({ run: "x", env: { T: "${{ github['token'] }}" } })), /job benchmark must not read the workflow token/);
  assert.match(problemsAfter((copy) => (copy.env.CARGO_REGISTRY_TOKEN = "${{ secrets.CARGO_REGISTRY_TOKEN }}")), /workflow-level env must not carry a secret/);
  assert.match(problemsAfter((copy) => (copy.env.GH_TOKEN = "${{ github.token }}")), /workflow-level env must not carry a secret or the workflow token/);
});

test("the publish condition and order come only from the plan and the plan's job graph", () => {
  assert.match(problemsAfter((copy) => (copy.jobs.plan.outputs.publish = "true")), /plan output publish must come from the tested plan step/);
  assert.match(problemsAfter((copy) => (copy.jobs.plan.outputs.matrix = '{"include":[]}')), /plan output matrix must come from the tested plan step/);
  assert.match(problemsAfter((copy) => (copy.jobs["publish-crates"].needs = ["plan", "assemble"])), /job publish-crates must need exactly plan, stage-github/);
  assert.match(problemsAfter((copy) => (copy.jobs["publish-github"].needs = ["plan", "stage-github"])), /job publish-github must need exactly plan, publish-crates, publish-pypi/);
  assert.match(problemsAfter((copy) => (copy.jobs.probe.permissions = { contents: "read" })), /job probe must hold exactly/);
});

test("no job hides a failure, no run expands an expression, and downloads are named", () => {
  assert.match(problemsAfter((copy) => (copy.jobs.gate["continue-on-error"] = true)), /job gate must not continue on error/);
  assert.match(problemsAfter((copy) => (copy.jobs.validate["continue-on-error"] = true)), /job validate must not continue on error/);
  assert.equal(problemsAfter((copy) => (copy.jobs.benchmark["continue-on-error"] = true)), "", "the informational benchmark may fail");
  assert.match(
    problemsAfter((copy) => (copy.jobs.gate.steps.find((step) => /gate-git/.test(step.run ?? ""))["continue-on-error"] = true)),
    /job gate step \d+: steps must not continue on error/,
  );
  assert.match(problemsAfter((copy) => copy.jobs.plan.steps.push({ run: "echo ${{ inputs.version }}" })), /run must not expand \$\{\{ \}\} expressions/);
  assert.match(
    problemsAfter((copy) => copy.jobs.assemble.steps.unshift({ uses: "actions/download-artifact@d3f86a106a0bac45b974a628896c90dbdf5c8093", with: { path: "x" } })),
    /download artifacts by name or pattern/,
  );
});

test("the crates.io token is never exported under Cargo's name", () => {
  assert.match(
    problemsAfter((copy) => {
      const step = copy.jobs["publish-crates"].steps.find((candidate) => candidate.env?.RELEASE_CRATES_TOKEN);
      step.env = { CARGO_REGISTRY_TOKEN: step.env.RELEASE_CRATES_TOKEN };
    }),
    /exports CARGO_REGISTRY_TOKEN/,
  );
  assert.match(problemsAfter((copy) => (copy.jobs["publish-crates"].env = { CARGO_REGISTRY_TOKEN: "${{ secrets.CARGO_REGISTRY_TOKEN }}" })), /job publish-crates exports CARGO_REGISTRY_TOKEN/);
  // Quoting a key changes nothing: the gate reads the parsed YAML, not the text.
  const quoted = TEXT.replace(
    "          RELEASE_CRATES_TOKEN: ${{ secrets.CARGO_REGISTRY_TOKEN }}",
    "          RELEASE_CRATES_TOKEN: ${{ secrets.CARGO_REGISTRY_TOKEN }}\n          'CARGO_REGISTRY_TOKEN': ${{ secrets.CARGO_REGISTRY_TOKEN }}",
  );
  assert.notEqual(quoted, TEXT);
  assert.match(checkReleaseWorkflow(parse(quoted), quoted).join("\n"), /exports CARGO_REGISTRY_TOKEN/);
  assert.match(
    problemsAfter((copy) => copy.jobs["publish-crates"].steps.splice(-1, 0, { run: 'echo "CARGO_REGISTRY_TOKEN=$RELEASE_CRATES_TOKEN" >> "$GITHUB_ENV"', env: { RELEASE_CRATES_TOKEN: "${{ secrets.CARGO_REGISTRY_TOKEN }}" } })),
    /must not write to \$GITHUB_ENV/,
  );
  assert.match(problemsAfter((copy) => copy.jobs["publish-crates"].steps.push({ run: "cargo build --release" })), /the job that holds the crates\.io token runs only/);
});

test("credential-named variables appear only where a job may hold them", () => {
  assert.match(problemsAfter((copy) => (copy.env.RELEASE_READ_TOKEN = "${{ github.token }}")), /the workflow must not set the credential RELEASE_READ_TOKEN/);
  assert.match(problemsAfter((copy) => copy.jobs.build.steps.push({ run: "x", env: { RELEASE_CRATES_TOKEN: "x" } })), /job build step \d+ must not set the credential RELEASE_CRATES_TOKEN/);
  assert.match(problemsAfter((copy) => copy.jobs.validate.steps.push({ run: "x", env: { GH_TOKEN: "x" } })), /job validate step \d+ must not set the credential GH_TOKEN/);
  assert.match(problemsAfter((copy) => (copy.jobs["publish-pypi"].env = { RELEASE_CRATES_TOKEN: "x" })), /job publish-pypi must not set the credential RELEASE_CRATES_TOKEN/);
});

test("the crates.io secret reaches only the publish step, as RELEASE_CRATES_TOKEN", () => {
  // Review C2: each of these hands the token to code other than the publish script.
  const SECRET = "${{ secrets.CARGO_REGISTRY_TOKEN }}";
  const publishStep = (copy) => copy.jobs["publish-crates"].steps.find((step) => step.run === PUBLISH_CRATES_RUN);
  const toolchainStep = (copy) => copy.jobs["publish-crates"].steps.find((step) => String(step.uses).startsWith("dtolnay/rust-toolchain@"));
  const cases = {
    "job-level env": [(copy) => {
      copy.jobs["publish-crates"].env = { RELEASE_CRATES_TOKEN: SECRET };
      delete publishStep(copy).env;
    }, /workflow\.jobs\.publish-crates\.env\.RELEASE_CRATES_TOKEN references secrets\.CARGO_REGISTRY_TOKEN/],
    "an action step's env": [(copy) => (toolchainStep(copy).env = { RELEASE_CRATES_TOKEN: SECRET }), /steps\[2\]\.env\.RELEASE_CRATES_TOKEN references secrets\.CARGO_REGISTRY_TOKEN/],
    "an action input": [(copy) => (toolchainStep(copy).with.token = SECRET), /steps\[2\]\.with\.token references secrets\.CARGO_REGISTRY_TOKEN/],
    "another env name on the publish step": [(copy) => (publishStep(copy).env.FOO = SECRET), /steps\[4\]\.env\.FOO references secrets\.CARGO_REGISTRY_TOKEN/],
    "another job": [(copy) => (copy.jobs["publish-pypi"].steps.at(-1).env = { ...copy.jobs["publish-pypi"].steps.at(-1).env, SPARE: SECRET }), /publish-pypi\.steps\[\d+\]\.env\.SPARE references secrets\.CARGO_REGISTRY_TOKEN/],
    "a bracketed reference": [(copy) => (toolchainStep(copy).with.token = "${{ secrets['CARGO_REGISTRY_TOKEN'] }}"), /with\.token references secrets\.CARGO_REGISTRY_TOKEN/],
    "a second value on the publish step": [(copy) => (publishStep(copy).env.EXTRA = "x"), /the publish-crates step's env may hold only RELEASE_CRATES_TOKEN, found EXTRA/],
    "an env on another step": [(copy) => (toolchainStep(copy).env = { RUSTUP_TOOLCHAIN: "1.98.0" }), /job publish-crates step 3: only the publish step may set env/],
    "a job-level env without the secret": [(copy) => (copy.jobs["publish-crates"].env = { CI: "true" }), /job publish-crates must not set a job-level env/],
    "a changed value": [(copy) => (publishStep(copy).env.RELEASE_CRATES_TOKEN = "${{ secrets.OTHER }}"), /must receive the crates\.io secret exactly as RELEASE_CRATES_TOKEN/],
  };
  for (const [label, [mutate, message]] of Object.entries(cases)) {
    assert.match(problemsAfter(mutate), message, label);
  }
  assert.deepEqual(crateSecretProblems(parse(TEXT), parse(TEXT).jobs), [], "the workflow as committed passes");
});
