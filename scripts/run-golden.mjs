#!/usr/bin/env node
// Run the tryscript CLI golden corpus against the built binary, hermetically, and prove
// which binary ran and that every block ran.
//
// Adapted from fdu `scripts/run-golden.mjs` at afbb2ee; see PROVENANCE.md. urollup has one
// surface (the Rust binary), honors CARGO_TARGET_DIR, and refuses an empty corpus.
//
// Sessions invoke a bare `urollup`, resolved through `path: [$UROLLUP_BIN]` front matter,
// because a bare command name is the only form /bin/sh and cmd.exe read the same way.
// tryscript's `path:` prepends to PATH, so this runner preflights the binary first: a
// missing build must be a diagnosed failure, not a pass against an installed urollup
// (check-golden-invocations.mjs keeps every session on $UROLLUP_BIN).
//
// Beyond the fdu original:
// - tryscript runs with the hermetic environment from golden-env.mjs, never the invoking
//   shell's, so no agent session variable or real HOME reaches urollup, and a run that
//   writes into the golden HOME fails;
// - on success the number of passed blocks must equal the console blocks in the selected
//   sessions, so a block tryscript dropped cannot pass as a smaller corpus;
// - `--update` and `--expand` refuse to run over unstaged golden changes, so the diff to
//   review afterwards is exactly what the update wrote; and
// - sessions may be named on the command line to update or expand them narrowly.
//
// Usage: node scripts/run-golden.mjs [--update | --expand ...] [--allow-dirty] [session.tryscript.md ...]

import { spawn, spawnSync } from "node:child_process";
import { accessSync, constants, readFileSync, statSync } from "node:fs";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

import { findSessions, parseBlocks } from "./check-golden-invocations.mjs";
import { openGoldenEnvironment } from "./golden-env.mjs";

const ROOT = path.dirname(path.dirname(fileURLToPath(import.meta.url)));
const CORPUS = path.join("tests", "golden");
export const FIXTURES_ROOT = path.join("crates", "urollup-core", "tests", "fixtures");
const REWRITING_FLAGS = new Set(["--update", "--expand", "--expand-generic", "--expand-all"]);

/** Split runner arguments into tryscript flags, session files and runner options. */
export function parseArgs(argv) {
  const options = { flags: [], files: [], allowDirty: false, rewrites: false, filtered: false };
  for (let index = 0; index < argv.length; index += 1) {
    const argument = argv[index];
    if (argument === "--allow-dirty") {
      options.allowDirty = true;
    } else if (argument.startsWith("-")) {
      options.flags.push(argument);
      if (REWRITING_FLAGS.has(argument)) {
        options.rewrites = true;
      }
      // Options that take a value keep it with them.
      if ((argument === "--filter" || argument === "--capture-log") && argv[index + 1] !== undefined) {
        options.flags.push(argv[index + 1]);
        index += 1;
      }
      if (argument === "--filter" || argument.startsWith("--filter=")) {
        options.filtered = true;
      }
    } else {
      options.files.push(argument);
    }
  }
  return options;
}

/** The `N passed` and `N failed` counts from tryscript's final stdout line, if any. */
export function parseSummary(stdout) {
  // Strip color codes in case a terminal forced them on.
  const text = stdout.replace(/\[[0-9;]*m/g, "");
  const line = text.trimEnd().split(/\r?\n/).pop() ?? "";
  if (/^no tests run\b/.test(line)) {
    return { passed: 0, failed: 0 };
  }
  const passed = /(\d+) passed/.exec(line);
  const failed = /(\d+) failed/.exec(line);
  if (!passed && !failed) {
    return undefined;
  }
  return { passed: Number(passed?.[1] ?? 0), failed: Number(failed?.[1] ?? 0) };
}

/** A reason to reject a run whose counts do not show every selected block passing. */
export function countProblem({ status, summary, expectedBlocks, filtered }) {
  if (status !== 0 || filtered) {
    return undefined;
  }
  if (summary === undefined) {
    return "tryscript printed no pass or fail summary, so nothing proves the blocks ran";
  }
  if (summary.passed !== expectedBlocks) {
    return `tryscript passed ${summary.passed} blocks, but the selected sessions hold ${expectedBlocks}; a block was skipped, focused or not parsed`;
  }
  return undefined;
}

/** Whether tracked golden files have unstaged changes, or `undefined` outside a git work tree. */
export function goldenTreeIsDirty(root, run = spawnSync) {
  const result = run("git", ["diff", "--quiet", "--", CORPUS], { cwd: root, encoding: "utf8" });
  if (result.error || (result.status !== 0 && result.status !== 1)) {
    return undefined;
  }
  return result.status === 1;
}

function preflightBinary() {
  const exe = process.platform === "win32" ? ".exe" : "";
  const targetDir = process.env.CARGO_TARGET_DIR ? path.resolve(ROOT, process.env.CARGO_TARGET_DIR) : path.join(ROOT, "target");
  const binary = path.join(targetDir, "debug", `urollup${exe}`);
  try {
    if (!statSync(binary).isFile()) {
      throw new Error("not a regular file");
    }
    if (process.platform !== "win32") {
      accessSync(binary, constants.X_OK);
    }
  } catch (error) {
    console.error(`run-golden: the urollup binary is not runnable at ${binary}`);
    console.error(`run-golden: ${error.message}`);
    console.error("run-golden: build it with `make build`");
    process.exit(2);
  }
  return binary;
}

async function main() {
  const options = parseArgs(process.argv.slice(2));
  const binary = preflightBinary();

  // A corpus that matches nothing would let tryscript report success for zero sessions,
  // which is indistinguishable from a passing run. A missing golden fails here instead.
  const sessions = options.files.length > 0 ? options.files.map((file) => path.relative(ROOT, path.resolve(file)).split(path.sep).join("/")) : findSessions(ROOT);
  if (sessions.length === 0) {
    console.error(`run-golden: no *.tryscript.md sessions in ${CORPUS}; the golden corpus is missing`);
    process.exit(1);
  }
  const expectedBlocks = sessions.reduce((sum, file) => sum + parseBlocks(readFileSync(path.join(ROOT, file), "utf8").split(/\r?\n/)).length, 0);

  if (options.rewrites && !options.allowDirty) {
    const dirty = goldenTreeIsDirty(ROOT);
    if (dirty) {
      console.error(`run-golden: ${CORPUS} has unstaged changes; stage or commit them first, so the diff after this`);
      console.error("run-golden: rewrite is exactly what tryscript wrote (or pass --allow-dirty)");
      // Distinct from tryscript's 1 for "updated a failing block", which make golden-update tolerates.
      process.exit(3);
    }
    if (dirty === undefined) {
      console.error("run-golden: not a git work tree, so review every rewritten block by hand");
    }
  }

  const golden = openGoldenEnvironment({
    extra: {
      UROLLUP_BIN: path.dirname(binary),
      GOLDEN_FIXTURES: path.join(ROOT, FIXTURES_ROOT),
      GOLDEN_SAMPLES: path.join(ROOT, CORPUS, "samples"),
    },
  });
  let status;
  let stdout = "";
  try {
    console.log(`run-golden: ${sessions.length} sessions, ${expectedBlocks} blocks, against ${binary}`);
    console.log(`run-golden: hermetic environment with HOME at ${golden.dirs.home}`);
    // Resolved from the locked tree rather than from PATH, so the harness cannot run a
    // tryscript nobody pinned. Run from tests/golden so its tryscript.config.mjs applies.
    const tryscript = path.join(ROOT, "node_modules", ".bin", `tryscript${process.platform === "win32" ? ".cmd" : ""}`);
    // Forward slashes even on Windows: these are globs for tryscript, not paths for the OS.
    const files = sessions.map((file) => path.join(ROOT, file).split(path.sep).join("/"));
    const child = spawn(tryscript, ["run", ...options.flags, ...files], {
      cwd: path.join(ROOT, CORPUS),
      stdio: ["ignore", "pipe", "inherit"],
      shell: process.platform === "win32",
      env: golden.env,
    });
    child.stdout.on("data", (chunk) => {
      stdout += chunk;
      process.stdout.write(chunk);
    });
    status = await new Promise((resolve) => {
      child.on("error", (error) => {
        console.error(`run-golden: could not run ${tryscript}: ${error.message}`);
        console.error("run-golden: install the locked Node tools with `npm ci --ignore-scripts`");
        resolve(1);
      });
      child.on("close", (code) => resolve(code ?? 1));
    });

    const writes = golden.homeWrites();
    if (writes.length > 0) {
      console.error("run-golden: a golden run wrote into the hermetic HOME, where urollup must never write:");
      for (const write of writes) {
        console.error(`  ${write}`);
      }
      status = 1;
    }
  } finally {
    golden.cleanup();
  }

  const problem = countProblem({ status, summary: parseSummary(stdout), expectedBlocks, filtered: options.filtered || options.rewrites });
  if (problem) {
    console.error(`run-golden: ${problem}`);
    status = 1;
  }
  if (options.rewrites) {
    const diff = spawnSync("git", ["diff", "--stat", "--", CORPUS], { cwd: ROOT, encoding: "utf8" });
    console.log(`run-golden: review every rewritten line before committing:\n${diff.stdout || "(no tracked golden changed)"}`);
  }
  process.exit(status);
}

if (import.meta.url === pathToFileURL(process.argv[1] ?? "").href) {
  await main();
}
