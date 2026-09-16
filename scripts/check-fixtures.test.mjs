import assert from "node:assert/strict";
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";
import zlib from "node:zlib";

import {
  EXPECTED_FORMAT,
  FIXTURE_DIR,
  checkCase,
  checkFixtures,
  checkJsonlText,
  collectRefs,
  scanPrivacy,
  splitLines,
  validateExpected,
} from "./check-fixtures.mjs";

const ROOT = dirname(dirname(fileURLToPath(import.meta.url)));
const ZERO = { uncached_input: 0, cache_read: 0, cache_write: 0, output: 0, reasoning: null };

function tokens(overrides = {}) {
  return { ...ZERO, ...overrides };
}

function expectedFor(caseName, dialect, overrides = {}) {
  return {
    format: EXPECTED_FORMAT,
    case: caseName,
    dialect,
    summary: "A synthetic case.",
    rules: ["design §3.4"],
    agent: { name: "Claude Code", versions: "2.1.x" },
    files: [{ path: "s.jsonl", lines: 2 }],
    decode: { malformed: [], pending_tail: [] },
    threads: [{ id: "s", own: { requests: 1, tokens: tokens({ output: 5 }) } }],
    requests: [
      {
        key: { message_id: "msg_1" },
        ownership: "owned",
        owner_thread: "s",
        tokens: tokens({ output: 5 }),
        selected: ["s.jsonl:2"],
        evidence: ["s.jsonl:1", "s.jsonl:2"],
      },
    ],
    copies: [],
    totals: { requests: { unique: 1, owned: 1, ambiguous: 0, unknown: 0 }, tokens: tokens({ output: 5 }) },
    diagnostics: [],
    limit_observations: [],
    naive: [{ rule: "sum every record", requests: 2, tokens: tokens({ output: 9 }) }],
    ...overrides,
  };
}

function writeCase(root, dialect, caseName, files) {
  const dir = join(root, dialect, caseName);
  for (const [name, content] of Object.entries(files)) {
    mkdirSync(dirname(join(dir, name)), { recursive: true });
    writeFileSync(join(dir, name), content);
  }
  return dir;
}

test("home directories must be placeholders, including Claude Code's encoded project names", () => {
  assert.deepEqual(scanPrivacy('{"cwd":"/Users/example/project"}'), []);
  assert.deepEqual(scanPrivacy("projects/-Users-example-project/s.jsonl"), []);
  assert.match(scanPrivacy('{"cwd":"/Users/jdoe/work"}')[0].why, /home directory/);
  assert.match(scanPrivacy('{"cwd":"/home/jdoe"}')[0].why, /home directory/);
  assert.match(scanPrivacy("C:\\Users\\jdoe\\work")[0].why, /home directory/);
  assert.match(scanPrivacy("projects/-Users-jdoe-work/s.jsonl")[0].why, /encoded home directory/);
});

test("emails and credential-shaped tokens are findings, placeholder domains are not", () => {
  assert.deepEqual(scanPrivacy("contact: someone@example.com"), []);
  assert.match(scanPrivacy("contact: jdoe@corp.test.io")[0].why, /email/);
  assert.match(scanPrivacy("key sk-ant-api03-abcdefghijklmnopqrstuvwxyz0123")[0].why, /credential/);
  assert.match(scanPrivacy("token ghp_abcdefghijklmnopqrstuvwxyz0123456789")[0].why, /credential/);
  assert.match(scanPrivacy("AKIAABCDEFGHIJKLMNOP")[0].why, /credential/);
  assert.match(scanPrivacy("-----BEGIN RSA PRIVATE KEY-----")[0].why, /credential/);
  // Synthetic native IDs are not credentials.
  assert.deepEqual(scanPrivacy('{"id":"msg_01Synthetic0001","requestId":"req_011Synthetic0001","response_id":"resp_synthetic_0001"}'), []);
});

test("the running machine's account name is a finding as a whole word only", () => {
  const found = scanPrivacy('{"note":"written by jdoe"}', { identities: ["jdoe"] });
  assert.equal(found.length, 1);
  assert.match(found[0].why, /account or home directory name/);
  assert.deepEqual(scanPrivacy('{"note":"jdoes and xjdoe"}', { identities: ["jdoe"] }), []);
  assert.equal(scanPrivacy("line one\nby jdoe", { identities: ["jdoe"] })[0].line, 2);
});

test("lines split with a pending tail only when the last newline is missing", () => {
  assert.deepEqual(splitLines("a\nb\n"), { lines: ["a", "b"], tail: false });
  assert.deepEqual(splitLines("a\nb"), { lines: ["a", "b"], tail: true });
  assert.deepEqual(splitLines(""), { lines: [], tail: false });
});

test("undeclared malformed lines and tails fail; declared ones must really be broken", () => {
  assert.deepEqual(checkJsonlText("s.jsonl", '{"a":1}\n{"b":2}\n'), []);
  assert.match(checkJsonlText("s.jsonl", '{"a":1}\n{broken\n')[0], /s\.jsonl:2: does not parse/);
  assert.match(checkJsonlText("s.jsonl", '{"a":1}\n[1,2]\n')[0], /does not parse as a JSON object/);
  assert.match(checkJsonlText("s.jsonl", '{"a":1}\n{"b":2}')[0], /does not end with a newline/);

  const declared = { malformed: new Set(["s.jsonl:2"]), pendingTail: new Set(["s.jsonl:4"]) };
  assert.deepEqual(checkJsonlText("s.jsonl", '{"a":1}\n{broken\n{"c":3}\n{"d":', declared), []);
  assert.match(checkJsonlText("s.jsonl", '{"a":1}\n{"b":2}\n{"c":3}\n{"d":', declared)[0], /declared malformed, but it parses/);
  assert.match(checkJsonlText("s.jsonl", '{"a":1}\n{broken\n{"c":3}\n{"d":4}\n', declared)[0], /not an unterminated last line/);
  assert.match(
    checkJsonlText("s.jsonl", '{"a":1}\n{"b":2}\n', { malformed: new Set(["s.jsonl:9"]) })[0],
    /past the end of the file/,
  );
});

test("references are collected from anywhere in expected.json", () => {
  assert.deepEqual(
    collectRefs({ a: ["projects/p/s.jsonl:3", "not a ref"], b: { c: "sessions/2026/09/01/r.jsonl.zst:12", d: "x.json:1" } }),
    ["projects/p/s.jsonl:3", "sessions/2026/09/01/r.jsonl.zst:12", "x.json:1"],
  );
});

test("expected.json must match its case, list its files and add up", () => {
  const lineCounts = new Map([["s.jsonl", 2]]);
  const context = { caseName: "c", dialect: "claude-project", lineCounts };
  assert.deepEqual(validateExpected(expectedFor("c", "claude-project"), context), []);

  const problems = (overrides) => validateExpected(expectedFor("c", "claude-project", overrides), context).join("\n");
  assert.match(problems({ case: "other" }), /case must be "c"/);
  assert.match(problems({ dialect: "codex-rollout" }), /dialect must be "claude-project"/);
  assert.match(problems({ files: [{ path: "s.jsonl", lines: 3 }] }), /has 3 lines, but it has 2/);
  assert.match(problems({ files: [{ path: "gone.jsonl" }] }), /does not exist in the case/);
  assert.match(problems({ files: [{ path: "gone.jsonl" }] }), /s\.jsonl is in the case but not listed/);
  assert.match(problems({ copies: [{ ref: "s.jsonl:7" }] }), /s\.jsonl:7 is past the end/);
  assert.match(problems({ diagnostics: [{ code: "x", refs: ["other.jsonl:1"] }] }), /names a file that is not in the case/);
  assert.match(
    problems({ totals: { requests: { unique: 1, owned: 1, ambiguous: 0, unknown: 0 }, tokens: tokens({ output: 6 }) } }),
    /request rows add up to .* not totals\.tokens/,
  );
  assert.match(
    problems({ totals: { requests: { unique: 2, owned: 2, ambiguous: 0, unknown: 0 }, tokens: tokens({ output: 5 }) } }),
    /unique is 2, but 1 request rows/,
  );
  assert.match(problems({ threads: [{ id: "s", own: { requests: 1, tokens: tokens({ output: 4 }) } }] }), /threads' own tokens/);
  assert.match(problems({ threads: [{ id: "s", own: { requests: 2, tokens: tokens({ output: 5 }) } }] }), /threads own 2 requests, but 1 request rows are owned/);
  assert.match(problems({ naive: [] }), /naive must list/);
  assert.match(problems({ requests: [{ ...expectedFor("c", "claude-project").requests[0], tokens: tokens({ reasoning: -1 }) }] }), /reasoning must be/);
  const withModels = expectedFor("c", "claude-project").requests[0];
  assert.match(
    problems({ requests: [{ ...withModels, model_usage: [{ model: "a", tokens: tokens({ output: 2 }) }] }] }),
    /model_usage does not add up/,
  );
});

test("a case is checked end to end, including zstd members", () => {
  const scratch = mkdtempSync(join(tmpdir(), "urollup-check-fixtures-test-"));
  try {
    const rollout = '{"timestamp":"2026-09-01T00:00:00.000Z","type":"session_meta","payload":{"id":"t"}}\n{"timestamp":"2026-09-01T00:00:01.000Z","type":"event_msg","payload":{"type":"task_started"}}\n';
    const expected = expectedFor("zst", "codex-rollout", {
      files: [{ path: "sessions/r.jsonl.zst", lines: 2 }],
      requests: [{ ...expectedFor("zst", "codex-rollout").requests[0], selected: ["sessions/r.jsonl.zst:2"], evidence: ["sessions/r.jsonl.zst:2"] }],
    });
    const zst = typeof zlib.zstdCompressSync === "function" ? zlib.zstdCompressSync(Buffer.from(rollout)) : null;
    const dir = writeCase(scratch, "codex-rollout", "zst", {
      "README.md": "# zst\n",
      "expected.json": JSON.stringify(expected),
      ...(zst ? { "sessions/r.jsonl.zst": zst } : {}),
    });
    if (zst) {
      const result = checkCase(dir, { caseName: "zst", dialect: "codex-rollout" });
      assert.deepEqual(result.problems, []);
      assert.equal(result.lines, 2);
    }

    writeFileSync(join(dir, "sessions", "bad.jsonl.zst"), "not zstd");
    writeFileSync(join(dir, "notes.txt"), "stray");
    const problems = checkCase(dir, { caseName: "zst", dialect: "codex-rollout" }).problems.join("\n");
    assert.match(problems, /bad\.jsonl\.zst: is not a zstd frame/);
    assert.match(problems, /notes\.txt: unexpected file type/);
  } finally {
    rmSync(scratch, { recursive: true, force: true });
  }
});

test("private data anywhere in a case fails it, and a missing README row fails the tree", () => {
  const scratch = mkdtempSync(join(tmpdir(), "urollup-check-fixtures-test-"));
  try {
    writeFileSync(join(scratch, "README.md"), "| claude-project/c |\n");
    writeCase(scratch, "claude-project", "c", {
      "README.md": "# c\n",
      "expected.json": JSON.stringify(expectedFor("c", "claude-project")),
      "s.jsonl": '{"cwd":"/Users/example/project"}\n{"cwd":"/Users/jdoe/project"}\n',
    });
    writeCase(scratch, "codex-rollout", "r", {
      "README.md": "# r\n",
      "expected.json": JSON.stringify(expectedFor("r", "codex-rollout")),
      "s.jsonl": '{"a":1}\n{"b":2}\n',
    });
    const { problems, counts } = checkFixtures(scratch);
    const text = problems.join("\n");
    assert.deepEqual(counts, { "claude-project": 1, "codex-rollout": 1 });
    assert.match(text, /claude-project\/c\/s\.jsonl:2: a home directory that is not a placeholder/);
    assert.match(text, /does not list codex-rollout\/r/);
    assert.doesNotMatch(text, /claude-project\/c\/s\.jsonl:1/);
  } finally {
    rmSync(scratch, { recursive: true, force: true });
  }
});

test("the committed fixture tree passes", () => {
  const { problems, counts } = checkFixtures(join(ROOT, FIXTURE_DIR));
  assert.deepEqual(problems, []);
  assert.ok(counts["claude-project"] > 0 && counts["codex-rollout"] > 0);
  assert.ok(readFileSync(join(ROOT, FIXTURE_DIR, "README.md"), "utf8").includes("synthetic"));
});
