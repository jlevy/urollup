import assert from "node:assert/strict";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

import {
  CONFIG_PATH,
  EXTRACTORS,
  checkE2E,
  compareResults,
  discoverCases,
  formatDifferences,
  isStub,
  normalizeExpected,
  overcountSummary,
  parseArgs,
  validateConfig,
} from "./check-e2e-results.mjs";
import { validateExpected } from "./check-fixtures.mjs";
import { HOME_CANARY_TOKEN } from "./golden-env.mjs";

const ROOT = path.dirname(path.dirname(fileURLToPath(import.meta.url)));
const SAMPLES = path.join(ROOT, "tests", "golden", "samples");
const FIXTURES = path.join(SAMPLES, "fixtures");
const OUTPUTS = path.join(SAMPLES, "outputs");
const EMPTY = "/golden/empty-root";
const COMMITTED = JSON.parse(readFileSync(path.join(ROOT, CONFIG_PATH), "utf8"));

const readJson = (...parts) => JSON.parse(readFileSync(path.join(...parts), "utf8"));

/** The smallest expected.json the checker accepts: the fixture record's format and totals. */
const record = (overrides = {}) =>
  JSON.stringify({
    format: "urollup-fixture-expected/v1",
    totals: { requests: { unique: 1, owned: 1, ambiguous: 0, unknown: 0 }, tokens: { uncached_input: 1, cache_read: 0, cache_write: 0, output: 1, reasoning: null } },
    ...overrides,
  });
const expectedFor = (id) => normalizeExpected(readJson(FIXTURES, ...id.split("/"), "expected.json"));
const outputFor = (id, name) => readJson(OUTPUTS, ...id.split("/"), `${name}.json`);

/** The committed config with every pending entry removed, as it will be once commands exist. */
function implementedConfig(overrides = {}) {
  const { pendingFixtures, ...config } = structuredClone(COMMITTED);
  config.commands = config.commands.map(({ pending, ...command }) => command);
  return { ...config, ...overrides };
}

/** Every command and the corpus pending, as on the scaffold; independent of the committed state. */
function pendingConfig() {
  const config = implementedConfig();
  config.pendingFixtures = { bead: "uro-obx5", unblock: "the corpus lands" };
  config.commands = config.commands.map((command) => ({ ...command, pending: { bead: "uro-d135", unblock: `urollup ${command.name} prints JSON` } }));
  return config;
}

function stubResult(name) {
  return { status: 2, stdout: "", stderr: `error: \`urollup ${name}\` is not implemented yet; this build is the repository scaffold\n` };
}

/**
 * A stand-in for the built binary. Implemented commands print the sample output for the
 * case whose directory the discovery variables name; `output` can substitute a file name.
 */
function fakeUrollup({ implemented = ["report", "daily", "sessions"], output = (name) => name, transform = (stdout) => stdout } = {}) {
  const calls = [];
  const run = (args, env) => {
    calls.push({ args, env });
    const [name] = args;
    if (!implemented.includes(name)) {
      return stubResult(name);
    }
    if (args.length === 1) {
      return { status: 2, stdout: "", stderr: "error: no current session detected; pass --session, --latest or --all\n" };
    }
    const caseDir = env.CLAUDE_CONFIG_DIR !== EMPTY ? env.CLAUDE_CONFIG_DIR : env.CODEX_HOME;
    const file = path.join(OUTPUTS, path.relative(FIXTURES, caseDir), `${output(name)}.json`);
    if (!existsSync(file)) {
      return { status: 1, stdout: "", stderr: `error: no sample output ${file}\n` };
    }
    return { status: 0, stdout: transform(readFileSync(file, "utf8"), calls.length), stderr: "" };
  };
  run.calls = calls;
  return run;
}

function check(options) {
  return checkE2E({ config: implementedConfig(), fixturesRoot: FIXTURES, runUrollup: fakeUrollup(), emptyRoot: EMPTY, local: true, ...options });
}

function temporaryCorpus(t, files) {
  const root = mkdtempSync(path.join(tmpdir(), "e2e-results-test-"));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  for (const [file, content] of Object.entries(files)) {
    mkdirSync(path.join(root, path.dirname(file)), { recursive: true });
    writeFileSync(path.join(root, file), content);
  }
  return root;
}

test("the committed configuration is valid and checks every milestone 0.1 command", () => {
  assert.equal(validateConfig(COMMITTED), COMMITTED);
  const without = (name) => ({ ...COMMITTED, commands: COMMITTED.commands.filter((command) => command.name !== name) });
  assert.throws(() => validateConfig(without("sessions")), /must check sessions/);
  const mutate = (change) => ({ ...COMMITTED, commands: COMMITTED.commands.map((command) => (command.name === "daily" ? { ...command, ...change } : command)) });
  assert.throws(() => validateConfig(mutate({ args: ["daily", "--all", "--format", "json"] })), /must fix --timezone/);
  assert.throws(() => validateConfig(mutate({ pending: { bead: "uro-x" } })), /bead and an unblock/);
  assert.throws(() => validateConfig(mutate({ compare: ["tokens", "cost"] })), /compare must be/);
  assert.throws(() => validateConfig(mutate({ name: "weekly", args: ["weekly", "--timezone", "UTC"] })), /no result extractor|must check daily/);
});

test("cases are discovered generically, including derived cases with subagent metadata", () => {
  const cases = discoverCases(FIXTURES);
  assert.deepEqual(
    cases.map(({ id, roots }) => [id, roots]),
    [
      ["claude-project/block-records", { claude: true, codex: false }],
      ["claude-project/subagent-derived", { claude: true, codex: false }],
      ["codex-rollout/archived-rollout", { claude: false, codex: true }],
      ["codex-rollout/cumulative-repeat", { claude: false, codex: true }],
    ],
  );
});

test("every sample is the same record the frozen corpus uses, by the fixtures checker's own rules", () => {
  const dataFiles = (dir, prefix = "") =>
    readdirSync(path.join(dir, prefix), { withFileTypes: true }).flatMap((entry) => {
      const relative = prefix ? `${prefix}/${entry.name}` : entry.name;
      if (entry.isDirectory()) {
        return dataFiles(dir, relative);
      }
      return entry.name === "expected.json" || entry.name === "README.md" ? [] : [relative];
    });
  const lines = (file) => readFileSync(file, "utf8").split("\n").filter((line, index, all) => line !== "" || index < all.length - 1).length;
  for (const { id, dir } of discoverCases(FIXTURES)) {
    const [dialect, caseName] = id.split("/");
    const lineCounts = new Map(dataFiles(dir).map((file) => [file, lines(path.join(dir, file))]));
    assert.deepEqual(validateExpected(readJson(dir, "expected.json"), { caseName, dialect, lineCounts }), [], id);
  }
});

test("a case nested inside another case is refused", (t) => {
  const root = temporaryCorpus(t, { "a/expected.json": "{}", "a/projects/b/expected.json": "{}" });
  assert.throws(() => discoverCases(root), /a\/projects\/b is nested inside case a/);
});

test("results are read out of the fixture record: totals, and the lists whose length is a count", () => {
  const claude = expectedFor("claude-project/block-records");
  assert.deepEqual(Object.keys(claude.reconciled).sort(), ["copies_excluded", "diagnostics", "limit_observations", "ownership", "requests", "tokens", "unresolved"]);
  assert.equal(claude.reconciled.requests, 1, "the count is totals.requests.unique, not the request rows themselves");
  assert.deepEqual(claude.reconciled.ownership, { owned: 1, ambiguous: 0, unknown: 0 });
  assert.equal(claude.reconciled.copies_excluded, 3, "three copies are listed");
  assert.equal(claude.reconciled.limit_observations, 0);
  assert.deepEqual(claude.reconciled.diagnostics, [{ code: "sample.block_usage_differs", count: 1 }], "a diagnostic's count is how many refs it fired at");
  assert.equal(claude.naive[0].requests, 4);

  const codex = expectedFor("codex-rollout/cumulative-repeat");
  assert.equal(codex.reconciled.requests, 2);
  assert.equal(codex.reconciled.limit_observations, 1);

  // A token category the dialect does not report is null, and compares as absent or zero.
  const unreported = normalizeExpected(JSON.parse(record({ totals: { requests: { unique: 1, owned: 1, ambiguous: 0, unknown: 0 }, tokens: { output: 5, reasoning: null } } })));
  assert.deepEqual(unreported.reconciled.tokens, { output: 5, reasoning: null });
  assert.deepEqual(compareResults(unreported.reconciled, { tokens: { output: 5 } }, ["tokens"]).differences, []);
  assert.deepEqual(compareResults(unreported.reconciled, { tokens: { output: 5, reasoning: 0 } }, ["tokens"]).differences, []);
  assert.deepEqual(compareResults(unreported.reconciled, { tokens: { output: 5, reasoning: 7 } }, ["tokens"]).differences, [{ field: "tokens.reasoning", expected: "absent or 0", actual: 7 }]);
});

test("an expected.json in another shape, or one that would check nothing, is refused", () => {
  const parsed = (overrides) => JSON.parse(record(overrides));
  assert.throws(() => normalizeExpected({ reconciled: { requests: 1 } }), /format must be urollup-fixture-expected\/v1/);
  assert.throws(() => normalizeExpected(parsed({ totals: { notes: "nothing countable" } })), /states no results the checker compares/);
  assert.throws(() => normalizeExpected(parsed({ totals: { requests: { unique: -1 } } })), /non-negative integer/);
  assert.throws(() => normalizeExpected(parsed({ totals: { requests: { unique: 1 }, tokens: { output: "600" } } })), /tokens\.output must be/);
  assert.throws(() => normalizeExpected(parsed({ diagnostics: [{ detail: "no code" }] })), /string code/);
  assert.throws(() => normalizeExpected(parsed({ naive: { requests: 4 } })), /naive must be a list/);
  assert.throws(() => normalizeExpected([]), /JSON object/);
});

test("reconciled output matches its expected results exactly", () => {
  const expected = expectedFor("claude-project/block-records").reconciled;
  const { compared, differences } = compareResults(expected, EXTRACTORS.report(outputFor("claude-project/block-records", "report")));
  assert.equal(compared.length, 7);
  assert.deepEqual(differences, []);
});

test("a naive-sum report fails with a field-by-field diff", () => {
  const expected = expectedFor("claude-project/block-records").reconciled;
  const { differences } = compareResults(expected, EXTRACTORS.report(outputFor("claude-project/block-records", "report.naive")));
  assert.deepEqual(differences, [
    { field: "requests", expected: 1, actual: 4 },
    { field: "ownership.owned", expected: 1, actual: 4 },
    { field: "tokens.uncached_input", expected: 3, actual: 12 },
    { field: "tokens.cache_read", expected: 40000, actual: 160000 },
    { field: "tokens.output", expected: 600, actual: 1224 },
    { field: "copies_excluded", expected: 3, actual: 0 },
    { field: "diagnostics", expected: "sample.block_usage_differs x1", actual: "missing" },
    { field: "diagnostics", expected: "none", actual: "unexpected sample.unexpected_note x2" },
  ]);
  const table = formatDifferences(differences);
  assert.match(table[0], /^field\s+expected\s+actual$/);
  assert.match(table.find((line) => line.startsWith("tokens.cache_read")), /40,000\s+160,000$/);
});

test("absent fields and count mismatches in diagnostics are differences, never skipped", () => {
  const { differences } = compareResults({ requests: 1, tokens: { output: 5 }, diagnostics: [{ code: "a", count: 2 }] }, { diagnostics: [{ code: "a", count: 1 }] });
  assert.deepEqual(differences, [
    { field: "requests", expected: 1, actual: "absent" },
    { field: "tokens", expected: "an object", actual: "absent" },
    { field: "diagnostics a", expected: 2, actual: 1 },
  ]);
});

test("daily and sessions rows sum to the same reconciled truth as the report", () => {
  const expected = expectedFor("claude-project/block-records").reconciled;
  for (const name of ["daily", "sessions"]) {
    const { compared, differences } = compareResults(expected, EXTRACTORS[name](outputFor("claude-project/block-records", name)), ["requests", "ownership", "tokens"]);
    assert.deepEqual(compared, ["requests", "ownership", "tokens"], name);
    assert.deepEqual(differences, [], name);
  }
  assert.deepEqual(EXTRACTORS.daily({ no: "rows" }), {});
});

test("the naive-sum overcount is summarized for context", () => {
  const { reconciled, naive } = expectedFor("claude-project/block-records");
  assert.equal(
    overcountSummary(reconciled, naive),
    'naive rule "Sum message.usage over every assistant record": 4 requests (4.0x the reconciled 1), 161,236 tokens (4.0x the reconciled 40,603)',
  );
  const codex = expectedFor("codex-rollout/cumulative-repeat");
  assert.match(overcountSummary(codex.reconciled, codex.naive), /^naive rule "Add every total_token_usage snapshot.*: 3 requests \(1\.5x the reconciled 2\), 60,000 tokens \(2\.4x the reconciled 25,000\)$/);
  assert.equal(overcountSummary(reconciled, undefined), "no naive sum recorded");

  // With several rules, the one furthest from the truth is the one worth printing.
  const rules = [
    { rule: "close", requests: 1, tokens: { output: 40700 } },
    { rule: "a strict reader that drops the whole file", requests: 0, tokens: { uncached_input: 0, cache_read: 0, output: 0 } },
  ];
  assert.match(overcountSummary(reconciled, rules), /^naive rule "a strict reader that drops the whole file": 0 requests \(0\.0x the reconciled 1\), 0 tokens \(0\.0x the reconciled 40,603\), worst of 2 rules$/);
});

test("only the scaffold's exit 2 with its diagnostic and no stdout is a stub", () => {
  assert.equal(isStub(stubResult("report")), true);
  assert.equal(isStub({ status: 2, stdout: "", stderr: "error: unexpected argument '--all' found\n" }), false);
  assert.equal(isStub({ ...stubResult("report"), status: 1 }), false);
  assert.equal(isStub({ ...stubResult("report"), stdout: "{}" }), false);
});

test("while commands are stubs and fixtures are absent, the run is visibly PENDING and checks nothing", (t) => {
  const root = temporaryCorpus(t, {});
  const outcome = checkE2E({ config: pendingConfig(), fixturesRoot: path.join(root, "absent"), goldenSessions: [], runUrollup: fakeUrollup({ implemented: [] }), emptyRoot: EMPTY });
  assert.equal(outcome.status, "pending");
  assert.equal(outcome.checked, 0);
  const text = outcome.lines.join("\n");
  assert.match(text, /PENDING urollup report is not implemented yet \(uro-d135/);
  assert.match(text, /PENDING no fixture corpus at crates\/urollup-core\/tests\/fixtures yet \(uro-obx5/);
  assert.match(outcome.lines.at(-1), /^e2e-results: PENDING: results checked for 0 of 0 fixture cases/);
});

test("while commands are stubs, every expected.json is still validated", (t) => {
  const root = temporaryCorpus(t, {
    "claude-project/good/expected.json": record(),
    "claude-project/good/projects/p/s.jsonl": "{}\n",
    "claude-project/bad/expected.json": JSON.stringify({ reconciled: { requests: 1 } }),
    "claude-project/bad/projects/p/s.jsonl": "{}\n",
    "codex-rollout/no-root/expected.json": record(),
    "codex-rollout/no-root/rollout.jsonl": "{}\n",
  });
  const outcome = checkE2E({ config: pendingConfig(), fixturesRoot: root, runUrollup: fakeUrollup({ implemented: [] }), emptyRoot: EMPTY, local: true });
  assert.equal(outcome.status, "fail");
  const text = outcome.lines.join("\n");
  assert.match(text, /FAIL    claude-project\/bad\/expected\.json: format must be urollup-fixture-expected\/v1/);
  assert.match(text, /FAIL    codex-rollout\/no-root has neither projects\/ \(Claude Code\) nor sessions\//);
  assert.doesNotMatch(text, /good/);
});

test("pending entries are ratchets: a stale entry, a missing entry or a missing corpus fails", (t) => {
  const stubs = fakeUrollup({ implemented: [] });
  const implemented = fakeUrollup();
  const empty = temporaryCorpus(t, { "README.md": "no cases yet\n" });

  const staleCommand = checkE2E({ config: pendingConfig(), fixturesRoot: FIXTURES, runUrollup: implemented, emptyRoot: EMPTY, local: true });
  assert.match(staleCommand.lines.join("\n"), /FAIL    urollup report is implemented now; delete its pending entry/);

  const unlistedStub = checkE2E({ config: implementedConfig(), fixturesRoot: FIXTURES, runUrollup: stubs, emptyRoot: EMPTY, local: true });
  assert.match(unlistedStub.lines.join("\n"), /FAIL    urollup daily is still the scaffold stub, but tests\/golden\/e2e\.config\.json lists it as implemented/);

  const staleFixtures = checkE2E({ config: pendingConfig(), fixturesRoot: empty, goldenSessions: [], runUrollup: stubs, emptyRoot: EMPTY });
  assert.match(staleFixtures.lines.join("\n"), /FAIL    fixtures exist at crates\/urollup-core\/tests\/fixtures; delete pendingFixtures/);
  assert.match(staleFixtures.lines.join("\n"), /FAIL    no fixture cases \(directories with expected\.json\)/);

  const missingCorpus = checkE2E({ config: implementedConfig(), fixturesRoot: path.join(empty, "absent"), goldenSessions: [], runUrollup: implemented, emptyRoot: EMPTY });
  assert.match(missingCorpus.lines.join("\n"), /FAIL    the fixture corpus crates\/urollup-core\/tests\/fixtures is missing/);

  for (const outcome of [staleCommand, unlistedStub, staleFixtures, missingCorpus]) {
    assert.equal(outcome.status, "fail");
  }
});

test("once commands exist, matching cases pass with their overcount, from the case's roots only", () => {
  const run = fakeUrollup();
  const outcome = check({ runUrollup: run, caseFilter: "block-records" });
  assert.equal(outcome.status, "ok", outcome.lines.join("\n"));
  assert.equal(outcome.checked, 1);
  assert.match(outcome.lines.join("\n"), /^ok      claude-project\/block-records: report, daily, sessions match; naive rule "Sum message\.usage over every assistant record": 4 requests \(4\.0x/m);
  const reportCall = run.calls.find(({ args }) => args[0] === "report" && args.length > 1);
  assert.deepEqual(reportCall.args, ["report", "--all", "--format", "json", "--timezone", "UTC"]);
  assert.equal(reportCall.env.CLAUDE_CONFIG_DIR, path.join(FIXTURES, "claude-project", "block-records"));
  assert.equal(reportCall.env.CODEX_HOME, EMPTY);
  // Two runs of every command, for determinism.
  assert.equal(run.calls.filter(({ args }) => args.length > 1).length, 6);
});

test("a disagreeing report fails with its diff and the naive sum for context", () => {
  const outcome = check({ runUrollup: fakeUrollup({ output: (name) => (name === "report" ? "report.naive" : name) }), caseFilter: "block-records" });
  assert.equal(outcome.status, "fail");
  const text = outcome.lines.join("\n");
  assert.match(text, /FAIL    claude-project\/block-records/);
  assert.match(text, /urollup report --all --format json --timezone UTC disagrees with claude-project\/block-records\/expected\.json:/);
  assert.match(text, /tokens\.cache_read\s+40,000\s+160,000/);
  assert.match(text, /for context, naive rule "Sum message\.usage over every assistant record": 4 requests \(4\.0x the reconciled 1\)/);
});

test("nondeterministic output, canary output, a failing exit and non-JSON output each fail", () => {
  const cases = [
    [{ transform: (stdout, call) => (call % 2 === 0 ? stdout.replace('"schema_version": 1', '"schema_version": 1 ') : stdout) }, /is not deterministic: two identical runs differ at stdout line 2/],
    [{ transform: (stdout) => stdout.replace("sample.block_usage_differs", HOME_CANARY_TOKEN) }, /read the hermetic HOME's canary logs/],
    [{ output: (name) => `${name}.absent` }, /exited 1\n.*stderr: error: no sample output/],
    [{ transform: () => "report table\n" }, /did not print JSON/],
  ];
  for (const [options, pattern] of cases) {
    const outcome = check({ runUrollup: fakeUrollup(options), caseFilter: "block-records" });
    assert.equal(outcome.status, "fail", String(pattern));
    assert.match(outcome.lines.join("\n"), pattern);
  }
});

test("a case filter that matches nothing fails rather than checking zero cases", () => {
  const outcome = check({ caseFilter: "no-such-case" });
  assert.equal(outcome.status, "fail");
  assert.match(outcome.lines.join("\n"), /--case no-such-case matches no fixture case/);
});

test("every case needs its transcript golden once report exists, and a golden needs its case", () => {
  const config = implementedConfig();
  const sessions = ["tests/golden/cli-surface.tryscript.md", "tests/golden/e2e/claude-project/block-records.tryscript.md", "tests/golden/e2e/claude-project/retired-case.tryscript.md"];
  const run = fakeUrollup({ implemented: [] });
  const withStubs = checkE2E({ config: { ...pendingConfig(), pendingFixtures: undefined }, fixturesRoot: FIXTURES, goldenSessions: sessions, runUrollup: run, emptyRoot: EMPTY });
  assert.match(withStubs.lines.join("\n"), /PENDING 3 fixture cases have no transcript golden yet/);
  assert.match(withStubs.lines.join("\n"), /FAIL    tests\/golden\/e2e\/claude-project\/retired-case\.tryscript\.md has no fixture case/);

  const implemented = checkE2E({ config, fixturesRoot: FIXTURES, goldenSessions: sessions, runUrollup: fakeUrollup({ implemented: ["report", "daily", "sessions"] }), emptyRoot: EMPTY });
  const text = implemented.lines.join("\n");
  assert.match(text, /FAIL    3 fixture cases have no transcript golden; create each with node scripts\/new-e2e-golden\.mjs <case>/);
  assert.match(text, /tests\/golden\/e2e\/codex-rollout\/cumulative-repeat\.tryscript\.md/);
});

test("command-line options are the case filter and a local corpus, nothing else", () => {
  assert.deepEqual(parseArgs(["--case", "codex", "--fixtures", "/tmp/corpus"]), { case: "codex", fixtures: "/tmp/corpus" });
  assert.throws(() => parseArgs(["--update"]), /usage/);
});
