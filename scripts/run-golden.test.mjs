import assert from "node:assert/strict";
import test from "node:test";
import { mkdtempSync, mkdirSync, writeFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";

import { countProblem, goldenTreeIsDirty, parseArgs, parseSummary, spawnTryscript } from "./run-golden.mjs";

test("runner arguments separate tryscript flags, their values, sessions and runner options", () => {
  const options = parseArgs(["--update", "--filter", "Report", "--allow-dirty", "tests/golden/e2e/claude-project/a.tryscript.md"]);
  assert.deepEqual(options.flags, ["--update", "--filter", "Report"]);
  assert.deepEqual(options.files, ["tests/golden/e2e/claude-project/a.tryscript.md"]);
  assert.equal(options.allowDirty, true);
  assert.equal(options.rewrites, true);
  assert.equal(options.filtered, true);
  assert.deepEqual(parseArgs([]), { flags: [], files: [], allowDirty: false, rewrites: false, filtered: false });
  assert.equal(parseArgs(["--expand"]).rewrites, true);
});

test("tryscript's summary line yields pass and fail counts, even with color codes", () => {
  assert.deepEqual(parseSummary("PASS x\n\n7 passed (98ms)\n"), { passed: 7, failed: 0 });
  assert.deepEqual(parseSummary("[32m5 passed[39m, [31m2 failed[39m (1.2s)\n"), { passed: 5, failed: 2 });
  assert.deepEqual(parseSummary("no tests run (3ms)\n"), { passed: 0, failed: 0 });
  assert.equal(parseSummary("tryscript crashed\n"), undefined);
});

test("a successful run must pass exactly the selected blocks", () => {
  assert.equal(countProblem({ status: 0, summary: { passed: 7, failed: 0 }, expectedBlocks: 7, filtered: false }), undefined);
  assert.match(countProblem({ status: 0, summary: { passed: 6, failed: 0 }, expectedBlocks: 7, filtered: false }), /passed 6 blocks, but the selected sessions hold 7/);
  assert.match(countProblem({ status: 0, summary: { passed: 0, failed: 0 }, expectedBlocks: 7, filtered: false }), /passed 0 blocks/);
  assert.match(countProblem({ status: 0, summary: undefined, expectedBlocks: 7, filtered: false }), /no pass or fail summary/);
  // A failing run already fails, and a filter selects fewer blocks on purpose.
  assert.equal(countProblem({ status: 1, summary: { passed: 6, failed: 1 }, expectedBlocks: 7, filtered: false }), undefined);
  assert.equal(countProblem({ status: 0, summary: { passed: 2, failed: 0 }, expectedBlocks: 7, filtered: true }), undefined);
});

test("the update guard reads git's unstaged golden changes and admits when it cannot tell", () => {
  const calls = [];
  const git = (status, error) => (command, args) => {
    calls.push([command, ...args]);
    return { status, error };
  };
  assert.equal(goldenTreeIsDirty("/repo", git(0)), false);
  assert.equal(goldenTreeIsDirty("/repo", git(1)), true);
  assert.equal(goldenTreeIsDirty("/repo", git(128)), undefined);
  assert.equal(goldenTreeIsDirty("/repo", git(null, new Error("ENOENT"))), undefined);
  assert.deepEqual(calls[0].slice(0, 4), ["git", "diff", "--quiet", "--"]);
});

// No .bin shell shim exists in this fixture: the locked JavaScript entrypoint must
// run directly, preserving both the executable path and each argument on Windows.
test("tryscript launches without interpreting spaces or shell metacharacters", async () => {
  const root = mkdtempSync(path.join(tmpdir(), "golden path & literal-"));
  try {
    const entrypoint = path.join(root, "node_modules", "tryscript", "dist", "bin.mjs");
    mkdirSync(path.dirname(entrypoint), { recursive: true });
    writeFileSync(entrypoint, "process.stdout.write(JSON.stringify(process.argv.slice(2)));\n");
    const args = ["run", path.join(root, "session with spaces & literal.tryscript.md")];
    const child = spawnTryscript(root, args, { stdio: ["ignore", "pipe", "pipe"] });
    let stdout = "";
    let stderr = "";
    child.stdout.on("data", (chunk) => { stdout += chunk; });
    child.stderr.on("data", (chunk) => { stderr += chunk; });
    const status = await new Promise((resolve, reject) => {
      child.on("error", reject);
      child.on("close", resolve);
    });
    assert.equal(status, 0, stderr);
    assert.deepEqual(JSON.parse(stdout), args);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});
