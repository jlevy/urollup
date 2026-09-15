// Adapted from fdu `scripts/check-supply-chain.test.mjs` at afbb2ee; see PROVENANCE.md.
import assert from "node:assert/strict";
import test from "node:test";

import {
  assertAged,
  assertNoExpiredExceptions,
  assertEqualProvenance,
  fetchWithRetry,
  isNestedWorkingCopy,
  parseActionUses,
  parseCargoLock,
  parseUvLock,
  selectGithubToken,
  validateBootstrapPin,
  validateDownloadScripts,
  validateExceptions,
  validateFirstParty,
  validatePolicy,
  validateWorkflowSecurity,
} from "./check-supply-chain.mjs";

const NOW = new Date("2026-08-09T12:00:00Z");

test("fresh and undated releases fail closed", () => {
  assert.throws(
    () =>
      assertAged(
        { ecosystem: "cargo", name: "example", version: "1.0.0", publishedAt: "2026-08-01T00:00:00Z" },
        NOW,
        14,
        [],
      ),
    /has not cleared the 14-day cool-off/,
  );
  assert.throws(
    () =>
      assertAged(
        { ecosystem: "npm", name: "example", version: "1.0.0", publishedAt: undefined },
        NOW,
        14,
        [],
      ),
    /missing a valid publication time/,
  );
});

test("only a complete, matching, unexpired exception bypasses release age", () => {
  const exception = {
    id: "first-party-example",
    ecosystem: "npm",
    name: "example",
    version: "1.0.0",
    reason: "First-party tool pinned to the reviewed release.",
    approvedBy: "Repository maintainers in AGENTS.md",
    approvedAt: "2026-08-01T00:00:00Z",
    expiresAt: "2026-08-15T00:00:00Z",
    followUp: "Remove the exception after the release clears the normal cool-off.",
  };

  validateExceptions([exception]);
  assert.doesNotThrow(() =>
    assertAged(
      { ecosystem: "npm", name: "example", version: "1.0.0", publishedAt: "2026-08-01T00:00:00Z" },
      NOW,
      14,
      [exception],
    ),
  );
  assert.throws(() => validateExceptions([{ ...exception, followUp: "" }]), /followUp/);
  assert.throws(
    () =>
      assertAged(
        { ecosystem: "npm", name: "example", version: "2.0.0", publishedAt: "2026-08-01T00:00:00Z" },
        NOW,
        14,
        [exception],
      ),
    /has not cleared/,
  );
});

test("provenance mismatch is never an age exception", () => {
  assert.throws(
    () => assertEqualProvenance("crate checksum", "sha256:expected", "sha256:actual"),
    /crate checksum mismatch/,
  );
});

test("provenance requests retry only transient failures", async () => {
  const statuses = [503, 429, 200];
  const attempts = [];
  const response = await fetchWithRetry(
    "https://example.invalid/provenance",
    {},
    async () => {
      const status = statuses.shift();
      attempts.push(status);
      return { ok: status === 200, status };
    },
    async () => {},
  );
  assert.equal(response.status, 200);
  assert.deepEqual(attempts, [503, 429, 200]);

  await assert.rejects(
    () =>
      fetchWithRetry(
        "https://example.invalid/missing",
        {},
        async () => ({ ok: false, status: 404 }),
        async () => {},
      ),
    /failed \(404\)/,
  );
});

test("GitHub provenance prefers explicit tokens and supports local gh credentials", () => {
  assert.equal(
    selectGithubToken({ GITHUB_TOKEN: " workflow-token ", GH_TOKEN: "gh-token" }, "local-token"),
    "workflow-token",
  );
  assert.equal(selectGithubToken({ GH_TOKEN: " gh-token " }, "local-token"), "gh-token");
  assert.equal(selectGithubToken({}, " local-token\n"), "local-token");
  assert.equal(selectGithubToken({}, ""), undefined);
});

test("every shell downloader is explicitly inventoried", () => {
  const files = [
    { path: "scripts/reviewed.sh", text: "curl https://example.invalid/tool" },
    { path: "scripts/local.sh", text: "cargo test" },
  ];
  assert.doesNotThrow(() => validateDownloadScripts(files, new Set(["scripts/reviewed.sh"])));
  assert.throws(() => validateDownloadScripts(files, new Set()), /absent from the bootstrap inventory/);
});

test("GitHub actions must use an exact commit", () => {
  assert.throws(
    () => parseActionUses([{ path: ".github/workflows/ci.yml", text: "- uses: actions/checkout@v6\n" }]),
    /must pin an exact 40-character commit/,
  );
  assert.deepEqual(
    parseActionUses([
      {
        path: ".github/workflows/ci.yml",
        text: "- uses: actions/checkout@d23441a48e516b6c34aea4fa41551a30e30af803\n",
      },
    ]),
    [
      {
        path: ".github/workflows/ci.yml",
        repository: "actions/checkout",
        revision: "d23441a48e516b6c34aea4fa41551a30e30af803",
      },
    ],
  );
});

test("pull-request workflows use least privilege and gate all executable jobs", () => {
  const revision = "d23441a48e516b6c34aea4fa41551a30e30af803";
  const valid = `name: CI
on:
  pull_request:
permissions:
  contents: read
jobs:
  supply-chain:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@${revision}
        with:
          persist-credentials: false
      - run: node --test scripts/check-supply-chain.test.mjs
      - run: node scripts/check-supply-chain.mjs
        env:
          GITHUB_TOKEN: \${{ github.token }}
  test:
    needs: supply-chain
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@${revision}
        with:
          persist-credentials: false
      - run: npm ci --ignore-scripts
`;
  assert.doesNotThrow(() =>
    validateWorkflowSecurity([{ path: ".github/workflows/ci.yml", text: valid }]),
  );
  assert.throws(
    () =>
      validateWorkflowSecurity([
        { path: ".github/workflows/ci.yml", text: valid.replace("contents: read", "contents: write") },
      ]),
    /grant only contents: read/,
  );
  assert.throws(
    () =>
      validateWorkflowSecurity([
        {
          path: ".github/workflows/ci.yml",
          text: valid.replace("persist-credentials: false", "persist-credentials: true"),
        },
      ]),
    /persist-credentials: false/,
  );
  assert.throws(
    () =>
      validateWorkflowSecurity([
        { path: ".github/workflows/ci.yml", text: valid.replace("npm ci --ignore-scripts", "npm ci\n        cache: npm") },
      ]),
    /cache writes are forbidden/,
  );
  assert.throws(
    () =>
      validateWorkflowSecurity([
        {
          path: ".github/workflows/ci.yml",
          text: valid.replace("          GITHUB_TOKEN: \${{ github.token }}\n", ""),
        },
      ]),
    /must authenticate with the least-privilege workflow token/,
  );
});

test("Cargo registry entries require checksums", () => {
  const lock = `version = 4

[[package]]
name = "example"
version = "1.0.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
`;
  assert.throws(() => parseCargoLock(lock), /example@1.0.0 is missing a checksum/);
});

test("uv registry artifacts require hashes and upload times", () => {
  const lock = `version = 1
revision = 3

[[package]]
name = "example"
version = "1.0.0"
source = { registry = "https://pypi.org/simple" }
wheels = [
    { url = "https://files.pythonhosted.org/example.whl", hash = "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa" },
]
`;
  assert.throws(() => parseUvLock(lock), /example@1.0.0 artifact is missing an upload time/);
});

const PIN = {
  name: "get-tbd",
  version: "0.7.1",
  versionSource: {
    file: ".tbd/config.yml",
    key: "tbd_fallback_version",
    variable: "configured_fallback_version",
  },
};
const CONFIG = "tbd_format: f08\ntbd_fallback_version: 0.7.1\n";

test("an inline pin is accepted", () => {
  validateBootstrapPin("hook.sh", 'npx --yes get-tbd@0.7.1 closing\n', { name: "get-tbd", version: "0.7.1" }, null);
});

test("indirection is accepted only when its source declares the pinned version", () => {
  const hook = 'npx --yes "get-tbd@$configured_fallback_version" closing\n';
  validateBootstrapPin("hook.sh", hook, PIN, CONFIG);
});

test("indirection whose source pins a different version fails closed", () => {
  // The whole risk of reading the version at run time: the file the hook trusts could
  // drift away from the reviewed pin without the hook itself changing at all.
  const hook = 'npx --yes "get-tbd@$configured_fallback_version" closing\n';
  assert.throws(
    () => validateBootstrapPin("hook.sh", hook, PIN, "tbd_fallback_version: 0.9.9\n"),
    /declares tbd_fallback_version 0\.9\.9, but the bootstrap pin is get-tbd@0\.7\.1/,
  );
});

test("indirection whose source omits the key fails closed", () => {
  const hook = 'npx --yes "get-tbd@$configured_fallback_version" closing\n';
  assert.throws(
    () => validateBootstrapPin("hook.sh", hook, PIN, "tbd_format: f08\n"),
    /does not declare tbd_fallback_version/,
  );
});

test("an npx line resolving through some other variable is unreviewed", () => {
  const hook = 'npx --yes "get-tbd@$SOMETHING_ELSE" closing\n';
  assert.throws(
    () => validateBootstrapPin("hook.sh", hook, PIN, CONFIG),
    /must use only the exact bootstrap get-tbd@0\.7\.1/,
  );
});

test("@latest is rejected even beside a correct pin", () => {
  const hook = 'npx --yes get-tbd@0.7.1 closing\nnpx --yes get-tbd@latest other\n';
  assert.throws(
    () => validateBootstrapPin("hook.sh", hook, PIN, CONFIG),
    /must not resolve get-tbd through @latest/,
  );
});

test("a second npx line escaping the pin is caught", () => {
  const hook = 'npx --yes "get-tbd@$configured_fallback_version" closing\nnpx --yes some-other-tool run\n';
  assert.throws(
    () => validateBootstrapPin("hook.sh", hook, PIN, CONFIG),
    /unreviewed npx execution/,
  );
});

const FIRST_PARTY = [
  { ecosystem: "npm", name: "get-tbd", reason: "first-party", approvedBy: "maintainers" },
];

test("a first-party package is exempt from the cool-off by identity, not version", () => {
  // The point of the list: a tbd patch released yesterday passes without anyone
  // authoring a dated exception for that exact version.
  assertAged(
    { ecosystem: "npm", name: "get-tbd", version: "9.9.9", publishedAt: "2026-08-09T00:00:00Z" },
    NOW,
    14,
    [],
    FIRST_PARTY,
  );
});

test("first-party status does not leak to other packages or ecosystems", () => {
  assert.throws(
    () =>
      assertAged(
        { ecosystem: "npm", name: "some-other-tool", version: "1.0.0", publishedAt: "2026-08-09T00:00:00Z" },
        NOW,
        14,
        [],
        FIRST_PARTY,
      ),
    /has not cleared the 14-day cool-off/,
  );
  assert.throws(
    () =>
      assertAged(
        { ecosystem: "cargo", name: "get-tbd", version: "1.0.0", publishedAt: "2026-08-09T00:00:00Z" },
        NOW,
        14,
        [],
        FIRST_PARTY,
      ),
    /has not cleared the 14-day cool-off/,
  );
});

test("first-party never waives a missing publication time", () => {
  // Age is the only thing this list may waive; absent provenance still fails closed.
  assert.throws(
    () => assertAged({ ecosystem: "npm", name: "get-tbd", version: "1.0.0" }, NOW, 14, [], FIRST_PARTY),
    /missing a valid publication time/,
  );
});

test("a first-party entry pinning a version is rejected", () => {
  assert.throws(
    () => validateFirstParty([{ ecosystem: "npm", name: "x", reason: "r", approvedBy: "m", version: "1.0.0" }]),
    /must not pin a version/,
  );
});

test("a first-party entry missing approval is rejected", () => {
  assert.throws(
    () => validateFirstParty([{ ecosystem: "npm", name: "x", reason: "r" }]),
    /firstParty x\.approvedBy must be a non-empty string/,
  );
});

const EXPIRED = {
  id: "expired-example",
  ecosystem: "cargo",
  name: "example",
  version: "1.0.0",
  reason: "Security patch inside the cool-off window.",
  approvedBy: "Repository maintainers",
  approvedAt: "2026-07-01T00:00:00Z",
  expiresAt: "2026-07-15T00:00:00Z",
  followUp: "Remove once example@1.0.0 clears the normal cool-off.",
};

test("an expired exception fails the policy even when no locked package uses it", () => {
  assert.throws(() => assertNoExpiredExceptions([EXPIRED], NOW), /expired-example expired/);
  assert.doesNotThrow(() =>
    assertNoExpiredExceptions([{ ...EXPIRED, expiresAt: "2026-09-01T00:00:00Z" }], NOW),
  );
  const policy = {
    schema: 1,
    minimumAgeDays: 14,
    exceptions: [EXPIRED],
    firstParty: [],
    bootstrap: { npm: [], githubReleases: [], rustToolchains: [], nodeRuntimes: [], identicalFileGroups: [] },
  };
  assert.throws(() => validatePolicy(policy, NOW), /expired-example expired/);
  assert.doesNotThrow(() => validatePolicy({ ...policy, exceptions: [] }, NOW));
  assert.throws(() => validatePolicy({ ...policy, exceptions: [], minimumAgeDays: 7 }, NOW), /at least 14 days/);
});

test("every workflow, not only pull-request ones, starts read-only", () => {
  const release = `name: Release
on:
  push:
    tags: ["v*"]
permissions:
  contents: write
jobs:
  publish:
    runs-on: ubuntu-latest
`;
  assert.throws(
    () => validateWorkflowSecurity([{ path: ".github/workflows/release.yml", text: release }]),
    /must grant only contents: read at the top level/,
  );
  const scoped = release.replace("permissions:\n  contents: write", "permissions:\n  contents: read").replace(
    "  publish:\n",
    "  publish:\n    permissions:\n      contents: write\n",
  );
  // Outside pull requests, a job may ask for the authority it needs in its own block.
  assert.doesNotThrow(() => validateWorkflowSecurity([{ path: ".github/workflows/release.yml", text: scoped }]));
  const blanket = scoped.replace("    permissions:\n      contents: write\n", "    permissions: write-all\n");
  assert.notEqual(blanket, scoped);
  assert.throws(
    () => validateWorkflowSecurity([{ path: ".github/workflows/release.yml", text: blanket }]),
    /blanket write-all or read-all permissions are forbidden/,
  );
});

test("pull-request jobs may not widen the read-only default", () => {
  const revision = "d23441a48e516b6c34aea4fa41551a30e30af803";
  const workflow = `name: CI
on:
  pull_request:
permissions:
  contents: read
jobs:
  supply-chain:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@${revision}
        with:
          persist-credentials: false
      - run: node --test scripts/check-supply-chain.test.mjs
      - run: node scripts/check-supply-chain.mjs
        env:
          GITHUB_TOKEN: \${{ github.token }}
  test:
    needs: supply-chain
    permissions:
      contents: read
    runs-on: ubuntu-latest
`;
  assert.doesNotThrow(() => validateWorkflowSecurity([{ path: ".github/workflows/ci.yml", text: workflow }]));
  assert.throws(
    () =>
      validateWorkflowSecurity([
        {
          path: ".github/workflows/ci.yml",
          text: workflow.replace("    permissions:\n      contents: read", "    permissions:\n      pull-requests: write"),
        },
      ]),
    /pull-request jobs must not grant write permissions/,
  );
});

test("nested working copies are left to their own gate", () => {
  // An agent worktree under .claude/worktrees/ holds a mid-edit copy of this repository.
  assert.equal(isNestedWorkingCopy([".agents", ".claude", ".git", "Cargo.toml"]), true);
  assert.equal(isNestedWorkingCopy(["hooks", "scripts", "settings.json"]), false);
});
