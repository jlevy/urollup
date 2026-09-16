import assert from "node:assert/strict";
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import test from "node:test";

import { findSessions, lintCanary, lintSession, parseBlocks } from "./check-golden-invocations.mjs";
import { HOME_CANARY_TOKEN } from "./golden-env.mjs";

const FRONT = ["---", "sandbox: true", "path:", "  - $UROLLUP_BIN", "env:", '  NO_COLOR: "1"', "---", "# Session", ""];

function session(...body) {
  return [...FRONT, ...body, ""].join("\n");
}

const GOOD_BLOCK = ["```console", "$ urollup report --all --timezone UTC --format json", "{}", "? 0", "```"];

function messages(text) {
  return lintSession("x.tryscript.md", text).map((finding) => finding.replace(/^x\.tryscript\.md:\d+: /, ""));
}

test("a hermetic, portable session has no findings", () => {
  assert.deepEqual(messages(session(...GOOD_BLOCK)), []);
});

test("path must be exactly $UROLLUP_BIN, or urollup resolves from PATH", () => {
  const text = session(...GOOD_BLOCK).replace("  - $UROLLUP_BIN", "  - $TRYSCRIPT_GIT_ROOT/target/debug");
  assert.match(messages(text).join("\n"), /path must be exactly \[\$UROLLUP_BIN\]/);
  assert.match(messages(session(...GOOD_BLOCK).replace("path:\n  - $UROLLUP_BIN\n", "")).join("\n"), /no `path:` entry/);
});

test("a session without a sandbox, or with an absolute one, runs in or near the source tree", () => {
  for (const replacement of ["sandbox: false", "sandbox: /tmp/case", "cwd: ."]) {
    assert.match(messages(session(...GOOD_BLOCK).replace("sandbox: true", replacement)).join("\n"), /sandbox must be/, replacement);
  }
  assert.deepEqual(messages(session(...GOOD_BLOCK).replace("sandbox: true", "sandbox: ../../../crates/urollup-core/tests/fixtures/claude-project/x")), []);
});

test("hooks and ambient or harness-owned environment are refused", () => {
  const text = session(...GOOD_BLOCK).replace(
    '  NO_COLOR: "1"',
    '  NO_COLOR: "1"\n  HOME: $GOLDEN_HOME\n  CLAUDE_CONFIG_DIR: $HOME/.claude\n  CODEX_HOME: $GOLDEN_EMPTY_ROOT\nbefore: make build',
  );
  const found = messages(text).join("\n");
  assert.match(found, /env must not set HOME/);
  assert.match(found, /CLAUDE_CONFIG_DIR expands \$HOME/);
  assert.doesNotMatch(found, /CODEX_HOME/);
  assert.match(found, /no `before:` hooks/);
});

test("skip and only annotations are refused outside fences but allowed as output text", () => {
  assert.match(messages(session("## Case <!-- skip -->", ...GOOD_BLOCK)).join("\n"), /<!-- skip -->/);
  assert.match(messages(session("## Case <!-- only -->", ...GOOD_BLOCK)).join("\n"), /<!-- only -->/);
  const inOutput = ["```console", "$ urollup report --all --timezone UTC", "<!-- skip -->", "```"];
  assert.deepEqual(messages(session(...inOutput)), []);
});

test("unknown wildcards are scaffolding, not assertions", () => {
  assert.match(messages(session("```console", "$ urollup report --all --timezone UTC", "???", "```")).join("\n"), /unknown wildcard/);
  assert.match(messages(session("```console", "$ urollup report --all --timezone UTC", "! error: [??]", "? 2", "```")).join("\n"), /unknown wildcard/);
  assert.deepEqual(messages(session("```console", "$ urollup report --all --timezone UTC", "total [..]", "...", "```")), []);
});

test("commands are bare urollup invocations that /bin/sh and cmd.exe read alike", () => {
  for (const command of ["urollup report --all --timezone UTC 2>&1", "urollup report --source $GOLDEN_FIXTURES --timezone UTC", 'urollup report --project "a b" --timezone UTC', "urollup report --source C:\\x --timezone UTC"]) {
    assert.match(messages(session("```console", `$ ${command}`, "```")).join("\n"), /bare `urollup` invocation/, command);
  }
  assert.match(messages(session("```console", "$ cat out.json", "```")).join("\n"), /invoke urollup directly, found `cat`/);
  assert.match(messages(session("```console", "$ urollup report \\", "> --all", "```")).join("\n"), /continuation/);
});

test("a reading command expected to succeed must fix its timezone", () => {
  assert.match(messages(session("```console", "$ urollup daily --all", "```")).join("\n"), /`urollup daily` is expected to succeed, so it must pass --timezone/);
  assert.deepEqual(messages(session("```console", "$ urollup daily --all --timezone=UTC", "```")), []);
  // Failures and help do not bucket usage by time.
  assert.deepEqual(messages(session("```console", "$ urollup report", "! error: no current session", "? 2", "```")), []);
  assert.deepEqual(messages(session("```console", "$ urollup report --help", "```")), []);
});

test("a session that asserts nothing, or has no front matter, is refused", () => {
  assert.match(messages(session("No blocks here.")).join("\n"), /no console blocks/);
  assert.match(messages("# no front matter\n").join("\n"), /no closed front matter/);
});

test("parseBlocks reads the expected exit code and ignores documentation fences", () => {
  const blocks = parseBlocks(["````markdown", "```console", "$ not a test", "```", "````", "```console", "$ urollup --version", "urollup 0.1.0", "? 0", "```"]);
  assert.equal(blocks.length, 1);
  assert.equal(blocks[0].commands[0].text, "urollup --version");
  assert.equal(blocks[0].exit, 0);
});

test("the canary token is refused in any file under tests/golden", () => {
  assert.deepEqual(lintCanary("a.json", '{"model": "sonnet"}'), []);
  assert.match(lintCanary("a.json", `{\n"model": "${HOME_CANARY_TOKEN}"}`)[0], /^a\.json:2: output read from the golden HOME/);
});

test("session discovery recurses into e2e directories but not into harness samples", (t) => {
  const base = mkdtempSync(path.join(tmpdir(), "golden-lint-test-"));
  t.after(() => rmSync(base, { recursive: true, force: true }));
  for (const file of ["tests/golden/a.tryscript.md", "tests/golden/e2e/claude-project/b.tryscript.md", "tests/golden/samples/c.tryscript.md", "tests/golden/README.md"]) {
    mkdirSync(path.join(base, path.dirname(file)), { recursive: true });
    writeFileSync(path.join(base, file), "");
  }
  assert.deepEqual(findSessions(base), ["tests/golden/a.tryscript.md", "tests/golden/e2e/claude-project/b.tryscript.md"]);
});
