#!/usr/bin/env node
// Check urollup's final reconciled results on every fixture case against its expected.json.
//
// The transcript goldens in tests/golden record complete output for review; this checker
// is the structured layer beside them (golden-testing-guidelines, "Layer Domain-Focused
// Assertions"). For each fixture case it runs the built binary's JSON commands in the
// hermetic golden environment, with the case's directory as its only discovery root, and
// compares the reconciled totals, ownership counts, excluded copies, limit observations
// and diagnostics with the case's expected.json. A mismatch prints a field-by-field diff,
// and every case prints the naive-sum overcount it guards against.
//
// It never passes silently:
// - A command still exiting 2 as a scaffold stub is PENDING only while tests/golden/e2e.config.json
//   names its bead; once the command exists, that entry fails the run until removed, so
//   results are checked from the first build that can produce them.
// - A missing fixture root is PENDING only while `pendingFixtures` names its bead, with
//   the same ratchet; an existing root with no cases fails.
// - Every case needs a transcript golden at tests/golden/e2e/<case>.tryscript.md once the
//   report command exists, and a golden without a case fails.
// - Each command runs twice and must print identical bytes, and output naming the HOME
//   canary fails.
//
// Usage: node scripts/check-e2e-results.mjs [--case <substring>] [--fixtures <dir>]
// `--fixtures` points at an uncommitted corpus, such as locally sanitized cases; it skips
// the golden mapping and the fixture ratchet, and says so.

import { spawnSync } from "node:child_process";
import { accessSync, constants, existsSync, mkdirSync, readdirSync, readFileSync, statSync } from "node:fs";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

import { EXPECTED_FORMAT } from "./check-fixtures.mjs";
import { findSessions } from "./check-golden-invocations.mjs";
import { HOME_CANARY_TOKEN, canaryLines, openGoldenEnvironment, snapshotTree } from "./golden-env.mjs";

const ROOT = path.dirname(path.dirname(fileURLToPath(import.meta.url)));
// Named in diagnostics and joined onto ROOT, so it stays POSIX-style on every platform.
export const CONFIG_PATH = "tests/golden/e2e.config.json";
/** Milestone 0.1 commands whose results must be checked (plan, milestone 0.1). */
export const REQUIRED_COMMANDS = ["report", "daily", "sessions"];

// ---------------------------------------------------------------------------------------
// Configuration

function isObject(value) {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function pendingEntry(value, where) {
  if (value === undefined) {
    return undefined;
  }
  if (!isObject(value) || typeof value.bead !== "string" || typeof value.unblock !== "string") {
    throw new Error(`${where} needs a bead and an unblock condition`);
  }
  return value;
}

/** Validate tests/golden/e2e.config.json, so a command cannot drop out of the checks. */
export function validateConfig(config) {
  if (!isObject(config) || typeof config.fixturesRoot !== "string" || typeof config.goldenRoot !== "string") {
    throw new Error(`${CONFIG_PATH} needs fixturesRoot and goldenRoot`);
  }
  pendingEntry(config.pendingFixtures, "pendingFixtures");
  if (!Array.isArray(config.commands)) {
    throw new Error(`${CONFIG_PATH} needs a commands list`);
  }
  for (const command of config.commands) {
    if (!isObject(command) || typeof command.name !== "string" || !Array.isArray(command.args) || command.args[0] !== command.name) {
      throw new Error(`each command needs a name and args starting with that name: ${JSON.stringify(command)}`);
    }
    if (!(command.name in EXTRACTORS)) {
      throw new Error(`no result extractor for urollup ${command.name}`);
    }
    if (command.compare !== "all" && !(Array.isArray(command.compare) && command.compare.every((field) => field in RESULT_READERS))) {
      throw new Error(`urollup ${command.name}: compare must be "all" or a list of result fields`);
    }
    if (!command.args.includes("--timezone")) {
      throw new Error(`urollup ${command.name}: args must fix --timezone`);
    }
    pendingEntry(command.pending, `urollup ${command.name} pending`);
  }
  const missing = REQUIRED_COMMANDS.filter((name) => !config.commands.some((command) => command.name === name));
  if (missing.length > 0) {
    throw new Error(`${CONFIG_PATH} must check ${missing.join(", ")}`);
  }
  return config;
}

// ---------------------------------------------------------------------------------------
// Case discovery

/**
 * Fixture cases under `root`: every directory holding an expected.json, identified by its
 * `/`-separated relative path. Cases may not nest.
 */
export function discoverCases(root) {
  const cases = [];
  const walk = (dir, enclosing) => {
    const entries = readdirSync(dir, { withFileTypes: true });
    const isCase = entries.some((entry) => entry.isFile() && entry.name === "expected.json");
    if (isCase && enclosing) {
      throw new Error(`fixture case ${relative(root, dir)} is nested inside case ${enclosing}`);
    }
    const id = isCase ? relative(root, dir) : enclosing;
    if (isCase) {
      cases.push({ id, dir, roots: inferRoots(dir) });
    }
    for (const entry of entries) {
      if (entry.isDirectory()) {
        walk(path.join(dir, entry.name), id);
      }
    }
  };
  walk(root, undefined);
  return cases.sort((a, b) => (a.id < b.id ? -1 : a.id > b.id ? 1 : 0));
}

function relative(root, dir) {
  return path.relative(root, dir).split(path.sep).join("/");
}

/** Which native discovery roots a case directory is laid out as (design §2.1). */
export function inferRoots(dir) {
  const has = (name) => existsSync(path.join(dir, name)) && statSync(path.join(dir, name)).isDirectory();
  return { claude: has("projects"), codex: has("sessions") || has("archived_sessions") };
}

/** Discovery variables for a case: its own roots, and an empty root for every other agent. */
export function caseEnvironment(roots, caseDir, emptyRoot) {
  return {
    CLAUDE_CONFIG_DIR: roots.claude ? caseDir : emptyRoot,
    CODEX_HOME: roots.codex ? caseDir : emptyRoot,
    PI_CODING_AGENT_SESSION_DIR: path.join(emptyRoot, "sessions"),
  };
}

// ---------------------------------------------------------------------------------------
// The expected.json results contract (tests/golden/README.md, Results Contract)
//
// There is one contract: a case's expected.json is the frozen fixture record in format
// urollup-fixture-expected/v1, which scripts/check-fixtures.mjs validates in full (its
// request rows, thread totals and references must agree with its totals). This checker
// reads out of that record the results a command's output can be compared with. A result
// the record does not state is not compared, and a record stating none of them is refused,
// so a case can never pass by asserting nothing.

/** Canonical result names, and how each is read out of an expected.json. */
export const RESULT_READERS = {
  requests: (raw) => raw.totals?.requests?.unique,
  ownership: (raw) => selectCounts(raw.totals?.requests, OWNERSHIP_STATUSES),
  tokens: (raw) => raw.totals?.tokens,
  unresolved: (raw) => raw.totals?.unresolved,
  possible: (raw) => raw.totals?.possible,
  copies_excluded: (raw) => (Array.isArray(raw.copies) ? raw.copies.length : undefined),
  limit_observations: (raw) => (Array.isArray(raw.limit_observations) ? raw.limit_observations.length : undefined),
  diagnostics: (raw) => (Array.isArray(raw.diagnostics) ? normalizeDiagnostics(raw.diagnostics) : undefined),
};
/** Ownership statuses the ledger reports (design §3.6); every request row is one of them. */
export const OWNERSHIP_STATUSES = ["owned", "ambiguous", "unknown"];

function selectCounts(value, keys) {
  if (!isObject(value)) {
    return undefined;
  }
  const picked = Object.fromEntries(keys.filter((key) => value[key] !== undefined).map((key) => [key, value[key]]));
  return Object.keys(picked).length > 0 ? picked : undefined;
}

/**
 * Diagnostics as sorted `{code, count}` pairs. A fixture states a diagnostic as
 * `{code, refs}`, so its count is how many places it fired; a bare string is a code with
 * any count, and an explicit `count` wins.
 */
export function normalizeDiagnostics(list) {
  if (!Array.isArray(list)) {
    throw new Error("diagnostics must be a list of codes or {code, refs} objects");
  }
  return list
    .map((item) => {
      if (typeof item === "string") {
        return { code: item, count: undefined };
      }
      const code = item?.code ?? item?.id;
      const count = item?.count ?? (Array.isArray(item?.refs) ? item.refs.length : undefined);
      if (typeof code !== "string" || (count !== undefined && !Number.isInteger(count))) {
        throw new Error(`a diagnostic needs a string code and an optional integer count or refs: ${JSON.stringify(item)}`);
      }
      return { code, count };
    })
    .sort((a, b) => (a.code < b.code ? -1 : a.code > b.code ? 1 : 0));
}

/**
 * Counts must be non-negative integers, or objects of them. A token category is null when
 * the dialect does not report it at all (Claude Code has no separate reasoning count); that
 * still asserts something, so it is kept and compared as absent-or-zero.
 */
function checkCounts(name, value, { allowNull = false } = {}) {
  if (Number.isInteger(value) && value >= 0) {
    return;
  }
  if (value === null && allowNull) {
    return;
  }
  if (isObject(value) && Object.keys(value).length > 0) {
    for (const [key, inner] of Object.entries(value)) {
      checkCounts(`${name}.${key}`, inner, { allowNull });
    }
    return;
  }
  throw new Error(`${name} must be a non-negative integer or an object of them, found ${JSON.stringify(value)}`);
}

/**
 * Read an expected.json into `{reconciled, naive, dialect}`, with `reconciled` holding the
 * canonical results the case states and `naive` its naive-sum rules.
 */
export function normalizeExpected(raw) {
  if (!isObject(raw)) {
    throw new Error("expected.json must hold a JSON object");
  }
  if (raw.format !== EXPECTED_FORMAT) {
    throw new Error(`format must be ${EXPECTED_FORMAT}; scripts/check-fixtures.mjs defines the record and tests/golden/README.md the results read from it`);
  }
  const reconciled = {};
  for (const [name, read] of Object.entries(RESULT_READERS)) {
    const value = read(raw);
    if (value === undefined) {
      continue;
    }
    if (name !== "diagnostics") {
      checkCounts(name, value, { allowNull: name === "tokens" });
    }
    reconciled[name] = value;
  }
  if (Object.keys(reconciled).length === 0) {
    throw new Error(`expected.json states no results the checker compares (${Object.keys(RESULT_READERS).join(", ")}), so the case would check nothing`);
  }
  if (raw.naive !== undefined && !Array.isArray(raw.naive)) {
    throw new Error("naive must be a list of naive-sum rules and their results");
  }
  return { reconciled, naive: raw.naive, dialect: raw.dialect };
}

// ---------------------------------------------------------------------------------------
// Extraction from urollup JSON output
//
// Provisional: the paths follow the design's JSON rendering of summary totals (§4.2, §5.2)
// until `urollup report --format json` exists. The bead that lands it fixes these paths and
// the sample outputs in tests/golden/samples/outputs/ in the same commit.

function requestCount(value) {
  if (Number.isInteger(value)) {
    return value;
  }
  return isObject(value) ? Object.values(value).reduce((sum, count) => sum + (Number.isInteger(count) ? count : 0), 0) : undefined;
}

function addCounts(into, value) {
  for (const [key, inner] of Object.entries(value ?? {})) {
    if (isObject(inner)) {
      into[key] = addCounts(isObject(into[key]) ? into[key] : {}, inner);
    } else if (Number.isInteger(inner)) {
      into[key] = (into[key] ?? 0) + inner;
    }
  }
  return into;
}

function rowsExtractor(json) {
  if (!Array.isArray(json?.rows)) {
    return {};
  }
  const ownership = {};
  const tokens = {};
  for (const row of json.rows) {
    addCounts(ownership, isObject(row.requests) ? row.requests : {});
    addCounts(tokens, row.tokens);
  }
  const requests = json.rows.reduce((sum, row) => sum + (requestCount(row.requests) ?? 0), 0);
  return { requests, ownership, tokens };
}

/** Per command, a function from parsed JSON output to canonical results. */
export const EXTRACTORS = {
  report: (json) => ({
    requests: requestCount(json?.totals?.requests),
    ownership: isObject(json?.totals?.requests) ? json.totals.requests : undefined,
    tokens: json?.totals?.tokens,
    unresolved: json?.totals?.unresolved,
    possible: json?.totals?.possible,
    copies_excluded: json?.coverage?.copies_excluded,
    limit_observations: json?.coverage?.limit_observations,
    diagnostics: Array.isArray(json?.diagnostics) ? normalizeDiagnostics(json.diagnostics) : undefined,
  }),
  // Group rows must sum to the totals (design §4.2), so rollups compare against the same truth.
  daily: rowsExtractor,
  sessions: rowsExtractor,
};

// ---------------------------------------------------------------------------------------
// Comparison

function compareValue(field, expected, actual, differences) {
  if (isObject(expected)) {
    if (!isObject(actual)) {
      differences.push({ field, expected: "an object", actual: actual === undefined ? "absent" : JSON.stringify(actual) });
      return;
    }
    for (const key of Object.keys(expected)) {
      compareValue(`${field}.${key}`, expected[key], actual[key], differences);
    }
    return;
  }
  if (expected === null) {
    // The dialect reports no such category, so the output may omit it or print zero.
    if (actual !== undefined && actual !== null && actual !== 0) {
      differences.push({ field, expected: "absent or 0", actual });
    }
    return;
  }
  if (expected !== actual) {
    differences.push({ field, expected, actual: actual === undefined ? "absent" : actual });
  }
}

function compareDiagnostics(expected, actual, differences) {
  if (actual === undefined) {
    differences.push({ field: "diagnostics", expected: `${expected.length} codes`, actual: "absent" });
    return;
  }
  const counts = (list) => list.reduce((map, { code, count }) => map.set(code, (map.get(code) ?? 0) + (count ?? 1)), new Map());
  const found = counts(actual);
  for (const { code, count } of expected) {
    if (!found.has(code)) {
      differences.push({ field: "diagnostics", expected: count === undefined ? code : `${code} x${count}`, actual: "missing" });
    } else if (count !== undefined && found.get(code) !== count) {
      differences.push({ field: `diagnostics ${code}`, expected: count, actual: found.get(code) });
    }
  }
  const wanted = new Set(expected.map(({ code }) => code));
  for (const [code, count] of found) {
    if (!wanted.has(code)) {
      differences.push({ field: "diagnostics", expected: "none", actual: `unexpected ${code} x${count}` });
    }
  }
}

/** Field-by-field differences between expected and extracted results, for `fields`. */
export function compareResults(expected, actual, fields = "all") {
  const names = Object.keys(expected).filter((name) => fields === "all" || fields.includes(name));
  const differences = [];
  for (const name of names) {
    if (name === "diagnostics") {
      compareDiagnostics(expected.diagnostics, actual.diagnostics, differences);
    } else {
      compareValue(name, expected[name], actual[name], differences);
    }
  }
  return { compared: names, differences };
}

function formatCount(value) {
  return typeof value === "number" ? value.toLocaleString("en-US") : String(value);
}

function tokenTotal(tokens) {
  // Reasoning tokens are a subset of output (design §4.1), so totals never add both.
  return isObject(tokens) ? Object.entries(tokens).reduce((sum, [key, value]) => sum + (key !== "reasoning" && Number.isInteger(value) ? value : 0), 0) : undefined;
}

function ratio(naive, reconciled) {
  return reconciled > 0 ? `${(naive / reconciled).toFixed(1)}x` : "n/a";
}

/**
 * One line putting the case's naive-sum rules beside the reconciled truth: the rule whose
 * token total goes furthest wrong, which is the double counting (or, for a strict reader
 * that drops a whole file, the undercounting) the case exists to prove urollup avoids.
 */
export function overcountSummary(reconciled, naive) {
  const rules = (Array.isArray(naive) ? naive : []).filter(isObject);
  if (rules.length === 0) {
    return "no naive sum recorded";
  }
  const reconciledTokens = tokenTotal(reconciled.tokens);
  const distance = (rule) => {
    const tokens = tokenTotal(rule.tokens);
    if (tokens !== undefined && reconciledTokens !== undefined) {
      return Math.abs(tokens - reconciledTokens);
    }
    return Number.isInteger(rule.requests) && Number.isInteger(reconciled.requests) ? Math.abs(rule.requests - reconciled.requests) : 0;
  };
  const worst = rules.reduce((furthest, rule) => (distance(rule) > distance(furthest) ? rule : furthest));
  const parts = [];
  if (Number.isInteger(worst.requests) && Number.isInteger(reconciled.requests)) {
    parts.push(`${formatCount(worst.requests)} requests (${ratio(worst.requests, reconciled.requests)} the reconciled ${formatCount(reconciled.requests)})`);
  }
  const worstTokens = tokenTotal(worst.tokens);
  if (worstTokens !== undefined && reconciledTokens !== undefined) {
    parts.push(`${formatCount(worstTokens)} tokens (${ratio(worstTokens, reconciledTokens)} the reconciled ${formatCount(reconciledTokens)})`);
  }
  const scope = rules.length > 1 ? `, worst of ${rules.length} rules` : "";
  const rule = typeof worst.rule === "string" ? ` "${worst.rule}"` : "";
  return parts.length > 0 ? `naive rule${rule}: ${parts.join(", ")}${scope}` : "naive sum has no comparable counts";
}

export function formatDifferences(differences) {
  const width = Math.max(5, ...differences.map(({ field }) => field.length));
  const valueWidth = Math.max(8, ...differences.map(({ expected }) => formatCount(expected).length));
  return [
    `${"field".padEnd(width)}  ${"expected".padEnd(valueWidth)}  actual`,
    ...differences.map(({ field, expected, actual }) => `${field.padEnd(width)}  ${formatCount(expected).padEnd(valueWidth)}  ${formatCount(actual)}`),
  ];
}

// ---------------------------------------------------------------------------------------
// Running

/** Whether a bare command's result is the milestone 0.1 scaffold stub. */
export function isStub(result) {
  return result.status === 2 && result.stdout === "" && /is not implemented yet/.test(result.stderr);
}

function tail(text, count = 8) {
  return text.trimEnd().split(/\r?\n/).slice(-count);
}

/** Check one command on one case; returns failure lines, or none. */
function checkCommand({ command, fixtureCase, expected, runUrollup, env }) {
  const invocation = `urollup ${command.args.join(" ")}`;
  const first = runUrollup(command.args, env);
  const problems = [];
  if (first.status !== 0) {
    return [`${invocation} exited ${first.status}`, ...tail(first.stderr).map((line) => `  stderr: ${line}`)];
  }
  const canary = [...canaryLines(first.stdout), ...canaryLines(first.stderr)];
  if (canary.length > 0) {
    return [`${invocation} read the hermetic HOME's canary logs (${HOME_CANARY_TOKEN}) instead of only the case's roots`, ...canary.slice(0, 3).map((line) => `  ${line.trim()}`)];
  }
  const second = runUrollup(command.args, env);
  if (second.status !== first.status || second.stdout !== first.stdout) {
    const a = first.stdout.split("\n");
    const b = second.stdout.split("\n");
    const line = a.findIndex((text, index) => text !== b[index]);
    problems.push(`${invocation} is not deterministic: two identical runs differ${line >= 0 ? ` at stdout line ${line + 1}` : ""}`);
  }
  let json;
  try {
    json = JSON.parse(first.stdout);
  } catch (error) {
    return [...problems, `${invocation} did not print JSON: ${error.message}`, `  stdout: ${first.stdout.slice(0, 200)}`];
  }
  const { differences } = compareResults(expected.reconciled, EXTRACTORS[command.name](json), command.compare);
  if (differences.length > 0) {
    problems.push(`${invocation} disagrees with ${fixtureCase.id}/expected.json:`, ...formatDifferences(differences).map((line) => `  ${line}`));
  }
  return problems;
}

/**
 * Run every check. `runUrollup(args, envOverrides)` returns `{status, stdout, stderr}`;
 * `goldenSessions` lists committed session paths, or is undefined to skip the mapping.
 * Returns `{status: "ok" | "pending" | "fail", lines}`.
 */
export function checkE2E({ config, fixturesRoot, goldenSessions, runUrollup, emptyRoot, caseFilter, local = false }) {
  const lines = [];
  const failures = [];
  const pending = [];
  const fail = (message, details = []) => failures.push([message, ...details]);

  // Commands: a stub is pending only while its bead is named; an implemented command is checked.
  const active = [];
  const noCase = caseEnvironment({ claude: false, codex: false }, emptyRoot, emptyRoot);
  for (const command of config.commands) {
    const stub = isStub(runUrollup([command.name], noCase));
    if (stub && command.pending) {
      pending.push(`urollup ${command.name} is not implemented yet (${command.pending.bead}: ${command.pending.unblock})`);
    } else if (stub) {
      fail(`urollup ${command.name} is still the scaffold stub, but ${CONFIG_PATH} lists it as implemented`);
    } else if (command.pending) {
      fail(`urollup ${command.name} is implemented now; delete its pending entry in ${CONFIG_PATH} so its results are checked (${command.pending.bead})`);
    } else {
      active.push(command);
    }
  }

  // Fixtures: the same ratchet, unless an explicit local corpus was named.
  const rootExists = existsSync(fixturesRoot);
  if (local) {
    lines.push(`local corpus ${fixturesRoot}: golden mapping and fixture ratchet not applied`);
    if (!rootExists) {
      fail(`the fixture corpus ${fixturesRoot} does not exist`);
    }
  } else if (!rootExists && config.pendingFixtures) {
    pending.push(`no fixture corpus at ${config.fixturesRoot} yet (${config.pendingFixtures.bead}: ${config.pendingFixtures.unblock})`);
  } else if (!rootExists) {
    fail(`the fixture corpus ${config.fixturesRoot} is missing`);
  } else if (config.pendingFixtures) {
    fail(`fixtures exist at ${config.fixturesRoot}; delete pendingFixtures in ${CONFIG_PATH} (${config.pendingFixtures.bead})`);
  }

  let cases = [];
  if (rootExists) {
    try {
      cases = discoverCases(fixturesRoot);
    } catch (error) {
      fail(error.message);
    }
    if (cases.length === 0) {
      fail(`no fixture cases (directories with expected.json) under ${local ? fixturesRoot : config.fixturesRoot}`);
    }
  }
  if (caseFilter !== undefined) {
    cases = cases.filter((fixtureCase) => fixtureCase.id.includes(caseFilter));
    if (cases.length === 0) {
      fail(`--case ${caseFilter} matches no fixture case`);
    }
  }

  // Transcript goldens: tests/golden/e2e/<case>.tryscript.md for every case, and no orphans.
  if (goldenSessions !== undefined && rootExists && caseFilter === undefined) {
    const prefix = `${config.goldenRoot}/`;
    const expectedGoldens = new Map(cases.map((fixtureCase) => [`${prefix}${fixtureCase.id}.tryscript.md`, fixtureCase.id]));
    const committed = new Set(goldenSessions.filter((file) => file.startsWith(prefix)));
    for (const file of committed) {
      if (!expectedGoldens.has(file)) {
        fail(`${file} has no fixture case at ${config.fixturesRoot}/${file.slice(prefix.length, -".tryscript.md".length)}`);
      }
    }
    const missing = [...expectedGoldens.keys()].filter((file) => !committed.has(file));
    if (missing.length > 0 && active.some((command) => command.name === "report")) {
      fail(`${missing.length} fixture cases have no transcript golden; create each with node scripts/new-e2e-golden.mjs <case>`, missing.map((file) => `  ${file}`));
    } else if (missing.length > 0) {
      pending.push(`${missing.length} fixture cases have no transcript golden yet (created once urollup report exists)`);
    }
  }

  // Cases: every expected.json must satisfy the contract now; results are compared per active command.
  let checked = 0;
  for (const fixtureCase of cases) {
    let expected;
    try {
      expected = normalizeExpected(JSON.parse(readFileSync(path.join(fixtureCase.dir, "expected.json"), "utf8")));
    } catch (error) {
      fail(`${fixtureCase.id}/expected.json: ${error.message}`);
      continue;
    }
    if (!fixtureCase.roots.claude && !fixtureCase.roots.codex) {
      fail(`${fixtureCase.id} has neither projects/ (Claude Code) nor sessions/ or archived_sessions/ (Codex), so no discovery root names it`);
      continue;
    }
    if (active.length === 0) {
      continue;
    }
    const env = caseEnvironment(fixtureCase.roots, fixtureCase.dir, emptyRoot);
    const problems = active.flatMap((command) => checkCommand({ command, fixtureCase, expected, runUrollup, env }));
    const context = overcountSummary(expected.reconciled, expected.naive);
    if (problems.length > 0) {
      fail(`${fixtureCase.id}`, [...problems.map((line) => `  ${line}`), `  for context, ${context}`]);
    } else {
      checked += 1;
      lines.push(`ok      ${fixtureCase.id}: ${active.map((command) => command.name).join(", ")} match; ${context}`);
    }
  }

  for (const note of pending) {
    lines.push(`PENDING ${note}`);
  }
  for (const [message, ...details] of failures) {
    lines.push(`FAIL    ${message}`, ...details.map((detail) => `        ${detail}`));
  }
  const status = failures.length > 0 ? "fail" : pending.length > 0 ? "pending" : "ok";
  const summary =
    status === "fail"
      ? `${failures.length} failures`
      : status === "pending"
        ? `PENDING: results checked for ${checked} of ${cases.length} fixture cases; the rest stay unverified until each PENDING item above is resolved`
        : `${checked} fixture cases match their expected results`;
  lines.push(`e2e-results: ${summary}`);
  return { status, lines, checked };
}

// ---------------------------------------------------------------------------------------
// Command line

export function parseArgs(argv) {
  const options = {};
  for (let index = 0; index < argv.length; index += 1) {
    const argument = argv[index];
    if ((argument === "--case" || argument === "--fixtures") && argv[index + 1] !== undefined) {
      options[argument.slice(2)] = argv[index + 1];
      index += 1;
    } else {
      throw new Error("usage: check-e2e-results.mjs [--case <substring>] [--fixtures <dir>]");
    }
  }
  return options;
}

function binaryPath() {
  const targetDir = process.env.CARGO_TARGET_DIR ? path.resolve(ROOT, process.env.CARGO_TARGET_DIR) : path.join(ROOT, "target");
  const binary = path.join(targetDir, "debug", `urollup${process.platform === "win32" ? ".exe" : ""}`);
  if (!existsSync(binary) || !statSync(binary).isFile()) {
    throw new Error(`the urollup binary is not built at ${binary}; run make build`);
  }
  if (process.platform !== "win32") {
    accessSync(binary, constants.X_OK);
  }
  return binary;
}

function main() {
  const options = parseArgs(process.argv.slice(2));
  const config = validateConfig(JSON.parse(readFileSync(path.join(ROOT, CONFIG_PATH), "utf8")));
  const binary = binaryPath();
  const local = options.fixtures !== undefined;
  const fixturesRoot = local ? path.resolve(options.fixtures) : path.join(ROOT, config.fixturesRoot);
  const golden = openGoldenEnvironment();
  let outcome;
  try {
    const work = path.join(golden.dirs.root, "work");
    mkdirSync(work);
    const runUrollup = (args, overrides) => {
      const result = spawnSync(binary, args, { cwd: work, env: { ...golden.env, ...overrides }, encoding: "utf8", timeout: 60_000, maxBuffer: 64 * 1024 * 1024 });
      if (result.error) {
        return { status: -1, stdout: "", stderr: `could not run ${binary}: ${result.error.message}` };
      }
      return { status: result.status ?? -1, stdout: result.stdout, stderr: result.stderr };
    };
    console.log(`e2e-results: ${binary} in a hermetic environment`);
    outcome = checkE2E({
      config,
      fixturesRoot,
      goldenSessions: local ? undefined : findSessions(ROOT),
      runUrollup,
      emptyRoot: golden.dirs.emptyRoot,
      caseFilter: options.case,
      local,
    });
    const strays = [...golden.homeWrites(), ...[...snapshotTree(work).keys()].map((file) => `created work/${file}`)];
    if (strays.length > 0) {
      outcome.status = "fail";
      const summary = outcome.lines.pop();
      outcome.lines.push("FAIL    urollup wrote outside the capture directory:", ...strays.map((line) => `        ${line}`), summary.replace(/^e2e-results: .*/, "e2e-results: failed, urollup wrote into HOME or its working directory"));
    }
  } finally {
    golden.cleanup();
  }
  for (const line of outcome.lines) {
    console.log(line);
  }
  process.exitCode = outcome.status === "fail" ? 1 : 0;
}

if (import.meta.url === pathToFileURL(process.argv[1] ?? "").href) {
  try {
    main();
  } catch (error) {
    console.error(`e2e-results: ${error.message}`);
    process.exitCode = 2;
  }
}
