// The hermetic environment every golden and end-to-end run of urollup gets.
//
// The golden runners start from nothing rather than from the invoking shell. `make check`
// is usually run inside a coding agent, whose environment names the agent's own live
// session (CLAUDE_CODE_SESSION_ID, CODEX_THREAD_ID, PI_SESSION_FILE) and whose HOME holds
// the maintainer's real logs. A report inheriting either would read private data and
// produce output that exists on one machine only (general-testing-rules: an ambient
// variable that redirects a tool's target is an input the suite must control).
//
// So a run gets:
// - an allowlist of variables the shell and Node need, and nothing else;
// - HOME (and USERPROFILE, APPDATA, LOCALAPPDATA and the XDG base directories) inside a
//   fresh temporary golden root;
// - canary logs in every default discovery root under that HOME, so a session that
//   forgets to name its roots prints the canary token instead of silently reading
//   whatever HOME holds; and
// - a snapshot of that HOME, so a run that writes into it fails afterwards.
//
// Discovery roots, flags and the capture directory are then set per session or case, never
// inherited (tests/golden/README.md).

import { createHash } from "node:crypto";
import { existsSync, mkdirSync, mkdtempSync, readdirSync, readFileSync, realpathSync, rmSync, statSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";

/** Printed by any report that read a canary log; committed test data must never contain it. */
export const HOME_CANARY_TOKEN = "urollup-home-canary";

const CANARY_ID = "00000000-0000-4000-8000-00000000ca7a";

// Variables passed through from the invoking environment. Everything else is dropped,
// including every CLAUDE_*, CODEX_*, PI_*, UROLLUP_*, GIT_* and credential variable.
const PASSTHROUGH = {
  posix: ["PATH", "TMPDIR"],
  // cmd.exe, Node's shell spawning and the Rust standard library need these on Windows.
  win32: ["PATH", "PATHEXT", "ComSpec", "SystemRoot", "SystemDrive", "windir", "TEMP", "TMP", "NUMBER_OF_PROCESSORS", "PROCESSOR_ARCHITECTURE", "OS"],
};

/** Variables every golden run fixes, whatever the platform. */
export const FIXED_ENV = {
  LANG: "C",
  LC_ALL: "C",
  NO_COLOR: "1",
  TZ: "UTC",
};

// Canary logs, one per default discovery root and default config location under HOME
// (design §2.1 and §2.5). Each names the token in its directory, its model and its
// working directory, so whichever field a report prints, the token shows.
const CANARY_FILES = [
  {
    path: `.claude/projects/-${HOME_CANARY_TOKEN}/${CANARY_ID}.jsonl`,
    lines: [claudeRecord()],
  },
  {
    path: `.config/claude/projects/-${HOME_CANARY_TOKEN}/${CANARY_ID}.jsonl`,
    lines: [claudeRecord()],
  },
  {
    path: `.codex/sessions/2026/01/01/rollout-2026-01-01T00-00-00-${CANARY_ID}.jsonl`,
    lines: codexRecords(),
  },
  {
    path: `.codex/archived_sessions/rollout-2026-01-01T00-00-00-${CANARY_ID}.jsonl`,
    lines: codexRecords(),
  },
  {
    path: `.pi/agent/sessions/--${HOME_CANARY_TOKEN}--/2026-01-01T00-00-00-000Z_${CANARY_ID}.jsonl`,
    lines: [JSON.stringify({ type: "session", id: CANARY_ID, timestamp: "2026-01-01T00:00:00.000Z", cwd: `/${HOME_CANARY_TOKEN}` })],
  },
  {
    path: ".config/urollup/sources.yaml",
    lines: [`# ${HOME_CANARY_TOKEN}: a golden run read the default source manifest`, `roots: [/${HOME_CANARY_TOKEN}]`],
  },
  {
    path: "Library/Application Support/urollup/sources.yaml",
    lines: [`# ${HOME_CANARY_TOKEN}: a golden run read the default source manifest`, `roots: [/${HOME_CANARY_TOKEN}]`],
  },
];

function claudeRecord() {
  return JSON.stringify({
    type: "assistant",
    sessionId: CANARY_ID,
    uuid: `${HOME_CANARY_TOKEN}-u1`,
    timestamp: "2026-01-01T00:00:00.000Z",
    cwd: `/${HOME_CANARY_TOKEN}`,
    requestId: `req_${HOME_CANARY_TOKEN}`,
    message: {
      id: `msg_${HOME_CANARY_TOKEN}`,
      role: "assistant",
      model: HOME_CANARY_TOKEN,
      usage: { input_tokens: 1, cache_read_input_tokens: 0, cache_creation_input_tokens: 0, output_tokens: 1 },
    },
  });
}

function codexRecords() {
  const usage = { input_tokens: 1, cached_input_tokens: 0, output_tokens: 1, reasoning_output_tokens: 0, total_tokens: 2 };
  return [
    { timestamp: "2026-01-01T00:00:00.000Z", type: "session_meta", payload: { id: CANARY_ID, timestamp: "2026-01-01T00:00:00.000Z", cwd: `/${HOME_CANARY_TOKEN}`, originator: HOME_CANARY_TOKEN } },
    { timestamp: "2026-01-01T00:00:01.000Z", type: "turn_context", payload: { cwd: `/${HOME_CANARY_TOKEN}`, model: HOME_CANARY_TOKEN } },
    { timestamp: "2026-01-01T00:00:02.000Z", type: "event_msg", payload: { type: "token_count", info: { total_token_usage: usage, last_token_usage: usage } } },
  ].map((record) => JSON.stringify(record));
}

/**
 * Build the environment for a golden run from an allowlist of `baseEnv`.
 *
 * `dirs` names the golden root's `home`, `emptyRoot` and `capture` directories; `extra`
 * adds harness variables such as UROLLUP_BIN. Pure, so the allowlist is unit-tested.
 */
export function buildGoldenEnv({ baseEnv, platform, dirs, extra = {} }) {
  const env = {};
  const names = platform === "win32" ? PASSTHROUGH.win32 : PASSTHROUGH.posix;
  for (const name of names) {
    // Windows variable names are case-insensitive, and PATH is often spelled `Path`.
    const key = platform === "win32" ? Object.keys(baseEnv).find((candidate) => candidate.toUpperCase() === name.toUpperCase()) : name;
    if (key !== undefined && baseEnv[key] !== undefined) {
      env[name] = baseEnv[key];
    }
  }
  const home = dirs.home;
  Object.assign(env, FIXED_ENV, {
    HOME: home,
    XDG_CONFIG_HOME: path.join(home, ".config"),
    XDG_DATA_HOME: path.join(home, ".local", "share"),
    XDG_CACHE_HOME: path.join(home, ".cache"),
    XDG_STATE_HOME: path.join(home, ".local", "state"),
    // Reserved by design §2.5 for the capture store (milestone 0.3), set now so a capture
    // can never land in a platform data directory the harness does not own.
    UROLLUP_CAPTURE_DIR: dirs.capture,
    GOLDEN_HOME: home,
    GOLDEN_EMPTY_ROOT: dirs.emptyRoot,
  });
  if (platform === "win32") {
    Object.assign(env, {
      USERPROFILE: home,
      APPDATA: path.join(home, "AppData", "Roaming"),
      LOCALAPPDATA: path.join(home, "AppData", "Local"),
    });
  }
  return { ...env, ...extra };
}

/**
 * Create a golden root in the system temporary directory: `home/` with its canary logs,
 * `empty-root/` with empty Claude and Codex discovery directories, and `capture/`.
 */
export function createGoldenRoot({ parent = tmpdir() } = {}) {
  // The `urollup-golden-` prefix is what scripts/check-portability.mjs recognizes as a
  // temporary directory, so an `--update` that expands it into a golden is refused.
  // Resolved, so a binary that canonicalizes paths (macOS /var -> /private/var) prints the
  // same prefix the harness passed it.
  const root = realpathSync(mkdtempSync(path.join(parent, "urollup-golden-")));
  const dirs = {
    root,
    home: path.join(root, "home"),
    emptyRoot: path.join(root, "empty-root"),
    capture: path.join(root, "capture"),
  };
  for (const file of CANARY_FILES) {
    const target = path.join(dirs.home, ...file.path.split("/"));
    mkdirSync(path.dirname(target), { recursive: true });
    writeFileSync(target, `${file.lines.join("\n")}\n`);
  }
  // An existing but empty root for agents a case does not use: a missing root named by a
  // variable is an error (design §2.1), and an unset variable would fall back to HOME.
  for (const sub of ["projects", "sessions", "archived_sessions"]) {
    mkdirSync(path.join(dirs.emptyRoot, sub), { recursive: true });
  }
  mkdirSync(dirs.capture);
  return dirs;
}

/** Map each file under `dir` (relative, `/`-separated) to a digest of its bytes. */
export function snapshotTree(dir) {
  const snapshot = new Map();
  const walk = (current) => {
    for (const entry of readdirSync(current, { withFileTypes: true })) {
      const full = path.join(current, entry.name);
      const relative = path.relative(dir, full).split(path.sep).join("/");
      if (entry.isDirectory()) {
        snapshot.set(`${relative}/`, "dir");
        walk(full);
      } else {
        const bytes = entry.isFile() ? readFileSync(full) : Buffer.from(`non-regular ${statSync(full).mode}`);
        snapshot.set(relative, createHash("sha256").update(bytes).digest("hex"));
      }
    }
  };
  if (existsSync(dir)) {
    walk(dir);
  }
  return snapshot;
}

/** Describe every file created, changed or removed between two snapshots. */
export function diffSnapshots(before, after) {
  const findings = [];
  for (const [file, digest] of after) {
    if (!before.has(file)) {
      findings.push(`created ${file}`);
    } else if (before.get(file) !== digest) {
      findings.push(`changed ${file}`);
    }
  }
  for (const file of before.keys()) {
    if (!after.has(file)) {
      findings.push(`removed ${file}`);
    }
  }
  return findings.sort();
}

/** Lines of `text` naming the canary token, for a diagnostic. */
export function canaryLines(text) {
  return text.split(/\r?\n/).filter((line) => line.includes(HOME_CANARY_TOKEN));
}

/**
 * Create a golden root and its environment, and return a guard that reports anything a
 * run wrote into the golden HOME. Call `cleanup()` in a `finally`.
 */
export function openGoldenEnvironment({ baseEnv = process.env, platform = process.platform, extra = {}, parent } = {}) {
  const dirs = createGoldenRoot({ parent });
  const before = snapshotTree(dirs.home);
  return {
    dirs,
    env: buildGoldenEnv({ baseEnv, platform, dirs, extra }),
    homeWrites: () => diffSnapshots(before, snapshotTree(dirs.home)),
    cleanup: () => rmSync(dirs.root, { recursive: true, force: true }),
  };
}
