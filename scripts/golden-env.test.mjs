import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { existsSync, mkdtempSync, readdirSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import test from "node:test";

import {
  HOME_CANARY_TOKEN,
  buildGoldenEnv,
  canaryLines,
  diffSnapshots,
  openGoldenEnvironment,
  snapshotTree,
} from "./golden-env.mjs";

const DIRS = { home: "/g/home", emptyRoot: "/g/empty-root", capture: "/g/capture" };

// The environment `make check` really runs in: inside an agent, with real roots named.
const AMBIENT = {
  PATH: "/usr/bin:/bin",
  TMPDIR: "/tmp/",
  HOME: "/Users/someone",
  CLAUDE_CODE_SESSION_ID: "live-session",
  CLAUDE_CONFIG_DIR: "/Users/someone/.claude",
  CODEX_HOME: "/Users/someone/.codex",
  CODEX_THREAD_ID: "live-thread",
  PI_SESSION_FILE: "/Users/someone/.pi/agent/sessions/x.jsonl",
  UROLLUP_CLAUDE_CONFIG_DIRS: "/Users/someone/.claude",
  XDG_CONFIG_HOME: "/Users/someone/.config",
  GIT_DIR: "/Users/someone/repo/.git",
  GITHUB_TOKEN: "secret",
  TZ: "America/Los_Angeles",
  LANG: "en_US.UTF-8",
};

test("only the allowlist passes through, so no agent session, root or credential leaks in", () => {
  const env = buildGoldenEnv({ baseEnv: AMBIENT, platform: "darwin", dirs: DIRS });
  assert.deepEqual(Object.keys(env).sort(), [
    "GOLDEN_EMPTY_ROOT",
    "GOLDEN_HOME",
    "HOME",
    "LANG",
    "LC_ALL",
    "NO_COLOR",
    "PATH",
    "TMPDIR",
    "TZ",
    "UROLLUP_CAPTURE_DIR",
    "XDG_CACHE_HOME",
    "XDG_CONFIG_HOME",
    "XDG_DATA_HOME",
    "XDG_STATE_HOME",
  ]);
  assert.equal(env.PATH, AMBIENT.PATH);
  assert.equal(env.HOME, "/g/home");
  assert.equal(env.XDG_CONFIG_HOME, path.join("/g/home", ".config"));
  assert.equal(env.UROLLUP_CAPTURE_DIR, "/g/capture");
  assert.equal(env.TZ, "UTC");
  assert.equal(env.LC_ALL, "C");
});

test("Windows keeps its shell variables case-insensitively and redirects profile directories", () => {
  const env = buildGoldenEnv({
    baseEnv: { Path: "C:\\bin", ComSpec: "cmd.exe", SYSTEMROOT: "C:\\Windows", USERPROFILE: "C:\\Users\\someone", APPDATA: "C:\\Users\\someone\\AppData\\Roaming" },
    platform: "win32",
    dirs: DIRS,
  });
  assert.equal(env.PATH, "C:\\bin");
  assert.equal(env.ComSpec, "cmd.exe");
  assert.equal(env.SystemRoot, "C:\\Windows");
  assert.equal(env.USERPROFILE, "/g/home");
  assert.equal(env.APPDATA.startsWith("/g/home"), true);
  assert.equal(env.LOCALAPPDATA.startsWith("/g/home"), true);
});

test("harness variables are added last and may not be dropped by the allowlist", () => {
  const env = buildGoldenEnv({ baseEnv: AMBIENT, platform: "linux", dirs: DIRS, extra: { UROLLUP_BIN: "/repo/target/debug" } });
  assert.equal(env.UROLLUP_BIN, "/repo/target/debug");
});

test("a golden root holds canary logs under HOME and empty discovery roots beside it", (t) => {
  const golden = openGoldenEnvironment({ baseEnv: AMBIENT, platform: "linux" });
  t.after(golden.cleanup);
  const files = [...snapshotTree(golden.dirs.home).keys()].filter((file) => !file.endsWith("/"));
  for (const root of [".claude/projects/", ".config/claude/projects/", ".codex/sessions/", ".codex/archived_sessions/", ".pi/agent/sessions/"]) {
    assert.ok(files.some((file) => file.startsWith(root)), `no canary under ${root}: ${files.join(", ")}`);
  }
  assert.ok(path.basename(golden.dirs.root).startsWith("urollup-golden-"));
  for (const sub of ["projects", "sessions", "archived_sessions"]) {
    assert.deepEqual(readdirSync(path.join(golden.dirs.emptyRoot, sub)), []);
  }
  assert.deepEqual(golden.homeWrites(), []);
  golden.cleanup();
  assert.equal(existsSync(golden.dirs.root), false);
});

test("a canary log names the token in its path and records, so any printed field reveals it", (t) => {
  const golden = openGoldenEnvironment({ baseEnv: AMBIENT, platform: "linux" });
  t.after(golden.cleanup);
  for (const file of [...snapshotTree(golden.dirs.home).keys()].filter((name) => name.endsWith(".jsonl"))) {
    assert.ok(file.includes(HOME_CANARY_TOKEN) || file.includes("ca7a"), file);
  }
  assert.deepEqual(canaryLines(`ok\nmodel ${HOME_CANARY_TOKEN}\r\nok`), [`model ${HOME_CANARY_TOKEN}`]);
  assert.deepEqual(canaryLines("no canary here"), []);
});

test("the home guard reports files a run created, changed or removed under HOME", (t) => {
  const golden = openGoldenEnvironment({ baseEnv: AMBIENT, platform: "linux" });
  t.after(golden.cleanup);
  const [canary] = [...snapshotTree(golden.dirs.home).keys()].filter((file) => file.startsWith(".codex/sessions/") && !file.endsWith("/"));
  writeFileSync(path.join(golden.dirs.home, ...canary.split("/")), "rewritten\n");
  rmSync(path.join(golden.dirs.home, ".config", "urollup", "sources.yaml"));
  writeFileSync(path.join(golden.dirs.home, "written-by-run.txt"), "x");
  // Writes to the capture directory are not writes to HOME.
  writeFileSync(path.join(golden.dirs.capture, "entry"), "x");
  assert.deepEqual(golden.homeWrites(), [`changed ${canary}`, "created written-by-run.txt", "removed .config/urollup/sources.yaml"]);
});

test("diffSnapshots is empty for identical trees", () => {
  const snapshot = new Map([["a", "1"], ["b/", "dir"]]);
  assert.deepEqual(diffSnapshots(snapshot, new Map(snapshot)), []);
});

test("a child process started with the golden environment sees the golden HOME and no agent variables", (t) => {
  const parent = mkdtempSync(path.join(tmpdir(), "golden-env-test-"));
  t.after(() => rmSync(parent, { recursive: true, force: true }));
  const golden = openGoldenEnvironment({ baseEnv: { ...process.env, ...AMBIENT, PATH: process.env.PATH }, parent });
  const result = spawnSync(process.execPath, ["-e", "console.log(JSON.stringify(process.env))"], { env: golden.env, encoding: "utf8" });
  assert.equal(result.status, 0, result.stderr);
  const seen = JSON.parse(result.stdout);
  assert.equal(seen.HOME, golden.dirs.home);
  const leaked = Object.keys(seen).filter((name) => /^(CLAUDE|CODEX|PI_|GIT|GITHUB)/.test(name) || (name.startsWith("UROLLUP_") && name !== "UROLLUP_CAPTURE_DIR"));
  assert.deepEqual(leaked, []);
});
