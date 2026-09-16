#!/usr/bin/env node
// Keep every golden session hermetic and portable: it runs only the build under test, in a
// sandbox, with an environment the harness controls, and every block really runs.
//
// Adapted from fdu `scripts/check-golden-invocations.mjs` at afbb2ee; see PROVENANCE.md.
//
// Rules, each for a failure that would otherwise pass silently or on one machine only:
//
// - `path:` is exactly [$UROLLUP_BIN]. tryscript's `path:` prepends to the inherited PATH,
//   so an entry that fails to resolve falls through to whatever urollup is installed.
// - `sandbox:` is `true` or a relative case directory, so commands never run in, or write
//   to, the source tree.
// - No `before:` or `after:` hooks: they run in the platform shell, and tryscript discards a
//   successful hook's output, so behavior they cause is never asserted.
// - `env:` and `path:` expand only harness variables (GOLDEN_*, UROLLUP_BIN, TRYSCRIPT_*),
//   and never set HOME or the platform profile and XDG directories the harness owns.
// - No `<!-- skip -->` or `<!-- only -->` annotations: tryscript reports a skipped block as
//   passed, and `only` silently drops every other block.
// - No unknown wildcards (`[??]`, `???`): they are scaffolding for `--expand`, not
//   assertions.
// - Every command is a bare `urollup` invocation without shell syntax, because goldens run
//   under /bin/sh and cmd.exe alike.
// - A reading command expected to succeed names `--timezone`, since the default is the
//   system zone and Windows ignores TZ.
// - No committed file under tests/golden names the HOME canary token (scripts/golden-env.mjs).

import { readdirSync, readFileSync, statSync } from "node:fs";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

import { HOME_CANARY_TOKEN } from "./golden-env.mjs";

const ROOT = path.dirname(path.dirname(fileURLToPath(import.meta.url)));
export const GOLDEN_DIR = path.join("tests", "golden");
// Harness inputs, not sessions: synthetic outputs and cases for the results checker's tests.
const NON_SESSION_DIRS = new Set(["samples"]);

/** Commands that read usage and bucket it by calendar time (design §6.3). */
export const READING_COMMANDS = new Set(["sources", "sessions", "daily", "weekly", "monthly", "windows", "report", "requests", "tools", "tree", "export"]);

const HARNESS_VARIABLE = /^(?:GOLDEN_[A-Z0-9_]+|UROLLUP_BIN|TRYSCRIPT_[A-Z0-9_]+)$/;
const HARNESS_OWNED_ENV = /^(?:HOME|USERPROFILE|APPDATA|LOCALAPPDATA|XDG_[A-Z_]+|UROLLUP_CAPTURE_DIR|PATH)$/i;
// Characters /bin/sh or cmd.exe would interpret rather than pass to urollup.
const SHELL_SYNTAX = /[$%|&;<>()`"'*?~^\\!{}[\]]/;

/** Every `*.tryscript.md` under `dir`, as sorted `/`-separated paths relative to `base`. */
export function findSessions(base, dir = GOLDEN_DIR) {
  const found = [];
  const walk = (relative) => {
    for (const entry of readdirSync(path.join(base, relative), { withFileTypes: true })) {
      const child = path.join(relative, entry.name);
      if (entry.isDirectory()) {
        if (!(relative === dir && NON_SESSION_DIRS.has(entry.name))) {
          walk(child);
        }
      } else if (entry.name.endsWith(".tryscript.md")) {
        found.push(child.split(path.sep).join("/"));
      }
    }
  };
  walk(dir);
  return found.sort();
}

/** Every file under `dir`, relative to `base`, for content scans. */
export function findFiles(base, dir = GOLDEN_DIR) {
  const found = [];
  const walk = (relative) => {
    for (const entry of readdirSync(path.join(base, relative), { withFileTypes: true })) {
      const child = path.join(relative, entry.name);
      if (entry.isDirectory()) {
        walk(child);
      } else if (statSync(path.join(base, child)).isFile()) {
        found.push(child.split(path.sep).join("/"));
      }
    }
  };
  walk(dir);
  return found.sort();
}

/**
 * Parse the subset of YAML front matter tryscript sessions use: top-level scalars, lists
 * of `  - item` and maps of `  KEY: value`. Returns the entries and the closing line index.
 */
export function parseFrontMatter(lines) {
  const entries = new Map();
  if (lines[0] !== "---") {
    return { entries, end: -1 };
  }
  const end = lines.indexOf("---", 1);
  if (end === -1) {
    return { entries, end: -1, unclosed: true };
  }
  let current;
  for (let index = 1; index < end; index += 1) {
    const line = lines[index];
    const top = /^([A-Za-z_][A-Za-z0-9_]*):\s*(.*)$/.exec(line);
    if (top) {
      current = { line: index + 1, value: top[2].trim(), items: [], map: new Map() };
      entries.set(top[1], current);
      continue;
    }
    const item = /^\s+-\s+(.*)$/.exec(line);
    if (item && current) {
      current.items.push(item[1].trim());
      continue;
    }
    const pair = /^\s+([A-Za-z_][A-Za-z0-9_]*):\s*(.*)$/.exec(line);
    if (pair && current) {
      current.map.set(pair[1], { value: unquote(pair[2].trim()), line: index + 1 });
    }
  }
  return { entries, end };
}

function unquote(value) {
  return /^(["']).*\1$/.test(value) ? value.slice(1, -1) : value;
}

/** Console blocks at the top level of a session: command, expected exit, lines. */
export function parseBlocks(lines, start = 0) {
  const blocks = [];
  let fence;
  let block;
  for (let index = start; index < lines.length; index += 1) {
    const line = lines[index];
    const marker = /^(`{3,}|~{3,})\s*([A-Za-z0-9_-]*)/.exec(line);
    if (!fence && marker) {
      fence = marker[1];
      if (marker[2] === "console") {
        block = { line: index + 1, commands: [], continuations: [], body: [], exit: 0 };
      }
      continue;
    }
    if (fence && line.startsWith(fence) && line.slice(fence.length).trim() === "") {
      if (block) {
        blocks.push(block);
      }
      fence = undefined;
      block = undefined;
      continue;
    }
    if (!block) {
      continue;
    }
    if (line.startsWith("$ ")) {
      block.commands.push({ text: line.slice(2), line: index + 1 });
    } else if (line.startsWith("> ")) {
      block.continuations.push(index + 1);
    } else if (/^\? -?\d+$/.test(line)) {
      block.exit = Number(line.slice(2));
    } else {
      block.body.push({ text: line, line: index + 1 });
    }
  }
  return blocks;
}

/** Indexes of lines outside fenced blocks, where tryscript reads headings and annotations. */
export function topLevelLines(lines, start = 0) {
  const indexes = [];
  let fence;
  for (let index = start; index < lines.length; index += 1) {
    const marker = /^(`{3,}|~{3,})/.exec(lines[index]);
    if (!fence && marker) {
      fence = marker[1];
    } else if (fence && lines[index].startsWith(fence) && lines[index].slice(fence.length).trim() === "") {
      fence = undefined;
    } else if (!fence) {
      indexes.push(index);
    }
  }
  return indexes;
}

function variablesIn(value) {
  return [...value.matchAll(/\$(?:\{([A-Za-z_][A-Za-z0-9_]*)\}|([A-Za-z_][A-Za-z0-9_]*))/g)].map((match) => match[1] ?? match[2]);
}

/** Findings for one session's text; `file` is the path used in messages. */
export function lintSession(file, text) {
  const findings = [];
  const add = (line, message) => findings.push(`${file}:${line}: ${message}`);
  const lines = text.split(/\r?\n/);
  const { entries, end, unclosed } = parseFrontMatter(lines);
  if (unclosed || end === -1) {
    add(1, "no closed front matter, so the session sets no path, sandbox or environment");
    return findings;
  }

  const pathEntry = entries.get("path");
  if (!pathEntry) {
    add(1, "no `path:` entry, so urollup resolves from the inherited PATH");
  } else if (pathEntry.items.length !== 1 || pathEntry.items[0] !== "$UROLLUP_BIN") {
    add(pathEntry.line, `path must be exactly [$UROLLUP_BIN], found [${pathEntry.items.join(", ")}]`);
  }

  const sandbox = entries.get("sandbox");
  const sandboxValue = sandbox ? unquote(sandbox.value) : "";
  if (!sandbox || sandboxValue === "" || sandboxValue === "false" || path.isAbsolute(sandboxValue) || /^[A-Za-z]:[\\/]/.test(sandboxValue)) {
    add(sandbox?.line ?? 1, "sandbox must be `true` or a relative case directory, so commands never run in the source tree");
  }

  for (const hook of ["before", "after"]) {
    if (entries.has(hook)) {
      add(entries.get(hook).line, `no \`${hook}:\` hooks: they run in the platform shell and their output is never asserted`);
    }
  }

  for (const [name, { value, line }] of entries.get("env")?.map ?? []) {
    if (HARNESS_OWNED_ENV.test(name)) {
      add(line, `env must not set ${name}; the golden harness owns it (scripts/golden-env.mjs)`);
    }
    for (const variable of variablesIn(value)) {
      if (!HARNESS_VARIABLE.test(variable)) {
        add(line, `env ${name} expands $${variable}, which comes from the invoking environment; use a GOLDEN_* variable`);
      }
    }
  }

  for (const index of topLevelLines(lines, end + 1)) {
    const annotation = /<!--\s*(skip|only)\s*-->/.exec(lines[index]);
    if (annotation) {
      add(index + 1, `\`<!-- ${annotation[1]} -->\` hides blocks: tryscript reports a skipped block as passed and \`only\` drops the rest`);
    }
  }

  const blocks = parseBlocks(lines, end + 1);
  if (blocks.length === 0) {
    add(1, "no console blocks, so the session asserts nothing");
  }
  for (const block of blocks) {
    if (block.commands.length !== 1) {
      add(block.line, `a console block needs exactly one command, found ${block.commands.length}`);
    }
    for (const line of block.continuations) {
      add(line, "no `> ` continuation lines: /bin/sh and cmd.exe join them differently");
    }
    for (const { text: body, line } of block.body) {
      const content = body.startsWith("! ") ? body.slice(2) : body;
      if (content.includes("[??]") || content.trim() === "???") {
        add(line, "unknown wildcard left in place; run `node scripts/run-golden.mjs --expand <file>` and review the result");
      }
    }
    for (const { text: command, line } of block.commands) {
      const words = command.trim().split(/\s+/);
      if (words[0] !== "urollup") {
        add(line, `commands must invoke urollup directly, found \`${words[0]}\``);
        continue;
      }
      if (SHELL_SYNTAX.test(command)) {
        add(line, "commands must be a bare `urollup` invocation with no shell syntax, so /bin/sh and cmd.exe run the same command");
      }
      const subcommand = words.slice(1).find((word) => !word.startsWith("-"));
      const succeeds = block.exit === 0 && !words.some((word) => word === "--help" || word === "-h");
      if (succeeds && READING_COMMANDS.has(subcommand) && !words.some((word) => word === "--timezone" || word.startsWith("--timezone="))) {
        add(line, `\`urollup ${subcommand}\` is expected to succeed, so it must pass --timezone; the default is the system zone`);
      }
    }
  }
  return findings;
}

/** Findings for any file under tests/golden that names the HOME canary. */
export function lintCanary(file, text) {
  return text
    .split(/\r?\n/)
    .flatMap((line, index) => (line.includes(HOME_CANARY_TOKEN) ? [`${file}:${index + 1}: output read from the golden HOME's canary logs; name the case's discovery roots or pass --no-default-sources`] : []));
}

function main() {
  const findings = [];
  const sessions = findSessions(ROOT);
  for (const file of sessions) {
    findings.push(...lintSession(file, readFileSync(path.join(ROOT, file), "utf8")));
  }
  for (const file of findFiles(ROOT)) {
    findings.push(...lintCanary(file, readFileSync(path.join(ROOT, file), "utf8")));
  }
  if (sessions.length === 0) {
    findings.push("tests/golden: no *.tryscript.md sessions; the golden corpus is missing");
  }
  if (findings.length > 0) {
    console.error("golden sessions must be hermetic, portable and fully run:\n");
    for (const finding of findings) {
      console.error(`  ${finding}`);
    }
    process.exit(1);
  }
  console.log(`golden invocations ok: ${sessions.length} sessions pin $UROLLUP_BIN, run sandboxed and assert every block`);
}

if (import.meta.url === pathToFileURL(process.argv[1] ?? "").href) {
  main();
}
