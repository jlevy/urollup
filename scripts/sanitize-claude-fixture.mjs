#!/usr/bin/env node
// Turn selected Claude Code transcript excerpts into structure-only fixtures.
//
// Usage:
//   node scripts/sanitize-claude-fixture.mjs --input <excerpt-dir> --output <case-dir> \
//     [--date 2026-09-01T10:00:00.000Z]
//
// The input is a small excerpt laid out like a Claude config directory:
// `projects/<encoded-project>/<session>.jsonl`, `<session>/subagents/agent-<id>.jsonl`
// with `.meta.json`, and `subagents/workflows/<workflow>/`. Copy only the records a case
// needs into that tree; this script never reads a log store on its own. The output has the
// same layout and keeps only structure:
//
// - **Verbatim:** record order and count, keys, `type` and `role`, models, `usage`
//   numbers, stop reasons, tool names, `version`, `isSidechain` and other booleans and
//   numbers, so repeated block records, progress nesting and linkage survive.
// - **IDs:** every ID-shaped value (UUIDs, `msg_`, `req_`, `toolu_`, agent IDs) and every
//   value under an ID key is remapped through one deterministic table to a synthetic ID of
//   the same shape, in file names too, so duplicates and links still line up.
// - **Text:** every other string (prompts, replies, thinking, tool input and results,
//   paths, branches, titles, hook output) becomes a short placeholder, and object keys
//   that are not identifier-shaped (file paths used as map keys, say) are renamed.
// - **Time:** timestamps shift by one whole-second offset so the earliest lands on
//   `--date`, preserving order, intervals, fractional digits and zone designators;
//   `resetsAt` epoch seconds shift with them.
//
// The output is scanned with the fixture privacy check before anything is written, and the
// script never prints original values or the time offset. Review the result, and write the
// case's expected.json and README by hand, before committing. Zero dependencies.

import { existsSync, mkdirSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import path from "node:path";
import { pathToFileURL } from "node:url";

import { machineIdentities, scanPrivacy, splitLines } from "./check-fixtures.mjs";

export const DEFAULT_DATE = "2026-09-01T10:00:00.000Z";
export const PLACEHOLDER_CWD = "/Users/example/project";
export const PLACEHOLDER_PROJECT = "-Users-example-project";

const ISO_TIMESTAMP = /^(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2}):(\d{2})(\.\d+)?(Z|[+-]\d{2}:\d{2})$/;
const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;
const PREFIXED_ID = /^(msg_(?:[a-z]+_)?|req_|srvtoolu_|toolu_)([A-Za-z0-9]{6,})$/;
const AGENT_ID = /^a([0-9a-f]{16})$/;
const LABELED_AGENT_ID = /^a([A-Za-z_][A-Za-z0-9_-]*?)-([0-9a-f]{16})$/;
const SAFE_LABEL = /^[a-z_]{1,32}$/;
const SAFE_TOKEN = /^[A-Za-z0-9<>._:@+-]{1,80}$/;
const IDENTIFIER_KEY = /^[A-Za-z_$][A-Za-z0-9_$-]{0,63}$/;

// Keys whose string values are native IDs, whatever their shape.
const ID_KEYS = new Set([
  "agentId",
  "id",
  "leafUuid",
  "logicalParentUuid",
  "messageId",
  "parentToolUseID",
  "parentUuid",
  "promptId",
  "requestId",
  "sessionId",
  "session_id",
  "sourceToolAssistantUUID",
  "sourceToolUseID",
  "toolUseID",
  "toolUseId",
  "tool_use_id",
  "uuid",
]);
// Keys whose string values are enumerations or names that carry no private content, kept
// when they are a single safe token.
const VERBATIM_KEYS = new Set([
  "advisorModel",
  "agentType",
  "effort",
  "entrypoint",
  "inference_geo",
  "level",
  "model",
  "permissionMode",
  "rateLimitType",
  "role",
  "service_tier",
  "speed",
  "status",
  "stop_reason",
  "subtype",
  "type",
  "userType",
  "version",
]);
const TOOL_BLOCK_TYPES = new Set(["mcp_tool_use", "server_tool_use", "tool_use"]);
const EPOCH_SECOND_KEYS = new Set(["resetsAt"]);
const PLACEHOLDERS = {
  content: "Synthetic content.",
  description: "Synthetic description.",
  signature: "c3ludGhldGlj",
  summary: "Synthetic summary.",
  text: "Synthetic text.",
  thinking: "Synthetic thinking.",
};
const KEPT_SEGMENTS = new Set(["projects", "subagents", "workflows"]);

function fail(message) {
  throw new Error(message);
}

function parseTimestamp(value) {
  const match = ISO_TIMESTAMP.exec(value);
  if (!match) {
    return null;
  }
  const [, year, month, day, hour, minute, second, fraction = "", zone] = match;
  const zoneSeconds = zone === "Z" ? 0 : (zone[0] === "-" ? -1 : 1) * (Number(zone.slice(1, 3)) * 3600 + Number(zone.slice(4, 6)) * 60);
  const localSeconds = Date.UTC(Number(year), Number(month) - 1, Number(day), Number(hour), Number(minute), Number(second)) / 1000;
  return { utcSeconds: localSeconds - zoneSeconds, fraction, zone, zoneSeconds };
}

function pad(number, width) {
  return String(number).padStart(width, "0");
}

/** Shift an RFC 3339 timestamp by whole seconds, keeping its precision and zone. */
export function shiftTimestamp(value, offsetSeconds) {
  const parsed = parseTimestamp(value);
  if (!parsed) {
    return value;
  }
  const local = new Date((parsed.utcSeconds + offsetSeconds + parsed.zoneSeconds) * 1000);
  const date = `${pad(local.getUTCFullYear(), 4)}-${pad(local.getUTCMonth() + 1, 2)}-${pad(local.getUTCDate(), 2)}`;
  const time = `${pad(local.getUTCHours(), 2)}:${pad(local.getUTCMinutes(), 2)}:${pad(local.getUTCSeconds(), 2)}`;
  return `${date}T${time}${parsed.fraction}${parsed.zone}`;
}

/** A deterministic table from original IDs to synthetic IDs of the same shape. */
export class IdTable {
  constructor() {
    this.map = new Map();
    this.counters = new Map();
  }

  next(kind) {
    const value = (this.counters.get(kind) ?? 0) + 1;
    this.counters.set(kind, value);
    return value;
  }

  get size() {
    return this.map.size;
  }

  /** The synthetic ID for an ID-shaped value, or null when `value` has no known ID shape. */
  shaped(value) {
    if (this.map.has(value)) {
      return this.map.get(value);
    }
    let synthetic = null;
    let match;
    if (UUID.test(value)) {
      synthetic = `00000000-0000-4000-8000-${this.next("uuid").toString(16).padStart(12, "0")}`;
    } else if ((match = PREFIXED_ID.exec(value))) {
      const [, prefix, body] = match;
      synthetic = `${prefix}${`01Synth${this.next(prefix).toString(36)}`.padEnd(body.length, "0")}`;
    } else if (AGENT_ID.test(value)) {
      synthetic = `a${this.next("agent").toString(16).padStart(16, "0")}`;
    } else if ((match = LABELED_AGENT_ID.exec(value))) {
      const label = SAFE_LABEL.test(match[1]) ? match[1] : "label";
      synthetic = `a${label}-${this.next("agent").toString(16).padStart(16, "0")}`;
    }
    if (synthetic !== null) {
      this.map.set(value, synthetic);
    }
    return synthetic;
  }

  /** The synthetic ID for any native ID, falling back to a generic serial. */
  any(value, kind = "id") {
    return this.shaped(value) ?? this.generic(value, kind);
  }

  generic(value, kind) {
    if (!this.map.has(value)) {
      this.map.set(value, `${kind}-${pad(this.next(kind), 4)}`);
    }
    return this.map.get(value);
  }
}

/**
 * The earliest record time in an excerpt, from each record's own top-level `timestamp`.
 * Nested timestamps are excluded on purpose: an environment snapshot or a file history
 * entry can carry a much older time, and anchoring on it would leave every record near its
 * real date.
 */
function earliestTimestamp(record, found = { seconds: Infinity }) {
  const parsed = typeof record?.timestamp === "string" ? parseTimestamp(record.timestamp) : null;
  if (parsed && parsed.utcSeconds < found.seconds) {
    found.seconds = parsed.utcSeconds;
  }
  return found;
}

class Sanitizer {
  constructor({ offsetSeconds, cwd, ids }) {
    this.offsetSeconds = offsetSeconds;
    this.cwd = cwd;
    this.ids = ids;
    this.stats = { records: 0, strings: 0, timestamps: 0, keys: 0 };
  }

  key(key) {
    // Some maps are keyed by a native ID (`wireToolInputs` by tool-use ID), and others by
    // a file path, so keys are sanitized like values, not trusted because they are keys.
    const shaped = this.ids.shaped(key);
    if (shaped !== null) {
      return shaped;
    }
    if (IDENTIFIER_KEY.test(key)) {
      return key;
    }
    this.stats.keys += 1;
    return this.ids.generic(`key:${key}`, "key");
  }

  string(value, key, parent) {
    if (value === "") {
      return value;
    }
    if (ISO_TIMESTAMP.test(value)) {
      this.stats.timestamps += 1;
      return shiftTimestamp(value, this.offsetSeconds);
    }
    const shaped = this.ids.shaped(value);
    if (shaped !== null) {
      return shaped;
    }
    if (ID_KEYS.has(key)) {
      return this.ids.generic(value, "id");
    }
    if (VERBATIM_KEYS.has(key) && SAFE_TOKEN.test(value)) {
      return value;
    }
    if (key === "name" && TOOL_BLOCK_TYPES.has(parent?.type) && SAFE_TOKEN.test(value)) {
      return value;
    }
    this.stats.strings += 1;
    if (key === "cwd") {
      return this.cwd;
    }
    if (key === "gitBranch") {
      return "main";
    }
    return PLACEHOLDERS[key] ?? "synthetic";
  }

  value(value, key, parent) {
    if (typeof value === "string") {
      return this.string(value, key, parent);
    }
    if (typeof value === "number" && EPOCH_SECOND_KEYS.has(key)) {
      return value + this.offsetSeconds;
    }
    if (Array.isArray(value)) {
      return value.map((item) => this.value(item, key, parent));
    }
    if (value && typeof value === "object") {
      return Object.fromEntries(Object.entries(value).map(([childKey, child]) => [this.key(childKey), this.value(child, childKey, value)]));
    }
    return value;
  }

  record(line) {
    this.stats.records += 1;
    return JSON.stringify(this.value(JSON.parse(line), null, null));
  }
}

/** The output path for an excerpt path, with IDs remapped and the project renamed. */
export function sanitizePath(relPath, ids, project = PLACEHOLDER_PROJECT) {
  const segments = relPath.split("/");
  return segments
    .map((segment, index) => {
      if (KEPT_SEGMENTS.has(segment)) {
        return segment;
      }
      if (segments[index - 1] === "projects") {
        return project;
      }
      const [, agentPrefix = "", stem, suffix = ""] = /^(agent-)?(.+?)((?:\.meta)?\.jsonl?)?$/.exec(segment);
      const kind = segments[index - 1] === "workflows" ? "workflow" : "dir";
      return `${agentPrefix}${ids.any(stem, kind)}${suffix}`;
    })
    .join("/");
}

/**
 * Sanitize excerpt files, given as `{ path, text }` with `/`-separated relative paths.
 * Returns `{ files, stats }`; nothing here touches the filesystem.
 */
export function sanitizeExcerpt(files, { date = DEFAULT_DATE, cwd = PLACEHOLDER_CWD, project = PLACEHOLDER_PROJECT } = {}) {
  const anchor = parseTimestamp(date);
  if (!anchor) {
    fail(`--date must be an RFC 3339 timestamp, got ${JSON.stringify(date)}`);
  }
  const sorted = [...files].sort((a, b) => (a.path < b.path ? -1 : a.path > b.path ? 1 : 0));
  for (const file of sorted) {
    if (path.isAbsolute(file.path) || file.path.split("/").includes("..")) {
      fail(`excerpt paths must be relative and inside the excerpt: ${file.path}`);
    }
    if (!/\.(jsonl|json)$/.test(file.path)) {
      fail(`excerpts hold .jsonl transcripts and .json metadata only: ${file.path}`);
    }
  }

  // One offset for every file, from the earliest parseable timestamp.
  const earliest = { seconds: Infinity };
  for (const file of sorted) {
    for (const line of splitLines(file.text).lines) {
      try {
        earliestTimestamp(JSON.parse(line), earliest);
      } catch {
        // Malformed lines are replaced below and carry no timestamp.
      }
    }
  }
  const offsetSeconds = Number.isFinite(earliest.seconds) ? Math.round(anchor.utcSeconds - earliest.seconds) : 0;

  const ids = new IdTable();
  const sanitizer = new Sanitizer({ offsetSeconds, cwd, ids });
  let malformed = 0;
  const output = sorted.map((file) => {
    const outPath = sanitizePath(file.path, ids, project);
    if (file.path.endsWith(".json")) {
      return { path: outPath, text: `${sanitizer.record(file.text)}\n` };
    }
    const { lines, tail } = splitLines(file.text);
    const out = lines.map((line, index) => {
      try {
        return sanitizer.record(line);
      } catch {
        // Keep the defect, not the bytes: a torn tail stays unterminated.
        malformed += 1;
        return index === lines.length - 1 && tail ? '{"type":"synthetic","truncated":' : '{"type":"synthetic" malformed}';
      }
    });
    return { path: outPath, text: out.join("\n") + (tail ? "" : "\n") };
  });
  return { files: output, stats: { files: output.length, ids: ids.size, malformed, ...sanitizer.stats } };
}

/** Privacy findings in sanitized output, as strings. */
export function checkSanitizedOutput(files, { identities = [] } = {}) {
  return files.flatMap((file) => [
    ...scanPrivacy(file.path, { identities }).map((finding) => `${file.path}: ${finding.why} (in the path)`),
    ...scanPrivacy(file.text, { identities }).map((finding) => `${file.path}:${finding.line}: ${finding.why}`),
  ]);
}

function readExcerpt(directory, base = directory) {
  const files = [];
  for (const entry of readdirSync(directory, { withFileTypes: true })) {
    const full = path.join(directory, entry.name);
    if (entry.isDirectory()) {
      files.push(...readExcerpt(full, base));
    } else if (entry.isFile()) {
      files.push({ path: path.relative(base, full).split(path.sep).join("/"), text: readFileSync(full, "utf8") });
    }
  }
  return files;
}

export function parseArgs(argv) {
  const options = { date: DEFAULT_DATE };
  const usage = "usage: sanitize-claude-fixture.mjs --input <excerpt-dir> --output <case-dir> [--date <RFC 3339>]";
  for (let index = 0; index < argv.length; index += 1) {
    const flag = argv[index];
    const value = argv[index + 1];
    if (["--input", "--output", "--date"].includes(flag) && value) {
      options[flag.slice(2)] = value;
      index += 1;
    } else {
      fail(usage);
    }
  }
  if (!options.input || !options.output) {
    fail(usage);
  }
  return options;
}

function main() {
  const options = parseArgs(process.argv.slice(2));
  if (existsSync(options.output) && readdirSync(options.output).some((name) => name !== "README.md" && name !== "expected.json")) {
    fail(`${options.output} already holds fixture data; sanitize into a new case directory`);
  }
  const { files, stats } = sanitizeExcerpt(readExcerpt(options.input), { date: options.date });
  const findings = checkSanitizedOutput(files, { identities: machineIdentities() });
  if (findings.length > 0) {
    console.error(`sanitize: refusing to write ${files.length} files; the output still looks private:\n`);
    for (const finding of findings) {
      console.error(`  ${finding}`);
    }
    process.exitCode = 1;
    return;
  }
  for (const file of files) {
    const target = path.join(options.output, ...file.path.split("/"));
    mkdirSync(path.dirname(target), { recursive: true });
    writeFileSync(target, file.text);
  }
  console.log(
    `sanitize: wrote ${stats.files} files (${stats.records} records, ${stats.ids} IDs remapped, ` +
      `${stats.strings} strings and ${stats.keys} keys replaced, ${stats.timestamps} timestamps shifted, ` +
      `${stats.malformed} malformed lines kept as placeholders) to ${options.output}; review before committing`,
  );
}

if (import.meta.url === pathToFileURL(process.argv[1] ?? "").href) {
  try {
    main();
  } catch (error) {
    console.error(`sanitize: ${error.message}`);
    process.exitCode = 1;
  }
}
