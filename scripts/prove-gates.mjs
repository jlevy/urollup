#!/usr/bin/env node
// Prove that every `make check` gate fails on a committed violation.
//
// A gate nobody has watched fail is not a gate (ci-and-gates-rules). For each probe in
// tests/gate-probes/probes.json this copies the repository into a scratch directory,
// applies the probe's committed violation, runs the gate's make target exactly as
// `make check` does, and requires a nonzero exit whose output matches the probe's
// expected diagnostic. Matching the diagnostic matters as much as the exit: a gate that
// fails because a tool is missing proves nothing about the violation.
//
// Every prerequisite of `make check` must have a probe or a recorded reason in the
// manifest's `unprobed` map, so a new gate cannot land without its proof. fdu proves its
// gates the same way in miniature: tested scripts fed a violating input, and a make
// target driven against a stub tool (scripts/check-uv-version.test.mjs).

import { spawnSync } from "node:child_process";
import {
  chmodSync,
  copyFileSync,
  existsSync,
  lstatSync,
  mkdirSync,
  mkdtempSync,
  readdirSync,
  readFileSync,
  rmSync,
  statSync,
  symlinkSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
export const PROBE_DIR = path.join("tests", "gate-probes");
const PROBE_TIMEOUT_MS = 20 * 60 * 1000;

function fail(message) {
  throw new Error(message);
}

/** The prerequisites of the `check` target, in order. */
export function checkPrerequisites(makefileText) {
  const joined = makefileText.replace(/\\\n/g, " ");
  const line = joined.split("\n").find((candidate) => /^check:/.test(candidate));
  if (!line) {
    fail("Makefile has no check target");
  }
  const prerequisites = line.slice("check:".length).trim().split(/\s+/).filter(Boolean);
  if (prerequisites.length === 0) {
    fail("the check target has no prerequisites");
  }
  return prerequisites;
}

const EDIT_KINDS = ["append", "create", "replace", "substitute", "delete"];

/**
 * Validate the probe manifest against the gates `make check` runs.
 * Returns the probes, and fails on anything that would let a gate go unproved.
 */
export function validateManifest(manifest, gates) {
  if (!Array.isArray(manifest?.probes) || manifest.probes.length === 0) {
    fail("the probe manifest has no probes");
  }
  const unprobed = manifest.unprobed ?? {};
  const ids = new Set();
  for (const probe of manifest.probes) {
    for (const field of ["id", "gate", "violation", "expect"]) {
      if (typeof probe?.[field] !== "string" || probe[field].trim() === "") {
        fail(`probe ${probe?.id ?? "<unknown>"} needs a non-empty ${field}`);
      }
    }
    if (ids.has(probe.id)) {
      fail(`duplicate probe id ${probe.id}`);
    }
    ids.add(probe.id);
    if (!gates.includes(probe.gate)) {
      fail(`probe ${probe.id} names ${probe.gate}, which is not a prerequisite of make check`);
    }
    new RegExp(probe.expect, "m");
    if (!Array.isArray(probe.edits) || probe.edits.length === 0) {
      fail(`probe ${probe.id} has no edits, so it cannot introduce a violation`);
    }
    for (const edit of probe.edits) {
      const kinds = EDIT_KINDS.filter((kind) => kind in edit);
      if (typeof edit.path !== "string" || kinds.length !== 1) {
        fail(`probe ${probe.id} has an edit that needs a path and exactly one of ${EDIT_KINDS.join(", ")}`);
      }
      if (path.isAbsolute(edit.path) || edit.path.split(/[\\/]/).includes("..")) {
        fail(`probe ${probe.id} edits ${edit.path}, outside the repository copy`);
      }
    }
  }
  for (const gate of gates) {
    const proved = manifest.probes.some((probe) => probe.gate === gate);
    if (!proved && typeof unprobed[gate] !== "string") {
      fail(`gate ${gate} has no probe and no recorded reason in unprobed`);
    }
    if (proved && gate in unprobed) {
      fail(`gate ${gate} has a probe but is also listed as unprobed`);
    }
  }
  for (const gate of Object.keys(unprobed)) {
    if (!gates.includes(gate)) {
      fail(`unprobed lists ${gate}, which is not a prerequisite of make check`);
    }
  }
  return manifest.probes;
}

/** Apply one probe's edits inside `copyRoot`, reading probe files from `probeRoot`. */
export function applyEdits(copyRoot, probeRoot, probe) {
  for (const edit of probe.edits) {
    const target = path.join(copyRoot, edit.path);
    const probeFile = (name) => readFileSync(path.join(probeRoot, name), "utf8");
    if ("delete" in edit) {
      if (!existsSync(target)) {
        fail(`probe ${probe.id}: cannot delete ${edit.path}, which does not exist`);
      }
      rmSync(target, { recursive: true });
    } else if ("create" in edit) {
      if (existsSync(target)) {
        fail(`probe ${probe.id}: cannot create ${edit.path}, which already exists`);
      }
      mkdirSync(path.dirname(target), { recursive: true });
      writeFileSync(target, probeFile(edit.create));
      if (edit.executable) {
        chmodSync(target, 0o755);
      }
    } else if ("replace" in edit || "append" in edit) {
      if (!existsSync(target)) {
        fail(`probe ${probe.id}: cannot change ${edit.path}, which does not exist`);
      }
      const content = "replace" in edit ? probeFile(edit.replace) : readFileSync(target, "utf8") + probeFile(edit.append);
      writeFileSync(target, content);
    } else {
      if (!existsSync(target)) {
        fail(`probe ${probe.id}: cannot change ${edit.path}, which does not exist`);
      }
      const { from, to } = edit.substitute;
      const text = readFileSync(target, "utf8");
      const occurrences = text.split(from).length - 1;
      // A stale probe must fail the proof, not quietly apply nothing and "prove" a gate
      // against an unchanged tree.
      if (occurrences !== 1) {
        fail(`probe ${probe.id}: expected exactly one ${JSON.stringify(from)} in ${edit.path}, found ${occurrences}`);
      }
      writeFileSync(target, text.replace(from, () => to));
    }
  }
}

/** Decide whether a gate run proved the probe. */
export function judge(probe, result) {
  const output = `${result.stdout ?? ""}${result.stderr ?? ""}`;
  if (result.error) {
    return { ok: false, reason: `could not run the gate: ${result.error.message}` };
  }
  if (result.status === 0) {
    return { ok: false, reason: `make ${probe.gate} passed despite the violation` };
  }
  if (!new RegExp(probe.expect, "m").test(output)) {
    return {
      ok: false,
      reason: `make ${probe.gate} failed (exit ${result.status ?? result.signal}) without the expected diagnostic /${probe.expect}/`,
    };
  }
  return { ok: true, reason: `make ${probe.gate} failed as expected (exit ${result.status})` };
}

/** Repository files as git sees them: tracked plus untracked-but-not-ignored. */
function repositoryFiles(root) {
  const result = spawnSync("git", ["ls-files", "-z", "--cached", "--others", "--exclude-standard"], {
    cwd: root,
    encoding: "utf8",
    maxBuffer: 64 * 1024 * 1024,
  });
  if (result.error || result.status !== 0) {
    fail(`git ls-files failed: ${result.error?.message ?? result.stderr}`);
  }
  // Entries ending in "/" are nested working copies (agent worktrees); deleted tracked
  // files are listed but absent.
  const files = result.stdout
    .split("\0")
    .filter((file) => file && !file.endsWith("/") && existsSync(path.join(root, file)));
  if (files.length === 0) {
    fail("git ls-files listed no repository files");
  }
  return files;
}

function copyRepository(root, files, destination) {
  for (const file of files) {
    const source = path.join(root, file);
    if (lstatSync(source).isSymbolicLink() || !statSync(source).isFile()) {
      continue;
    }
    const target = path.join(destination, file);
    mkdirSync(path.dirname(target), { recursive: true });
    copyFileSync(source, target);
    chmodSync(target, statSync(source).mode & 0o777);
  }
  // Share the locked Node tools without reinstalling them: link each installed package,
  // and write a fresh install stamp so make sees node_modules as up to date.
  const nodeModules = path.join(root, "node_modules");
  if (existsSync(nodeModules)) {
    const linked = path.join(destination, "node_modules");
    mkdirSync(linked);
    for (const entry of readdirSync(nodeModules)) {
      if (entry === ".package-lock.json") {
        copyFileSync(path.join(nodeModules, entry), path.join(linked, entry));
      } else {
        symlinkSync(path.join(nodeModules, entry), path.join(linked, entry));
      }
    }
  }
}

export function parseArgs(argv) {
  const options = { only: [], list: false, keep: false };
  for (let index = 0; index < argv.length; index += 1) {
    const argument = argv[index];
    if (argument === "--only" && argv[index + 1]) {
      options.only.push(argv[index + 1]);
      index += 1;
    } else if (argument === "--list") {
      options.list = true;
    } else if (argument === "--keep") {
      options.keep = true;
    } else {
      fail("usage: prove-gates.mjs [--list] [--keep] [--only <probe-id>]...");
    }
  }
  return options;
}

function main() {
  const options = parseArgs(process.argv.slice(2));
  const probeRoot = path.join(ROOT, PROBE_DIR);
  const manifest = JSON.parse(readFileSync(path.join(probeRoot, "probes.json"), "utf8"));
  const gates = checkPrerequisites(readFileSync(path.join(ROOT, "Makefile"), "utf8"));
  let probes = validateManifest(manifest, gates);

  if (options.list) {
    for (const probe of probes) {
      console.log(`${probe.id}\t${probe.gate}\t${probe.violation}`);
    }
    for (const [gate, reason] of Object.entries(manifest.unprobed ?? {})) {
      console.log(`(unprobed)\t${gate}\t${reason}`);
    }
    return;
  }
  if (options.only.length > 0) {
    const unknown = options.only.filter((id) => !probes.some((probe) => probe.id === id));
    if (unknown.length > 0) {
      fail(`unknown probe ids: ${unknown.join(", ")}`);
    }
    probes = probes.filter((probe) => options.only.includes(probe.id));
  }

  const files = repositoryFiles(ROOT);
  const env = {
    ...process.env,
    // Reuse compiled dependencies across probes; only the probed crates rebuild.
    CARGO_TARGET_DIR: path.join(ROOT, "target", "gate-proofs"),
    // Each probe builds a copy at a new path, so incremental caches never get reused
    // and would otherwise accumulate gigabytes per run.
    CARGO_INCREMENTAL: "0",
    UV_PROJECT_ENVIRONMENT: path.join(ROOT, ".venv"),
  };
  delete env.MAKEFLAGS;
  delete env.MAKELEVEL;
  delete env.MFLAGS;

  const failures = [];
  for (const probe of probes) {
    const scratch = mkdtempSync(path.join(tmpdir(), "urollup-gate-proof-"));
    try {
      copyRepository(ROOT, files, scratch);
      applyEdits(scratch, probeRoot, probe);
      const started = Date.now();
      // `NPM=false`: a probe must never reinstall Node packages through the shared links.
      const result = spawnSync("make", ["--no-print-directory", probe.gate, "NPM=false", ...(probe.makeArgs ?? [])], {
        cwd: scratch,
        env: { ...env, ...(probe.env ?? {}) },
        encoding: "utf8",
        maxBuffer: 256 * 1024 * 1024,
        timeout: PROBE_TIMEOUT_MS,
      });
      const verdict = judge(probe, result);
      const seconds = ((Date.now() - started) / 1000).toFixed(1);
      if (verdict.ok) {
        console.log(`ok   ${probe.id}: ${verdict.reason} [${seconds}s]`);
      } else {
        console.log(`FAIL ${probe.id}: ${verdict.reason} [${seconds}s]`);
        const output = `${result.stdout ?? ""}${result.stderr ?? ""}`.trim().split("\n").slice(-25).join("\n");
        console.log(output.replace(/^/gm, "     | "));
        failures.push(probe.id);
      }
    } catch (error) {
      console.log(`FAIL ${probe.id}: ${error.message}`);
      failures.push(probe.id);
    } finally {
      if (options.keep) {
        console.log(`     kept ${scratch}`);
      } else {
        rmSync(scratch, { recursive: true, force: true });
      }
    }
  }

  if (failures.length > 0) {
    console.error(`gate-proofs: ${failures.length} of ${probes.length} probes did not prove their gate: ${failures.join(", ")}`);
    process.exitCode = 1;
    return;
  }
  console.log(`gate-proofs: all ${probes.length} probes failed their gates as expected`);
}

if (import.meta.url === pathToFileURL(process.argv[1] ?? "").href) {
  try {
    main();
  } catch (error) {
    console.error(`gate-proofs: ${error.message}`);
    process.exitCode = 1;
  }
}
