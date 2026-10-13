import assert from "node:assert/strict";
import { chmodSync, mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import test from "node:test";

import { HOME_CANARY_TOKEN } from "./golden-env.mjs";
import {
  SMOKE_CASES,
  checkFixtureReport,
  commandNames,
  findPython,
  fixtureCaseDir,
  isolatedUvEnv,
  reportArgs,
  resolveOnPath,
  smokeInstalled,
  smokePath,
} from "./release-smoke.mjs";
import { fixtureReportFor } from "./release-test-binaries.mjs";

const reportFor = fixtureReportFor;

function scratch() {
  return mkdtempSync(path.join(tmpdir(), "urollup-smoke-test-"));
}

test("the smoke report is the plan's explicit-source JSON report", () => {
  assert.deepEqual(reportArgs("/fixtures/case"), ["report", "--source", "/fixtures/case", "--no-default-sources", "--format", "json", "--timezone", "UTC"]);
  assert.deepEqual(SMOKE_CASES.map((fixtureCase) => fixtureCase.split("/")[0]).sort(), ["claude-project", "codex-rollout"]);
});

test("a report that matches the case passes, and any other outcome fails", () => {
  for (const fixtureCase of SMOKE_CASES) {
    const stdout = JSON.stringify(reportFor(fixtureCase));
    assert.deepEqual(checkFixtureReport(fixtureCase, { status: 0, stdout, stderr: "" }), [], fixtureCase);
  }
  const fixtureCase = SMOKE_CASES[0];
  const good = reportFor(fixtureCase);
  const wrong = { ...good, totals: { ...good.totals, requests: { owned: 1, ambiguous: 0, unknown: 0 } } };
  assert.match(checkFixtureReport(fixtureCase, { status: 0, stdout: JSON.stringify(wrong), stderr: "" }).join("\n"), /disagrees with expected\.json/);
  assert.match(checkFixtureReport(fixtureCase, { status: 3, stdout: "", stderr: "coverage incomplete" }).join("\n"), /exited 3/);
  assert.match(checkFixtureReport(fixtureCase, { status: 0, stdout: "not json", stderr: "" }).join("\n"), /did not print JSON/);
  assert.match(checkFixtureReport(fixtureCase, { status: 0, stdout: JSON.stringify(good), stderr: "warning\n" }).join("\n"), /wrote to stderr/);
  assert.deepEqual(checkFixtureReport(fixtureCase, { status: 0, stdout: JSON.stringify(good), stderr: "Installed 1 package\n" }, { allowStderr: true }), []);
  assert.match(checkFixtureReport(fixtureCase, { status: 0, stdout: `${JSON.stringify(good)}\n${HOME_CANARY_TOKEN}`, stderr: "" }).join("\n"), /canary/);
});

test("PATH resolution finds the first executable, and smoke PATH hides every other urollup", { skip: process.platform === "win32" }, () => {
  const dir = scratch();
  try {
    for (const name of ["install", "elsewhere", "tools"]) {
      mkdirSync(path.join(dir, name));
    }
    writeFileSync(path.join(dir, "install", "urollup"), "#!/bin/sh\n");
    chmodSync(path.join(dir, "install", "urollup"), 0o755);
    writeFileSync(path.join(dir, "elsewhere", "urollup"), "#!/bin/sh\n");
    chmodSync(path.join(dir, "elsewhere", "urollup"), 0o755);
    writeFileSync(path.join(dir, "tools", "urollup"), "not executable");
    const base = [path.join(dir, "elsewhere"), path.join(dir, "tools"), "/usr/bin"].join(":");
    assert.equal(resolveOnPath("urollup", base, "linux"), path.join(dir, "elsewhere", "urollup"));
    const smoke = smokePath([path.join(dir, "install")], base, "linux");
    assert.deepEqual(smoke.split(":"), [path.join(dir, "install"), path.join(dir, "tools"), "/usr/bin"]);
    assert.equal(resolveOnPath("urollup", smoke, "linux"), path.join(dir, "install", "urollup"));
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("Windows command names follow PATHEXT", () => {
  assert.deepEqual(commandNames("urollup", "win32", ".EXE;.CMD"), ["urollup.exe", "urollup.cmd"]);
  assert.deepEqual(commandNames("urollup", "linux"), ["urollup"]);
});

test("an installed executable is smoke-tested only through PATH and hermetically", { skip: process.platform === "win32" }, () => {
  const dir = scratch();
  try {
    const install = path.join(dir, "install");
    mkdirSync(install);
    const reports = Object.fromEntries(SMOKE_CASES.map((fixtureCase) => [fixtureCaseDir(fixtureCase), reportFor(fixtureCase)]));
    const fake = [
      `#!${process.execPath}`,
      `const reports = ${JSON.stringify(reports)};`,
      "const args = process.argv.slice(2);",
      'if (args[0] === "--version") { process.stdout.write("urollup 0.1.0\\n"); process.exit(0); }',
      'if (process.env.HOME === undefined || !process.env.HOME.includes("urollup-golden-")) { process.stderr.write("not hermetic\\n"); process.exit(9); }',
      'if (!args.includes("--no-default-sources")) { process.exit(8); }',
      'process.stdout.write(JSON.stringify(reports[args[args.indexOf("--source") + 1]]));',
    ].join("\n");
    writeFileSync(path.join(install, "urollup"), fake);
    chmodSync(path.join(install, "urollup"), 0o755);
    const passed = smokeInstalled({ expected: path.join(install, "urollup"), installDirs: [install], version: "0.1.0", label: "fake" });
    assert.deepEqual(passed.problems, []);
    assert.deepEqual(passed.checks.map((check) => check.check), ["path-resolution", "version", "fixture-report", "fixture-report"]);
    const wrongVersion = smokeInstalled({ expected: path.join(install, "urollup"), installDirs: [install], version: "0.2.0", label: "fake" });
    assert.match(wrongVersion.problems.join("\n"), /expected "urollup 0\.2\.0\\n"/);
    const other = path.join(dir, "other");
    mkdirSync(other);
    writeFileSync(path.join(other, "urollup"), fake);
    chmodSync(path.join(other, "urollup"), 0o755);
    const shadowed = smokeInstalled({ expected: path.join(install, "urollup"), installDirs: [other, install], version: "0.1.0", label: "fake" });
    assert.match(shadowed.problems.join("\n"), /resolves to .*other\/urollup, not the installed/);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("uv runs isolated from user configuration, caches and managed Python downloads", () => {
  const env = isolatedUvEnv("/w", { PATH: "/usr/bin", UV_INDEX_URL: "https://mirror.invalid/simple", UV_CACHE_DIR: "/home/u/.cache/uv" });
  assert.equal(env.UV_INDEX_URL, undefined);
  assert.equal(env.UV_CACHE_DIR, path.join("/w", "uv-cache"));
  assert.equal(env.UV_TOOL_BIN_DIR, path.join("/w", "uv-bin"));
  assert.equal(env.UV_PYTHON_DOWNLOADS, "never");
  assert.equal(env.UV_NO_CONFIG, "1");
  assert.equal(env.PATH, "/usr/bin");
  assert.equal(findPython({ PATH: "" }, "linux"), null);
});
