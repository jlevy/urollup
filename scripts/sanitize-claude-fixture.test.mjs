import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

import { IdTable, checkSanitizedOutput, parseArgs, sanitizeExcerpt, sanitizePath, shiftTimestamp } from "./sanitize-claude-fixture.mjs";

const SCRIPT = join(dirname(fileURLToPath(import.meta.url)), "sanitize-claude-fixture.mjs");

// An excerpt with made-up private-looking content: not a real transcript, but shaped like
// one, so the tests can assert that none of it survives.
const SESSION = "3f2a9c1e-7b4d-4e8f-9a0b-1c2d3e4f5a6b";
const AGENT = "a1b2c3d4e5f6a7b89";
const PRIVATE_CWD = "/Users/jdoe/src/secret-project";
const PROJECT = "-Users-jdoe-src-secret-project";
const SESSION_OUT = "00000000-0000-4000-8000-000000000001";

function assistant({ uuid, parentUuid, ts, content, output, extra = {} }) {
  return {
    parentUuid,
    isSidechain: false,
    userType: "external",
    cwd: PRIVATE_CWD,
    sessionId: SESSION,
    version: "2.1.214",
    gitBranch: "jdoe/feature-private",
    message: {
      id: "msg_01AbCdEfGhIjKlMnOpQrStUv",
      type: "message",
      role: "assistant",
      model: "claude-sonnet-4-5",
      content,
      stop_reason: output > 100 ? "tool_use" : null,
      stop_sequence: null,
      usage: { input_tokens: 3, cache_creation_input_tokens: 0, cache_read_input_tokens: 40000, output_tokens: output, service_tier: "standard" },
    },
    requestId: "req_011CAbCdEfGhIjKlMnOpQrSt",
    type: "assistant",
    uuid,
    timestamp: ts,
    ...extra,
  };
}

function excerpt() {
  const lines = [
    { parentUuid: null, isSidechain: false, cwd: PRIVATE_CWD, sessionId: SESSION, version: "2.1.214", type: "user", message: { role: "user", content: "Please read the notes about jdoe@corp.test" }, uuid: "11111111-2222-4333-8444-555555555555", timestamp: "2025-12-31T23:59:58.123456Z", promptId: "99999999-8888-4777-8666-555555555555", attachment: { type: "environment", lastModified: "2001-02-03T04:05:06Z" } },
    assistant({ uuid: "aaaaaaaa-bbbb-4ccc-8ddd-eeeeeeeeeeee", parentUuid: "11111111-2222-4333-8444-555555555555", ts: "2026-01-01T00:00:01.5Z", content: [{ type: "thinking", thinking: "private plan", signature: "SIGNATUREvalue" }], output: 12 }),
    assistant({
      uuid: "ffffffff-bbbb-4ccc-8ddd-eeeeeeeeeeee",
      parentUuid: "aaaaaaaa-bbbb-4ccc-8ddd-eeeeeeeeeeee",
      ts: "2025-12-31T19:00:02.000-05:00",
      content: [{ type: "tool_use", id: "toolu_01XyZabcdefghijklmnopqrs", name: "Agent", input: { prompt: "look in /Users/jdoe/src", options: { id: null } } }],
      output: 600,
      extra: { quotaLimits: { status: "allowed_warning", rateLimitType: "five_hour", resetsAt: 1767243600 } },
    }),
    { type: "user", sessionId: SESSION, uuid: "12121212-3434-4565-8787-909090909090", timestamp: "2026-01-01T00:00:03.000Z", message: { role: "user", content: [{ type: "tool_result", tool_use_id: "toolu_01XyZabcdefghijklmnopqrs", content: "secret output", is_error: false }] }, toolUseResult: { agentId: AGENT, "/Users/jdoe/src/a.rs": { lines: 3 } }, wireToolInputs: { "toolu_01XyZabcdefghijklmnopqrs": { prompt: "look in /Users/jdoe/src" } } },
  ];
  const subagent = [{ ...assistant({ uuid: "aaaaaaaa-bbbb-4ccc-8ddd-eeeeeeeeeeee", parentUuid: null, ts: "2026-01-01T00:00:01.5Z", content: [], output: 12 }), isSidechain: true, agentId: AGENT }];
  return [
    { path: `projects/${PROJECT}/${SESSION}.jsonl`, text: `${lines.map((line) => JSON.stringify(line)).join("\n")}\n` },
    { path: `projects/${PROJECT}/${SESSION}/subagents/agent-${AGENT}.jsonl`, text: `${subagent.map((line) => JSON.stringify(line)).join("\n")}\n{"type":"assistant","message":` },
    { path: `projects/${PROJECT}/${SESSION}/subagents/agent-${AGENT}.meta.json`, text: JSON.stringify({ agentType: "general-purpose", description: "Investigate the secret plan", toolUseId: "toolu_01XyZabcdefghijklmnopqrs", spawnDepth: 1 }) },
  ];
}

const records = (files, match) =>
  files
    .find((file) => match(file.path))
    .text.split("\n")
    .filter((line) => line.startsWith("{") && line.endsWith("}"))
    .map((line) => JSON.parse(line));

test("timestamps shift by whole seconds and keep precision and zone", () => {
  assert.equal(shiftTimestamp("2026-01-01T00:00:00.123456Z", 3600), "2026-01-01T01:00:00.123456Z");
  assert.equal(shiftTimestamp("2025-12-31T19:00:02.000-05:00", 86400), "2026-01-01T19:00:02.000-05:00");
  assert.equal(shiftTimestamp("2026-02-28T23:59:59Z", 1), "2026-03-01T00:00:00Z");
  assert.equal(shiftTimestamp("not a time", 5), "not a time");
});

test("IDs map to synthetic IDs of the same shape, the same original to the same ID", () => {
  const ids = new IdTable();
  const uuid = ids.shaped(SESSION);
  assert.match(uuid, /^00000000-0000-4000-8000-[0-9a-f]{12}$/);
  assert.equal(ids.shaped(SESSION), uuid);
  const message = ids.shaped("msg_01AbCdEfGhIjKlMnOpQrStUv");
  assert.match(message, /^msg_01Synth/);
  assert.equal(message.length, "msg_01AbCdEfGhIjKlMnOpQrStUv".length);
  assert.match(ids.shaped("msg_bdrk_01AbCdEfGhIjKl"), /^msg_bdrk_01Synth/);
  assert.match(ids.shaped("req_011CAbCdEfGhIjKlMnOpQrSt"), /^req_01Synth/);
  assert.match(ids.shaped("toolu_01XyZabcdefghijklmnopqrs"), /^toolu_01Synth/);
  assert.match(ids.shaped(AGENT), /^a[0-9a-f]{16}$/);
  assert.match(ids.shaped("aaside_question-0123456789abcdef"), /^aaside_question-[0-9a-f]{16}$/);
  assert.equal(ids.shaped("plain words"), null);
  assert.match(ids.any("gw-reused-1"), /^id-\d{4}$/);
});

test("paths rename the project and remap session, agent and workflow IDs", () => {
  const ids = new IdTable();
  const session = sanitizePath(`projects/${PROJECT}/${SESSION}.jsonl`, ids);
  const agent = sanitizePath(`projects/${PROJECT}/${SESSION}/subagents/workflows/wf_private/agent-${AGENT}.meta.json`, ids);
  assert.equal(session, `projects/-Users-example-project/${SESSION_OUT}.jsonl`);
  assert.equal(agent, `projects/-Users-example-project/${SESSION_OUT}/subagents/workflows/workflow-0001/agent-a0000000000000001.meta.json`);
});

test("structure, usage and links survive while text, paths and IDs are replaced", () => {
  const original = excerpt();
  const { files, stats } = sanitizeExcerpt(original, { date: "2026-09-01T10:00:00.000Z" });
  const [user, thinking, toolUse, result] = records(files, (p) => p.endsWith(`${SESSION_OUT}.jsonl`));
  const [copy] = records(files, (p) => p.includes("/subagents/") && p.endsWith(".jsonl"));
  const meta = JSON.parse(files.find((file) => file.path.endsWith(".meta.json")).text);

  // Verbatim structure and usage.
  assert.deepEqual(Object.keys(thinking), Object.keys(JSON.parse(original[0].text.split("\n")[1])));
  assert.deepEqual(toolUse.message.usage, { input_tokens: 3, cache_creation_input_tokens: 0, cache_read_input_tokens: 40000, output_tokens: 600, service_tier: "standard" });
  assert.equal(toolUse.message.model, "claude-sonnet-4-5");
  assert.equal(toolUse.message.stop_reason, "tool_use");
  assert.equal(toolUse.message.content[0].name, "Agent");
  assert.equal(toolUse.message.content[0].input.options.id, null);
  assert.equal(toolUse.quotaLimits.rateLimitType, "five_hour");
  assert.equal(toolUse.version, "2.1.214");
  assert.equal(copy.isSidechain, true);
  assert.equal(meta.agentType, "general-purpose");
  assert.equal(meta.spawnDepth, 1);

  // Links still line up across records and files.
  assert.equal(thinking.message.id, toolUse.message.id);
  assert.equal(thinking.requestId, toolUse.requestId);
  assert.equal(thinking.parentUuid, user.uuid);
  assert.equal(copy.uuid, thinking.uuid);
  assert.equal(result.message.content[0].tool_use_id, toolUse.message.content[0].id);
  assert.equal(meta.toolUseId, toolUse.message.content[0].id);
  assert.equal(result.toolUseResult.agentId, copy.agentId);
  assert.ok(files.some((file) => file.path.endsWith(`agent-${copy.agentId}.jsonl`)));

  // Text, paths, branches and map keys are replaced.
  assert.equal(user.message.content, "Synthetic content.");
  assert.equal(thinking.message.content[0].thinking, "Synthetic thinking.");
  assert.equal(thinking.cwd, "/Users/example/project");
  assert.equal(thinking.gitBranch, "main");
  assert.equal(toolUse.message.content[0].input.prompt, "synthetic");
  assert.equal(meta.description, "Synthetic description.");
  assert.deepEqual(Object.keys(result.toolUseResult), ["agentId", "key-0001"]);
  // A map keyed by a native ID is remapped through the same table, not kept as a key.
  assert.deepEqual(Object.keys(result.wireToolInputs), [toolUse.message.content[0].id]);

  // Time moves to the anchor, keeping order and intervals. The anchor comes from record
  // timestamps, so an old nested one cannot leave the records near their real dates.
  assert.equal(user.timestamp, "2026-09-01T10:00:00.123456Z");
  assert.equal(user.attachment.lastModified, shiftTimestamp("2001-02-03T04:05:06Z", Date.parse("2026-09-01T10:00:00Z") / 1000 - Date.parse("2025-12-31T23:59:58Z") / 1000));
  assert.equal(thinking.timestamp, "2026-09-01T10:00:03.5Z");
  assert.equal(toolUse.timestamp, "2026-09-01T05:00:04.000-05:00");
  assert.equal(toolUse.quotaLimits.resetsAt - Date.parse("2026-09-01T10:00:00Z") / 1000, 1767243600 - Date.parse("2025-12-31T23:59:58Z") / 1000);

  // The torn tail stays a torn tail, and nothing private remains.
  const subagentFile = files.find((file) => file.path.includes("/subagents/") && file.path.endsWith(".jsonl"));
  assert.equal(subagentFile.text.endsWith("\n"), false);
  assert.equal(stats.malformed, 1);
  const all = files.map((file) => `${file.path}\n${file.text}`).join("\n");
  for (const secret of ["jdoe", "secret", "private", "AbCdEf", "XyZ", SESSION, AGENT, "2025-12-31", "corp.test"]) {
    assert.doesNotMatch(all, new RegExp(secret, "i"), secret);
  }
  assert.deepEqual(checkSanitizedOutput(files, { identities: ["jdoe"] }), []);
});

test("sanitizing is deterministic whatever order the files arrive in", () => {
  assert.deepEqual(sanitizeExcerpt(excerpt()).files, sanitizeExcerpt([...excerpt()].reverse()).files);
});

test("the output check reports anything that still looks private", () => {
  const findings = checkSanitizedOutput(
    [
      { path: "projects/-Users-jdoe-x/s.jsonl", text: '{"cwd":"/home/jdoe"}\n' },
      { path: "projects/-Users-example-project/t.jsonl", text: '{"note":"written by jdoe"}\n' },
    ],
    { identities: ["jdoe"] },
  );
  const text = findings.join("\n");
  assert.match(text, /encoded home directory .*\(in the path\)/);
  assert.match(text, /s\.jsonl:1: a home directory that is not a placeholder/);
  assert.match(text, /t\.jsonl:1: the account or home directory name/);
});

test("arguments and unsafe excerpts are refused", () => {
  assert.deepEqual(parseArgs(["--input", "in", "--output", "out"]), { input: "in", output: "out", date: "2026-09-01T10:00:00.000Z" });
  assert.throws(() => parseArgs(["--input", "in"]), /usage/);
  assert.throws(() => sanitizeExcerpt([{ path: "../x.jsonl", text: "{}\n" }]), /relative and inside/);
  assert.throws(() => sanitizeExcerpt([{ path: "notes.txt", text: "x" }]), /\.jsonl transcripts/);
  assert.throws(() => sanitizeExcerpt([], { date: "yesterday" }), /RFC 3339/);
});

test("the command writes a reviewed tree and refuses to overwrite fixture data", () => {
  const scratch = mkdtempSync(join(tmpdir(), "urollup-sanitize-test-"));
  try {
    const input = join(scratch, "in");
    for (const file of excerpt()) {
      mkdirSync(dirname(join(input, file.path)), { recursive: true });
      writeFileSync(join(input, file.path), file.text);
    }
    const output = join(scratch, "out");
    const run = () => spawnSync(process.execPath, [SCRIPT, "--input", input, "--output", output], { encoding: "utf8" });
    const first = run();
    assert.equal(first.status, 0, first.stderr);
    assert.match(first.stdout, /wrote 3 files/);
    assert.doesNotMatch(`${first.stdout}${first.stderr}`, /jdoe|secret/);
    const written = readFileSync(join(output, "projects", "-Users-example-project", `${SESSION_OUT}.jsonl`), "utf8");
    assert.match(written, /"output_tokens":600/);
    const second = run();
    assert.equal(second.status, 1);
    assert.match(second.stderr, /already holds fixture data/);
  } finally {
    rmSync(scratch, { recursive: true, force: true });
  }
});
