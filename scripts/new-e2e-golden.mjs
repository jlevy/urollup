#!/usr/bin/env node
// Scaffold the transcript golden for one fixture case.
//
// The mapping is fixed: the case crates/urollup-core/tests/fixtures/<dialect>/<case>/ gets
// tests/golden/e2e/<dialect>/<case>.tryscript.md, and scripts/check-e2e-results.mjs fails
// once `urollup report` exists if any case lacks one. The scaffold runs report, daily and
// sessions in table and JSON form against a sandbox copy of the case, through the case's
// native discovery variable, with unknown wildcards for the output. Fill them in with
// `node scripts/run-golden.mjs --expand <golden>`, read every line, and commit; the golden
// lint refuses a session that still holds `???`.
//
// Usage: node scripts/new-e2e-golden.mjs <dialect>/<case>

import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

import { CONFIG_PATH, inferRoots } from "./check-e2e-results.mjs";

const ROOT = path.dirname(path.dirname(fileURLToPath(import.meta.url)));

/** The views every case records: each command, as a table and as JSON. */
export const VIEWS = [
  ["Report", "report --all --timezone UTC"],
  ["Report JSON", "report --all --format json --timezone UTC"],
  ["Daily", "daily --all --timezone UTC"],
  ["Daily JSON", "daily --all --format json --timezone UTC"],
  ["Sessions", "sessions --all --timezone UTC"],
  ["Sessions JSON", "sessions --all --format json --timezone UTC"],
];

/** The golden path for a case id, relative to the repository root. */
export function goldenPath(config, caseId) {
  return `${config.goldenRoot}/${caseId}.tryscript.md`;
}

/** Render the scaffold session for `caseId` laid out as `roots`. */
export function renderGolden(config, caseId, roots) {
  const golden = goldenPath(config, caseId);
  const up = "../".repeat(golden.split("/").length - 1);
  const caseDir = `${up}${config.fixturesRoot}/${caseId}`;
  const env = [
    `  CLAUDE_CONFIG_DIR: ${roots.claude ? "." : "$GOLDEN_EMPTY_ROOT"}`,
    `  CODEX_HOME: ${roots.codex ? "." : "$GOLDEN_EMPTY_ROOT"}`,
    "  PI_CODING_AGENT_SESSION_DIR: $GOLDEN_EMPTY_ROOT/sessions",
  ];
  const variable = [roots.claude && "`CLAUDE_CONFIG_DIR`", roots.codex && "`CODEX_HOME`"].filter(Boolean).join(" and ");
  const blocks = VIEWS.flatMap(([title, args]) => [`## ${title}`, "", "```console", `$ urollup ${args}`, "???", "```", ""]);
  return [
    "---",
    `sandbox: ${caseDir}`,
    "path:",
    "  - $UROLLUP_BIN",
    "env:",
    ...env,
    "---",
    `# E2E: ${caseId}`,
    "",
    `The fixture case [\`${caseId}\`](${caseDir}/) read through ${variable} from a sandbox`,
    "copy, with every other discovery root empty and HOME hermetic.",
    "`make e2e-results` checks its reconciled results against `expected.json`; this session",
    "records the complete output of each view for review.",
    "",
    ...blocks,
    "<!-- This document follows common-doc-guidelines.md.",
    "See github.com/jlevy/practical-prose and review guidelines before editing.",
    "-->",
    "",
  ].join("\n");
}

function main() {
  const [caseId, ...rest] = process.argv.slice(2);
  if (!caseId || rest.length > 0 || caseId.includes("\\") || caseId.split("/").includes("..")) {
    throw new Error("usage: new-e2e-golden.mjs <dialect>/<case>");
  }
  const config = JSON.parse(readFileSync(path.join(ROOT, CONFIG_PATH), "utf8"));
  const caseDir = path.join(ROOT, config.fixturesRoot, ...caseId.split("/"));
  if (!existsSync(path.join(caseDir, "expected.json"))) {
    throw new Error(`${config.fixturesRoot}/${caseId} is not a fixture case (no expected.json)`);
  }
  const roots = inferRoots(caseDir);
  if (!roots.claude && !roots.codex) {
    throw new Error(`${config.fixturesRoot}/${caseId} has neither projects/ nor sessions/ or archived_sessions/`);
  }
  const target = path.join(ROOT, ...goldenPath(config, caseId).split("/"));
  if (existsSync(target)) {
    throw new Error(`${goldenPath(config, caseId)} exists; edit it or regenerate its output with run-golden.mjs --update`);
  }
  mkdirSync(path.dirname(target), { recursive: true });
  writeFileSync(target, renderGolden(config, caseId, roots));
  console.log(`wrote ${goldenPath(config, caseId)}; next:`);
  console.log(`  node scripts/run-golden.mjs --expand ${goldenPath(config, caseId)}`);
  console.log("  then read every expanded line and fix exit codes before committing");
}

if (import.meta.url === pathToFileURL(process.argv[1] ?? "").href) {
  try {
    main();
  } catch (error) {
    console.error(`new-e2e-golden: ${error.message}`);
    process.exitCode = 2;
  }
}
