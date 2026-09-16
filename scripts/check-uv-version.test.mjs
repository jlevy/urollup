// Adapted from fdu `scripts/check-uv-version.test.mjs` at afbb2ee; see PROVENANCE.md.
import assert from "node:assert/strict";
import { chmodSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { spawnSync } from "node:child_process";
import test from "node:test";
import { fileURLToPath } from "node:url";

const ROOT = dirname(dirname(fileURLToPath(import.meta.url)));
const MINIMUM = "0.12.1";

function shellQuote(value) {
  return `'${value.replaceAll("'", `'"'"'`)}'`;
}

function runGuard(output, exitCode = 0) {
  const scratch = mkdtempSync(join(tmpdir(), "urollup-uv-version-"));
  const stub = join(scratch, "uv");
  try {
    writeFileSync(
      stub,
      `#!/bin/sh\nprintf '%s\\n' ${shellQuote(output)}\nexit ${exitCode}\n`,
      "utf8",
    );
    chmodSync(stub, 0o755);
    return spawnSync("make", ["--no-print-directory", "uv-version", `UV=${stub}`], {
      cwd: ROOT,
      encoding: "utf8",
    });
  } finally {
    rmSync(scratch, { recursive: true, force: true });
  }
}

test("uv guard accepts only successful stable versions at or above the reviewed floor", () => {
  const missing = spawnSync(
    "make",
    ["--no-print-directory", "uv-version", "UV=/definitely/missing/urollup-test-uv"],
    { cwd: ROOT, encoding: "utf8" },
  );
  assert.notEqual(missing.status, 0);
  assert.match(missing.stdout, /uv is not installed/);
  assert.match(missing.stdout, new RegExp(`reviewed ${MINIMUM} release`));

  for (const version of [MINIMUM, "0.12.4", "1.0.0"]) {
    const result = runGuard(`uv ${version}`);
    assert.equal(result.status, 0, `${version}: ${result.stdout}${result.stderr}`);
  }

  for (const version of ["0.11.9", "0.11.27"]) {
    const result = runGuard(`uv ${version}`);
    assert.notEqual(result.status, 0, version);
    assert.match(result.stdout, new RegExp(`needs uv >= ${MINIMUM}`));
    // The pinned installer is the remedy that works regardless of how uv was
    // installed; `uv self update` is offered second because it fails outright when
    // an external manager owns the binary.
    assert.match(result.stdout, new RegExp(`astral\\.sh/uv/${MINIMUM}/install\\.sh`));
    assert.match(result.stdout, new RegExp(`uv self update ${MINIMUM}`));
  }

  for (const output of ["uv 0.12.1rc1", "uv 0.12.1.dev1", "uv malformed", "not-uv 1.0.0"]) {
    const result = runGuard(output);
    assert.notEqual(result.status, 0, output);
    assert.match(result.stdout, /could not determine a stable uv version/);
  }

  const failed = runGuard("uv 1.0.0", 42);
  assert.notEqual(failed.status, 0);
  assert.match(failed.stdout, /could not run uv --version/);
});

test("every directly uv-backed Make target depends on the version guard", () => {
  const makefile = readFileSync(join(ROOT, "Makefile"), "utf8");
  const uvBackedTargets = new Set();
  let currentTargets = [];
  for (const line of makefile.split("\n")) {
    if (!line.startsWith("\t")) {
      const match = line.match(/^([A-Za-z0-9_.-]+(?:\s+[A-Za-z0-9_.-]+)*):/);
      currentTargets = match ? match[1].split(/\s+/) : [];
      continue;
    }
    // A `uv` or `uvx` command word, not a substring such as `check-uv-version.test.mjs`.
    if (/(?:^|[\s;&|(])uvx?(?=\s)|\$\((?:UV|UV_RUN|FLOWMARK)\)/.test(line)) {
      for (const target of currentTargets) {
        if (target !== "uv-version") uvBackedTargets.add(target);
      }
    }
  }

  const database = spawnSync("make", ["-qp"], { cwd: ROOT, encoding: "utf8" }).stdout;
  assert(uvBackedTargets.size > 0);
  for (const target of uvBackedTargets) {
    const rule = database.split("\n").find((line) => line.startsWith(`${target}:`));
    assert.match(rule ?? "", /(?:^|\s)uv-version(?:\s|$)/, `${target}: ${rule}`);
  }
});

test("the bootstrap policy enforces one reviewed uv version in Make and CI", () => {
  const policy = JSON.parse(readFileSync(join(ROOT, "supply-chain-policy.json"), "utf8"));
  const uvRelease = policy.bootstrap.githubReleases.find(
    (release) => release.repository === "astral-sh/uv",
  );
  assert(uvRelease);
  assert.deepEqual(
    new Set(uvRelease.files),
    new Set([".github/workflows/ci.yml", "Makefile"]),
  );

  const makefile = readFileSync(join(ROOT, "Makefile"), "utf8");
  const makeVersion = makefile.match(/^UV_MIN_VERSION := (\S+)$/m)?.[1];
  assert.equal(makeVersion, uvRelease.version);
  assert.equal(uvRelease.tag, uvRelease.version);

  const workflowFiles = uvRelease.files.filter((file) => file.endsWith(".yml"));
  const workflowVersions = workflowFiles.flatMap((file) => {
    const workflow = readFileSync(join(ROOT, file), "utf8");
    const versions = [...workflow.matchAll(
      /uses: astral-sh\/setup-uv@[^\n]+\n\s+with:\n\s+version: "([^"]+)"/g,
    )].map((match) => match[1]);
    assert(versions.length > 0, `${file} inventories uv but has no setup-uv pin`);
    return versions;
  });
  assert.deepEqual(new Set(workflowVersions), new Set([uvRelease.version]));
});
