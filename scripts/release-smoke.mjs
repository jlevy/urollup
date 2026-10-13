// Smoke tests for an installed urollup: it must be the executable the artifact installed,
// print exactly the release version, and reproduce committed fixture results from explicit
// sources only.
//
// Every run uses the hermetic golden environment (scripts/golden-env.mjs): a temporary HOME
// whose default discovery roots hold canary logs, empty agent roots, and `--source` with
// `--no-default-sources`, so a smoke test can never read a real agent log. PATH holds the
// install directory and only system directories without a urollup, so neither the source
// tree's build nor an earlier installation can stand in for the artifact under test.

import { spawnSync } from "node:child_process";
import { accessSync, chmodSync, constants, existsSync, mkdirSync, readFileSync, realpathSync, statSync, writeFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { EXTRACTORS, compareResults, formatDifferences, normalizeExpected } from "./check-e2e-results.mjs";
import { canaryLines, openGoldenEnvironment } from "./golden-env.mjs";
import { readArchive } from "./release-archive.mjs";
import { checkVersionOutput } from "./release-identity.mjs";
import { executableName } from "./release-targets.mjs";

export const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const FIXTURES = path.join(ROOT, "crates", "urollup-core", "tests", "fixtures");

/** One Claude Code and one Codex case, each compared field by field with its expected.json. */
export const SMOKE_CASES = ["claude-project/timestamp-precision", "codex-rollout/token-usage-records"];

export function fixtureCaseDir(fixtureCase) {
  return path.join(FIXTURES, ...fixtureCase.split("/"));
}

/**
 * Problems with one fixture report run: a clean exit, an empty stderr (unless
 * `allowStderr`, for a wrapper such as uv that reports its own progress there), no canary
 * log read, JSON on stdout, and every result its expected.json states.
 */
export function checkFixtureReport(fixtureCase, result, { allowStderr = false } = {}) {
  const where = `urollup report on ${fixtureCase}`;
  if (result.error || result.status !== 0) {
    return [`${where} exited ${result.status ?? result.signal}: ${result.error?.message ?? String(result.stderr ?? "").slice(0, 400)}`];
  }
  const problems = [];
  if (!allowStderr && result.stderr !== "") {
    problems.push(`${where} wrote to stderr: ${JSON.stringify(result.stderr.slice(0, 200))}`);
  }
  if ([...canaryLines(result.stdout), ...canaryLines(result.stderr ?? "")].length > 0) {
    return [...problems, `${where} read the hermetic HOME's canary logs instead of only --source`];
  }
  let json;
  try {
    json = JSON.parse(result.stdout);
  } catch (error) {
    return [...problems, `${where} did not print JSON: ${error.message}`];
  }
  const expected = normalizeExpected(JSON.parse(readFileSync(path.join(fixtureCaseDir(fixtureCase), "expected.json"), "utf8")));
  const { differences } = compareResults(expected.reconciled, EXTRACTORS.report(json));
  if (differences.length > 0) {
    problems.push(`${where} disagrees with expected.json:`, ...formatDifferences(differences).map((line) => `  ${line}`));
  }
  return problems;
}

/** The arguments of the plan's representative report (Testing Strategy, Packaged-Artifact Tests). */
export function reportArgs(caseDir) {
  return ["report", "--source", caseDir, "--no-default-sources", "--format", "json", "--timezone", "UTC"];
}

function pathKey(env) {
  return Object.keys(env).find((key) => key.toUpperCase() === "PATH") ?? "PATH";
}

/** Executable file names a command resolves to on this platform. */
export function commandNames(command, platform = process.platform, pathext = process.env.PATHEXT) {
  if (platform !== "win32") {
    return [command];
  }
  const extensions = (pathext ?? ".COM;.EXE;.BAT;.CMD").split(";").filter(Boolean);
  return extensions.map((extension) => `${command}${extension.toLowerCase()}`);
}

function isExecutable(file, platform) {
  try {
    if (!statSync(file).isFile()) {
      return false;
    }
    if (platform !== "win32") {
      accessSync(file, constants.X_OK);
    }
    return true;
  } catch {
    return false;
  }
}

/** The first file on `pathValue` that `command` resolves to, as a shell would find it. */
export function resolveOnPath(command, pathValue, platform = process.platform, pathext = process.env.PATHEXT) {
  const separator = platform === "win32" ? ";" : ":";
  for (const directory of (pathValue ?? "").split(separator).filter(Boolean)) {
    for (const name of commandNames(command, platform, pathext)) {
      const candidate = path.join(directory, name);
      if (isExecutable(candidate, platform)) {
        return candidate;
      }
    }
  }
  return null;
}

/**
 * PATH for a smoke run: the install directories first, then every directory of the invoking
 * PATH that does not itself hold a urollup, so nothing else can answer to the name.
 */
export function smokePath(installDirs, basePath, platform = process.platform, pathext = process.env.PATHEXT) {
  const separator = platform === "win32" ? ";" : ":";
  const kept = (basePath ?? "")
    .split(separator)
    .filter(Boolean)
    .filter((directory) => resolveOnPath("urollup", directory, platform, pathext) === null);
  return [...installDirs, ...kept].join(separator);
}

function sameFile(a, b) {
  try {
    return realpathSync(a) === realpathSync(b);
  } catch {
    return false;
  }
}

function run(file, args, env, options = {}) {
  return spawnSync(file, args, { env, encoding: "utf8", timeout: options.timeout ?? 120_000, maxBuffer: 64 * 1024 * 1024 });
}

/**
 * Smoke-test an installed urollup. `expected` is the file PATH must resolve `urollup` to;
 * `installDirs` lead PATH. Returns `{checks, problems}`; each check names what it proved.
 */
export function smokeInstalled({ expected, installDirs, version, label, baseEnv = process.env, cases = SMOKE_CASES }) {
  const checks = [];
  const problems = [];
  const golden = openGoldenEnvironment({ baseEnv });
  try {
    const env = { ...golden.env };
    const key = pathKey(env);
    env[key] = smokePath(installDirs, golden.env[key] ?? baseEnv[pathKey(baseEnv)], process.platform);
    env.CLAUDE_CONFIG_DIR = golden.dirs.emptyRoot;
    env.CODEX_HOME = golden.dirs.emptyRoot;
    env.PI_CODING_AGENT_SESSION_DIR = path.join(golden.dirs.emptyRoot, "sessions");

    const resolved = resolveOnPath("urollup", env[key], process.platform, env.PATHEXT);
    if (!resolved || !sameFile(resolved, expected)) {
      problems.push(`${label}: urollup on PATH resolves to ${resolved ?? "nothing"}, not the installed ${expected}`);
      return { checks, problems };
    }
    checks.push({ check: "path-resolution", resolved });

    const versionRun = run(resolved, ["--version"], env);
    const versionProblems = versionRun.status === 0 ? checkVersionOutput(versionRun.stdout, version) : [`urollup --version exited ${versionRun.status ?? versionRun.signal}: ${versionRun.error?.message ?? versionRun.stderr}`];
    if (versionRun.stderr !== "") {
      versionProblems.push(`urollup --version wrote to stderr: ${JSON.stringify(versionRun.stderr.slice(0, 200))}`);
    }
    problems.push(...versionProblems.map((problem) => `${label}: ${problem}`));
    checks.push({ check: "version", stdout: versionRun.stdout, status: versionRun.status });

    for (const fixtureCase of cases) {
      const result = run(resolved, reportArgs(fixtureCaseDir(fixtureCase)), env);
      const caseProblems = checkFixtureReport(fixtureCase, result);
      problems.push(...caseProblems.map((problem) => `${label}: ${problem}`));
      checks.push({ check: "fixture-report", case: fixtureCase, status: result.status, passed: caseProblems.length === 0 });
    }
    const writes = golden.homeWrites();
    if (writes.length > 0) {
      problems.push(`${label}: the smoke runs wrote into the hermetic HOME: ${writes.join(", ")}`);
    }
  } finally {
    golden.cleanup();
  }
  return { checks, problems };
}

/** Find a Python interpreter for uv on the invoking PATH, skipping Windows Store aliases. */
export function findPython(baseEnv = process.env, platform = process.platform) {
  const pathValue = baseEnv[pathKey(baseEnv)] ?? "";
  const separator = platform === "win32" ? ";" : ":";
  for (const directory of pathValue.split(separator).filter(Boolean)) {
    if (/WindowsApps/i.test(directory)) {
      continue;
    }
    for (const name of platform === "win32" ? ["python.exe", "python3.exe"] : ["python3", "python"]) {
      const candidate = path.join(directory, name);
      if (existsSync(candidate) && isExecutable(candidate, platform)) {
        return candidate;
      }
    }
  }
  return null;
}

/**
 * The environment for uv in a smoke or probe: isolated cache, tool and bin directories under
 * `dir`, no user configuration, and no managed-Python downloads.
 */
export function isolatedUvEnv(dir, baseEnv = process.env) {
  const env = { ...baseEnv };
  for (const name of Object.keys(env)) {
    if (/^UV_/.test(name)) {
      delete env[name];
    }
  }
  return {
    ...env,
    UV_CACHE_DIR: path.join(dir, "uv-cache"),
    UV_TOOL_DIR: path.join(dir, "uv-tools"),
    UV_TOOL_BIN_DIR: path.join(dir, "uv-bin"),
    UV_PYTHON_DOWNLOADS: "never",
    UV_NO_CONFIG: "1",
    UV_NO_PROGRESS: "1",
  };
}

// ---------------------------------------------------------------------------------------
// Installing artifacts (shared by native validation and the post-publication probes)

/** spawnSync with the options every release subprocess uses; returns its result. */
export function spawnRun(program, args, { env = process.env, cwd, timeout = 600_000 } = {}) {
  return spawnSync(program, args, { env, cwd, encoding: "utf8", timeout, maxBuffer: 256 * 1024 * 1024 });
}

/** Write an archive's members into `dir` with their modes; returns the executable's path. */
export function extractArchiveTo(artifact, data, dir) {
  mkdirSync(dir, { recursive: true });
  for (const entry of readArchive(artifact.name, data)) {
    const target = path.join(dir, entry.name);
    writeFileSync(target, entry.data);
    if (process.platform !== "win32") {
      chmodSync(target, entry.mode & 0o777);
    }
  }
  return path.join(dir, executableName(artifact.os));
}

function uvCommand(baseEnv) {
  return resolveOnPath("uv", baseEnv[pathKey(baseEnv)]) ?? "uv";
}

/**
 * Install a wheel, or a PyPI requirement when `index` is set, with `uv tool install` into
 * isolated directories under `dir`; returns where the executable went. Throws on failure.
 */
export function installWheel({ wheel, dir, index = false, requirement, run = spawnRun, baseEnv = process.env }) {
  const env = isolatedUvEnv(dir, baseEnv);
  const python = findPython(baseEnv);
  const args = ["tool", "install", ...(python ? ["--python", python] : []), ...(index ? [requirement] : ["--no-index", wheel])];
  const result = run(uvCommand(baseEnv), args, { env });
  if (result.error || result.status !== 0) {
    throw new Error(`uv ${args.join(" ")} failed (${result.error?.message ?? `exit ${result.status}`}): ${String(result.stderr ?? "").trim().slice(-400)}`);
  }
  const binDir = env.UV_TOOL_BIN_DIR;
  const installed = resolveOnPath("urollup", binDir);
  if (!installed) {
    throw new Error(`uv tool install left no urollup in ${binDir}`);
  }
  return { binDir, toolDir: env.UV_TOOL_DIR, installed };
}

/**
 * The ephemeral `uv tool run` path: `--version` and one fixture report through uv, from a
 * local wheel with no index, or from PyPI when `index` is set. Returns `{checks, problems}`.
 */
export function ephemeralUv({ from, dir, version, index = false, run = spawnRun, baseEnv = process.env }) {
  const env = isolatedUvEnv(dir, baseEnv);
  const python = findPython(baseEnv);
  const base = ["tool", "run", "--isolated", ...(python ? ["--python", python] : []), ...(index ? [] : ["--no-index"]), "--from", from, "urollup"];
  const problems = [];
  const versionRun = run(uvCommand(baseEnv), [...base, "--version"], { env, timeout: 300_000 });
  problems.push(...(versionRun.status === 0 ? checkVersionOutput(versionRun.stdout, version) : [`uv tool run urollup --version exited ${versionRun.status}: ${versionRun.stderr}`]));
  const fixtureCase = SMOKE_CASES[0];
  const reportRun = run(uvCommand(baseEnv), [...base, ...reportArgs(fixtureCaseDir(fixtureCase))], {
    env: { ...env, NO_COLOR: "1", CLAUDE_CONFIG_DIR: path.join(dir, "empty"), CODEX_HOME: path.join(dir, "empty") },
    timeout: 300_000,
  });
  problems.push(...checkFixtureReport(fixtureCase, reportRun, { allowStderr: true }).map((problem) => `uv tool run: ${problem}`));
  return { checks: [{ check: "uv-tool-run", from: path.basename(from), version: String(versionRun.stdout ?? "").trim() }], problems };
}
