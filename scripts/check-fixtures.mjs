#!/usr/bin/env node
// Check the frozen dialect fixtures under crates/urollup-core/tests/fixtures.
//
// Every fixture case is a small discovery root plus an `expected.json` holding the
// reconciled truth. This check keeps the corpus honest without a Rust build:
//
// - every `.jsonl` line parses as a JSON object, except lines a case declares malformed or
//   as its pending tail, which must really fail to parse;
// - `.jsonl.zst` files decompress (Node's zlib zstd), or are skipped with a note on a Node
//   that lacks zstd;
// - `expected.json` is well formed, lists exactly the case's data files, resolves every
//   `path:line` reference, and its request rows add up to its totals;
// - nothing looks private: home directories other than placeholders, email addresses,
//   credential-shaped tokens, or the name and home directory of the machine running it.
//
// Zero dependencies. The credential patterns follow metaproc's public-hygiene scanner
// (devtools/public_hygiene.py); see PROVENANCE.md.

import { existsSync, readdirSync, readFileSync, statSync } from "node:fs";
import { homedir, userInfo } from "node:os";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import zlib from "node:zlib";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
export const FIXTURE_DIR = path.join("crates", "urollup-core", "tests", "fixtures");
export const DIALECTS = ["claude-project", "codex-rollout"];
export const EXPECTED_FORMAT = "urollup-fixture-expected/v1";
export const TOKEN_KEYS = ["uncached_input", "cache_read", "cache_write", "output", "reasoning"];
const OWNERSHIP = ["owned", "ambiguous", "unknown"];
const ZSTD_MAGIC = Buffer.from([0x28, 0xb5, 0x2f, 0xfd]);
const CASE_FILES = new Set(["expected.json", "README.md"]);
const REF_PATTERN = /^((?:[^\s:/]+\/)*[^\s:/]+\.(?:jsonl|jsonl\.zst|json)):([1-9][0-9]*)$/;

// Placeholder account names that fixtures may use in home-directory paths.
export const PLACEHOLDER_NAMES = new Set(["example"]);
// Generic account names (CI runners, containers) that are ordinary words in logs, such as
// `"role":"user"`, so they are not treated as identifying the machine.
const GENERIC_ACCOUNT_NAMES = new Set(["admin", "codespace", "node", "root", "runner", "runneradmin", "test", "ubuntu", "user", "vscode"]);
const PLACEHOLDER_EMAIL_DOMAINS = ["example.com", "example.org", "example.net", "example.invalid"];

const CREDENTIAL_PATTERNS = [
  /\bsk-(?:ant-|proj-)?[A-Za-z0-9_-]{20,}/,
  /\bgh[opsu]_[A-Za-z0-9]{20,}/,
  /\bgithub_pat_[A-Za-z0-9_]{20,}/,
  /\bglpat-[A-Za-z0-9_-]{20,}/,
  /\bAKIA[0-9A-Z]{16}\b/,
  /\bxox[abprs]-[A-Za-z0-9-]{10,}/,
  /\bAIza[0-9A-Za-z_-]{35}/,
  /\bnpm_[A-Za-z0-9]{36}\b/,
  /\bhf_[A-Za-z0-9]{30,}/,
  /\beyJ[A-Za-z0-9_-]{10,}\.eyJ[A-Za-z0-9_-]{10,}\./,
  /\bBearer\s+[A-Za-z0-9._~+/-]{20,}/,
  /-----BEGIN (?:[A-Z]+ )?PRIVATE KEY-----/,
];

/**
 * Names that identify the machine running the check: its account name and the last
 * component of its home directory, when they are long enough to mean something and are
 * not fixture placeholders.
 */
export function machineIdentities() {
  const names = new Set();
  try {
    names.add(userInfo().username);
  } catch {
    // No passwd entry (some containers); the home directory still applies.
  }
  names.add(path.basename(homedir()));
  return [...names].filter((name) => {
    const lower = name?.toLowerCase() ?? "";
    return lower.length >= 3 && !PLACEHOLDER_NAMES.has(lower) && !GENERIC_ACCOUNT_NAMES.has(lower);
  });
}

function escapeRegExp(text) {
  return text.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

/**
 * Return privacy findings for one text, as `{ line, why }`.
 * `identities` are extra names (such as the local account name) that must not appear as
 * whole words.
 */
export function scanPrivacy(text, { identities = [] } = {}) {
  const findings = [];
  const identityPatterns = identities.map(
    (name) => new RegExp(`(?<![A-Za-z0-9])${escapeRegExp(name)}(?![A-Za-z0-9])`, "i"),
  );
  text.split("\n").forEach((line, index) => {
    const report = (why) => findings.push({ line: index + 1, why });
    for (const match of line.matchAll(/[/\\](?:Users|home)[/\\]+([^/\\\s"'`:]+)/g)) {
      if (!PLACEHOLDER_NAMES.has(match[1].toLowerCase())) {
        report(`a home directory that is not a placeholder (${match[0]})`);
      }
    }
    // Claude Code encodes the working directory into project directory names.
    for (const match of line.matchAll(/(?<![A-Za-z0-9])-(?:Users|home)-([A-Za-z0-9._]+)/g)) {
      if (!PLACEHOLDER_NAMES.has(match[1].toLowerCase())) {
        report(`an encoded home directory that is not a placeholder (${match[0]})`);
      }
    }
    for (const match of line.matchAll(/[A-Za-z0-9._%+-]+@([A-Za-z0-9-]+(?:\.[A-Za-z0-9-]+)*\.[A-Za-z]{2,})/g)) {
      const domain = match[1].toLowerCase();
      if (!PLACEHOLDER_EMAIL_DOMAINS.some((allowed) => domain === allowed || domain.endsWith(`.${allowed}`))) {
        report("an email address");
      }
    }
    if (CREDENTIAL_PATTERNS.some((pattern) => pattern.test(line))) {
      report("a credential-shaped token");
    }
    if (identityPatterns.some((pattern) => pattern.test(line))) {
      report("the account or home directory name of the machine running the check");
    }
  });
  return findings;
}

/** Split file content into lines; `tail` is true when the last line has no newline. */
export function splitLines(text) {
  if (text === "") {
    return { lines: [], tail: false };
  }
  const tail = !text.endsWith("\n");
  const lines = (tail ? text : text.slice(0, -1)).split("\n");
  return { lines, tail };
}

function parsesAsObject(line) {
  try {
    const value = JSON.parse(line);
    return value !== null && typeof value === "object" && !Array.isArray(value);
  } catch {
    return false;
  }
}

/**
 * Check one JSONL text against the lines a case declares malformed or pending.
 * Returns problems as strings.
 */
export function checkJsonlText(relPath, text, { malformed = new Set(), pendingTail = new Set() } = {}) {
  const problems = [];
  const { lines, tail } = splitLines(text);
  if (lines.length === 0) {
    problems.push(`${relPath}: is empty`);
  }
  lines.forEach((line, index) => {
    const ref = `${relPath}:${index + 1}`;
    const isLast = index === lines.length - 1;
    const parses = parsesAsObject(line);
    if (pendingTail.has(ref)) {
      if (!isLast || !tail) {
        problems.push(`${ref}: declared as the pending tail, but it is not an unterminated last line`);
      } else if (parses) {
        problems.push(`${ref}: declared as the pending tail, but it parses as a complete record`);
      }
    } else if (malformed.has(ref)) {
      if (parses) {
        problems.push(`${ref}: declared malformed, but it parses as JSON`);
      }
    } else if (!parses) {
      problems.push(`${ref}: does not parse as a JSON object and is not declared malformed or pending`);
    } else if (isLast && tail) {
      problems.push(`${ref}: the file does not end with a newline, and no pending tail is declared`);
    }
  });
  for (const ref of [...malformed, ...pendingTail]) {
    const match = REF_PATTERN.exec(ref);
    if (match && match[1] === relPath && Number(match[2]) > lines.length) {
      problems.push(`${ref}: declared line is past the end of the file`);
    }
  }
  return problems;
}

function isCount(value) {
  return Number.isInteger(value) && value >= 0;
}

/** Problems with a token object, or [] when it is valid. */
export function tokenProblems(tokens, where) {
  if (tokens === null || typeof tokens !== "object") {
    return [`${where}: tokens must be an object`];
  }
  const problems = [];
  for (const key of TOKEN_KEYS) {
    const value = tokens[key];
    if (key === "reasoning" ? !(value === null || isCount(value)) : !isCount(value)) {
      problems.push(`${where}: tokens.${key} must be a non-negative integer${key === "reasoning" ? " or null" : ""}`);
    }
  }
  return problems;
}

function addTokens(rows) {
  const sum = Object.fromEntries(TOKEN_KEYS.map((key) => [key, 0]));
  let reasoningRecorded = false;
  for (const tokens of rows) {
    for (const key of TOKEN_KEYS) {
      if (key === "reasoning") {
        if (tokens.reasoning !== null) {
          reasoningRecorded = true;
          sum.reasoning += tokens.reasoning;
        }
      } else {
        sum[key] += tokens[key];
      }
    }
  }
  if (!reasoningRecorded) {
    sum.reasoning = null;
  }
  return sum;
}

function sameTokens(a, b) {
  return TOKEN_KEYS.every((key) => a[key] === b[key]);
}

/** Every string in a JSON value that looks like a `path:line` reference. */
export function collectRefs(value, found = []) {
  if (typeof value === "string") {
    if (REF_PATTERN.test(value)) {
      found.push(value);
    }
  } else if (Array.isArray(value)) {
    value.forEach((item) => collectRefs(item, found));
  } else if (value && typeof value === "object") {
    Object.values(value).forEach((item) => collectRefs(item, found));
  }
  return found;
}

/**
 * Validate an expected.json value. `lineCounts` maps each data file's relative path to its
 * line count (null for files that are not line-oriented or were not decoded).
 */
export function validateExpected(expected, { caseName, dialect, lineCounts }) {
  const problems = [];
  const where = `${dialect}/${caseName}/expected.json`;
  const fail = (message) => problems.push(`${where}: ${message}`);
  if (expected === null || typeof expected !== "object" || Array.isArray(expected)) {
    return [`${where}: must be a JSON object`];
  }
  if (expected.format !== EXPECTED_FORMAT) {
    fail(`format must be ${EXPECTED_FORMAT}`);
  }
  if (expected.case !== caseName) {
    fail(`case must be ${JSON.stringify(caseName)}, the directory name`);
  }
  if (expected.dialect !== dialect) {
    fail(`dialect must be ${JSON.stringify(dialect)}, the parent directory`);
  }
  if (typeof expected.summary !== "string" || expected.summary.trim() === "") {
    fail("summary must be a non-empty string");
  }
  if (!Array.isArray(expected.rules) || expected.rules.length === 0 || !expected.rules.every((rule) => typeof rule === "string" && rule)) {
    fail("rules must be a non-empty array of design rule names");
  }
  if (typeof expected.agent?.name !== "string" || typeof expected.agent?.versions !== "string") {
    fail("agent must name the agent and the versions the case models");
  }
  for (const field of ["threads", "requests", "copies", "diagnostics", "limit_observations"]) {
    if (!Array.isArray(expected[field])) {
      fail(`${field} must be an array`);
    }
  }
  if (!Array.isArray(expected.naive) || expected.naive.length === 0) {
    fail("naive must list at least one naive-sum rule and its result");
  }
  if (!Array.isArray(expected.decode?.malformed) || !Array.isArray(expected.decode?.pending_tail)) {
    fail("decode must have malformed and pending_tail arrays");
  }

  // Files: exactly the case's data files, with correct line counts.
  if (!Array.isArray(expected.files) || expected.files.length === 0) {
    fail("files must list the case's data files");
  } else {
    const listed = new Set();
    for (const file of expected.files) {
      if (typeof file?.path !== "string") {
        fail("every files entry needs a path");
        continue;
      }
      listed.add(file.path);
      if (!lineCounts.has(file.path)) {
        fail(`files lists ${file.path}, which does not exist in the case`);
      } else if (file.lines !== undefined && lineCounts.get(file.path) !== null && file.lines !== lineCounts.get(file.path)) {
        fail(`files says ${file.path} has ${file.lines} lines, but it has ${lineCounts.get(file.path)}`);
      }
    }
    for (const actual of lineCounts.keys()) {
      if (!listed.has(actual)) {
        fail(`${actual} is in the case but not listed in files`);
      }
    }
  }

  // References resolve to a file and a line within it.
  for (const ref of collectRefs(expected)) {
    const [, file, line] = REF_PATTERN.exec(ref);
    if (!lineCounts.has(file)) {
      fail(`reference ${ref} names a file that is not in the case`);
    } else if (lineCounts.get(file) !== null && Number(line) > lineCounts.get(file)) {
      fail(`reference ${ref} is past the end of ${file}`);
    }
  }

  // Request rows add up to the totals.
  const requests = Array.isArray(expected.requests) ? expected.requests : [];
  const rowTokens = [];
  requests.forEach((request, index) => {
    const at = `requests[${index}]`;
    if (request === null || typeof request !== "object" || typeof request.key !== "object" || request.key === null) {
      fail(`${at} needs a key object`);
      return;
    }
    if (!OWNERSHIP.includes(request.ownership)) {
      fail(`${at}.ownership must be one of ${OWNERSHIP.join(", ")}`);
    }
    for (const field of ["selected", "evidence"]) {
      if (!Array.isArray(request[field]) || request[field].length === 0 || !request[field].every((ref) => REF_PATTERN.test(ref))) {
        fail(`${at}.${field} must be a non-empty array of path:line references`);
      }
    }
    const tokenIssues = tokenProblems(request.tokens, at);
    problems.push(...tokenIssues.map((issue) => `${where}: ${issue}`));
    if (tokenIssues.length === 0) {
      rowTokens.push(request.tokens);
      if (Array.isArray(request.model_usage)) {
        const modelIssues = request.model_usage.flatMap((row, rowIndex) => tokenProblems(row?.tokens, `${at}.model_usage[${rowIndex}]`));
        problems.push(...modelIssues.map((issue) => `${where}: ${issue}`));
        if (modelIssues.length === 0 && !sameTokens(addTokens(request.model_usage.map((row) => row.tokens)), request.tokens)) {
          fail(`${at}.model_usage does not add up to the request's tokens`);
        }
      }
    }
  });
  const totals = expected.totals;
  if (totals === null || typeof totals !== "object") {
    fail("totals must be an object");
  } else {
    const counts = totals.requests ?? {};
    if (![counts.unique, counts.owned, counts.ambiguous, counts.unknown].every(isCount)) {
      fail("totals.requests needs unique, owned, ambiguous and unknown counts");
    } else {
      if (counts.unique !== requests.length) {
        fail(`totals.requests.unique is ${counts.unique}, but ${requests.length} request rows are listed`);
      }
      if (counts.owned + counts.ambiguous + counts.unknown !== counts.unique) {
        fail("totals.requests owned, ambiguous and unknown must add up to unique");
      }
      for (const status of OWNERSHIP) {
        const rows = requests.filter((request) => request?.ownership === status).length;
        if (rows !== counts[status]) {
          fail(`totals.requests.${status} is ${counts[status]}, but ${rows} request rows are ${status}`);
        }
      }
    }
    const totalIssues = tokenProblems(totals.tokens, "totals");
    problems.push(...totalIssues.map((issue) => `${where}: ${issue}`));
    if (totalIssues.length === 0 && rowTokens.length === requests.length && !sameTokens(addTokens(rowTokens), totals.tokens)) {
      fail(`request rows add up to ${JSON.stringify(addTokens(rowTokens))}, not totals.tokens`);
    }
  }

  // Threads' own usage adds up to the totals too.
  if (Array.isArray(expected.threads) && expected.threads.length > 0 && totals?.tokens) {
    const own = expected.threads.map((thread, index) => {
      const issues = tokenProblems(thread?.own?.tokens, `threads[${index}].own`);
      problems.push(...issues.map((issue) => `${where}: ${issue}`));
      return issues.length === 0 ? thread.own : null;
    });
    if (own.every(Boolean)) {
      const ownedRequests = requests.filter((request) => request?.ownership === "owned").length;
      const threadRequests = own.reduce((sum, item) => sum + (item.requests ?? 0), 0);
      if (threadRequests !== ownedRequests) {
        fail(`threads own ${threadRequests} requests, but ${ownedRequests} request rows are owned`);
      }
      const ownedTokens = addTokens(requests.filter((request) => request?.ownership === "owned").map((request) => request.tokens));
      if (rowTokens.length === requests.length && !sameTokens(addTokens(own.map((item) => item.tokens)), ownedTokens)) {
        fail("threads' own tokens do not add up to the owned request rows");
      }
    }
  }

  expected.naive?.forEach?.((naive, index) => {
    if (typeof naive?.rule !== "string" || naive.rule.trim() === "") {
      fail(`naive[${index}] needs a rule`);
    }
    problems.push(...tokenProblems(naive?.tokens, `naive[${index}]`).map((issue) => `${where}: ${issue}`));
  });
  expected.diagnostics?.forEach?.((diagnostic, index) => {
    if (typeof diagnostic?.code !== "string" || !Array.isArray(diagnostic?.refs)) {
      fail(`diagnostics[${index}] needs a code and refs`);
    }
  });
  return problems;
}

/** Decompress a zstd buffer, or return null when this Node has no zstd support. */
export function decompressZstd(buffer) {
  if (typeof zlib.zstdDecompressSync !== "function") {
    return null;
  }
  return zlib.zstdDecompressSync(buffer);
}

function walkFiles(directory, base = directory) {
  const files = [];
  for (const entry of readdirSync(directory, { withFileTypes: true })) {
    const full = path.join(directory, entry.name);
    if (entry.isDirectory()) {
      files.push(...walkFiles(full, base));
    } else {
      files.push(path.relative(base, full).split(path.sep).join("/"));
    }
  }
  return files.sort();
}

/**
 * Check one case directory. Returns `{ problems, notes, files, lines }`.
 */
export function checkCase(caseDir, { caseName, dialect, identities = [] }) {
  const problems = [];
  const notes = [];
  const prefix = `${dialect}/${caseName}`;
  const all = walkFiles(caseDir);
  for (const required of CASE_FILES) {
    if (!all.includes(required)) {
      problems.push(`${prefix}: missing ${required}`);
    }
  }
  const dataFiles = all.filter((file) => !CASE_FILES.has(file));
  let expected = null;
  if (all.includes("expected.json")) {
    try {
      expected = JSON.parse(readFileSync(path.join(caseDir, "expected.json"), "utf8"));
    } catch (error) {
      problems.push(`${prefix}/expected.json: does not parse (${error.message})`);
    }
  }
  const malformed = new Set(expected?.decode?.malformed ?? []);
  const pendingTail = new Set(expected?.decode?.pending_tail ?? []);
  const lineCounts = new Map();
  let lines = 0;

  const scan = (relPath, text) => {
    for (const finding of scanPrivacy(text, { identities })) {
      problems.push(`${prefix}/${relPath}:${finding.line}: ${finding.why}`);
    }
  };

  for (const relPath of all) {
    const full = path.join(caseDir, relPath);
    if (CASE_FILES.has(relPath)) {
      scan(relPath, readFileSync(full, "utf8"));
      continue;
    }
    if (relPath.endsWith(".jsonl.zst")) {
      const bytes = readFileSync(full);
      if (!bytes.subarray(0, 4).equals(ZSTD_MAGIC)) {
        problems.push(`${prefix}/${relPath}: is not a zstd frame`);
        lineCounts.set(relPath, null);
        continue;
      }
      let text;
      try {
        const decoded = decompressZstd(bytes);
        if (decoded === null) {
          notes.push(`${prefix}/${relPath}: skipped decompression, this Node has no zlib zstd support`);
          lineCounts.set(relPath, null);
          continue;
        }
        text = decoded.toString("utf8");
      } catch (error) {
        problems.push(`${prefix}/${relPath}: does not decompress (${error.message})`);
        lineCounts.set(relPath, null);
        continue;
      }
      problems.push(...checkJsonlText(relPath, text, { malformed, pendingTail }).map((problem) => `${prefix}/${problem}`));
      scan(relPath, text);
      const count = splitLines(text).lines.length;
      lineCounts.set(relPath, count);
      lines += count;
    } else if (relPath.endsWith(".jsonl")) {
      const text = readFileSync(full, "utf8");
      problems.push(...checkJsonlText(relPath, text, { malformed, pendingTail }).map((problem) => `${prefix}/${problem}`));
      scan(relPath, text);
      const count = splitLines(text).lines.length;
      lineCounts.set(relPath, count);
      lines += count;
    } else if (relPath.endsWith(".json")) {
      const text = readFileSync(full, "utf8");
      if (!parsesAsObject(text)) {
        problems.push(`${prefix}/${relPath}: does not parse as a JSON object`);
      }
      scan(relPath, text);
      lineCounts.set(relPath, splitLines(text).lines.length);
    } else {
      problems.push(`${prefix}/${relPath}: unexpected file type; fixtures hold .jsonl, .jsonl.zst and .json data`);
    }
  }
  if (expected !== null) {
    problems.push(...validateExpected(expected, { caseName, dialect, lineCounts }));
  }
  return { problems, notes, files: dataFiles.length, lines };
}

/** Check the whole fixture tree. */
export function checkFixtures(fixtureRoot, { identities = [] } = {}) {
  const problems = [];
  const notes = [];
  const counts = {};
  let files = 0;
  let lines = 0;
  if (!existsSync(fixtureRoot)) {
    return { problems: [`${FIXTURE_DIR} does not exist`], notes, counts, files, lines };
  }
  const readme = path.join(fixtureRoot, "README.md");
  const readmeText = existsSync(readme) ? readFileSync(readme, "utf8") : null;
  if (readmeText === null) {
    problems.push("fixtures: missing README.md with the case table");
  } else {
    problems.push(...scanPrivacy(readmeText, { identities }).map((finding) => `README.md:${finding.line}: ${finding.why}`));
  }
  for (const entry of readdirSync(fixtureRoot, { withFileTypes: true })) {
    if (entry.isFile() && ![".gitattributes", "README.md"].includes(entry.name)) {
      problems.push(`fixtures: unexpected top-level file ${entry.name}`);
    } else if (entry.isDirectory() && !DIALECTS.includes(entry.name)) {
      problems.push(`fixtures: unexpected directory ${entry.name}; dialect directories are ${DIALECTS.join(", ")}`);
    }
  }
  for (const dialect of DIALECTS) {
    const dialectDir = path.join(fixtureRoot, dialect);
    if (!existsSync(dialectDir) || !statSync(dialectDir).isDirectory()) {
      problems.push(`fixtures: missing ${dialect}/`);
      continue;
    }
    const cases = readdirSync(dialectDir, { withFileTypes: true });
    counts[dialect] = 0;
    for (const entry of cases) {
      if (!entry.isDirectory()) {
        problems.push(`${dialect}/${entry.name}: cases are directories`);
        continue;
      }
      counts[dialect] += 1;
      const result = checkCase(path.join(dialectDir, entry.name), { caseName: entry.name, dialect, identities });
      problems.push(...result.problems);
      notes.push(...result.notes);
      files += result.files;
      lines += result.lines;
      if (readmeText !== null && !readmeText.includes(`${dialect}/${entry.name}`)) {
        problems.push(`README.md: the case table does not list ${dialect}/${entry.name}`);
      }
    }
    if (counts[dialect] === 0) {
      problems.push(`${dialect}: has no cases`);
    }
  }
  return { problems, notes, counts, files, lines };
}

function main() {
  const fixtureRoot = path.join(ROOT, FIXTURE_DIR);
  const { problems, notes, counts, files, lines } = checkFixtures(fixtureRoot, { identities: machineIdentities() });
  for (const note of notes) {
    console.log(`note: ${note}`);
  }
  if (problems.length > 0) {
    console.error(`fixtures: ${problems.length} problems in ${FIXTURE_DIR}:\n`);
    for (const problem of problems) {
      console.error(`  ${problem}`);
    }
    console.error("\nFixtures must parse, match their expected.json, and hold only synthetic data (README.md).");
    process.exitCode = 1;
    return;
  }
  const summary = DIALECTS.map((dialect) => `${dialect} ${counts[dialect] ?? 0}`).join(", ");
  console.log(`fixtures ok: ${summary} cases; ${files} data files, ${lines} lines`);
}

if (import.meta.url === pathToFileURL(process.argv[1] ?? "").href) {
  try {
    main();
  } catch (error) {
    console.error(`fixtures: ${error.message}`);
    process.exitCode = 1;
  }
}
