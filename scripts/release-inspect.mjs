// Inspect packaged release artifacts: the executable's format, architecture, dynamic
// dependencies and OS floor, the archive's exact membership, and a wheel's script payload,
// RECORD and metadata.
//
// Every check here reads the packaged bytes, never target/release, so a validation job can
// start from the downloaded artifact alone. The parsers cover only what the release needs:
// ELF64 little-endian, thin 64-bit Mach-O and PE32+.

import { createHash } from "node:crypto";

import { readArchive } from "./release-archive.mjs";
import { pep440Version } from "./release-identity.mjs";
import { MACOS_DEPLOYMENT_TARGET, MANYLINUX_GLIBC, PRODUCT, archiveMembers, executableName } from "./release-targets.mjs";

export function sha256(data) {
  return createHash("sha256").update(data).digest("hex");
}

/** A wheel RECORD digest: urlsafe base64 SHA-256 without padding (PEP 376, PEP 427). */
export function recordDigest(data) {
  return `sha256=${createHash("sha256").update(data).digest("base64url")}`;
}

function cString(buffer, offset) {
  const end = buffer.indexOf(0, offset);
  return buffer.toString("utf8", offset, end < 0 ? buffer.length : end);
}

function compareVersions(a, b) {
  const left = a.split(".").map(Number);
  const right = b.split(".").map(Number);
  for (let index = 0; index < Math.max(left.length, right.length); index += 1) {
    const difference = (left[index] ?? 0) - (right[index] ?? 0);
    if (difference !== 0) {
      return difference;
    }
  }
  return 0;
}

// ---------------------------------------------------------------------------------------
// ELF

const ELF_MACHINES = { 62: "x86_64", 183: "aarch64" };

/** Parse the parts of an ELF64 little-endian executable the release checks. */
export function parseElf(buffer) {
  if (buffer.length < 64 || buffer.readUInt32BE(0) !== 0x7f454c46) {
    return null;
  }
  if (buffer[4] !== 2 || buffer[5] !== 1) {
    throw new Error("only 64-bit little-endian ELF executables are released");
  }
  const machine = buffer.readUInt16LE(18);
  const phoff = Number(buffer.readBigUInt64LE(32));
  const shoff = Number(buffer.readBigUInt64LE(40));
  const phentsize = buffer.readUInt16LE(54);
  const phnum = buffer.readUInt16LE(56);
  const shentsize = buffer.readUInt16LE(58);
  const shnum = buffer.readUInt16LE(60);
  let interpreter = null;
  for (let index = 0; index < phnum; index += 1) {
    const header = phoff + index * phentsize;
    if (buffer.readUInt32LE(header) === 3) {
      const offset = Number(buffer.readBigUInt64LE(header + 8));
      interpreter = cString(buffer, offset);
    }
  }
  const sections = [];
  for (let index = 0; index < shnum; index += 1) {
    const header = shoff + index * shentsize;
    if (header + 64 > buffer.length) {
      break;
    }
    sections.push({
      type: buffer.readUInt32LE(header + 4),
      offset: Number(buffer.readBigUInt64LE(header + 24)),
      size: Number(buffer.readBigUInt64LE(header + 32)),
      link: buffer.readUInt32LE(header + 40),
    });
  }
  const needed = [];
  const versions = [];
  for (const section of sections) {
    const strings = sections[section.link];
    if (section.type === 6 && strings) {
      for (let entry = section.offset; entry + 16 <= section.offset + section.size; entry += 16) {
        const tag = buffer.readBigInt64LE(entry);
        if (tag === 0n) {
          break;
        }
        if (tag === 1n) {
          needed.push(cString(buffer, strings.offset + Number(buffer.readBigUInt64LE(entry + 8))));
        }
      }
    }
    if (section.type === 0x6ffffffe && strings) {
      let need = section.offset;
      for (;;) {
        const file = cString(buffer, strings.offset + buffer.readUInt32LE(need + 4));
        let aux = need + buffer.readUInt32LE(need + 8);
        for (let count = buffer.readUInt16LE(need + 2); count > 0; count -= 1) {
          versions.push({ file, version: cString(buffer, strings.offset + buffer.readUInt32LE(aux + 8)) });
          const nextAux = buffer.readUInt32LE(aux + 12);
          if (nextAux === 0) {
            break;
          }
          aux += nextAux;
        }
        const next = buffer.readUInt32LE(need + 12);
        if (next === 0) {
          break;
        }
        need += next;
      }
    }
  }
  return { format: "elf", arch: ELF_MACHINES[machine] ?? `machine-${machine}`, interpreter, needed, versions };
}

/** The highest version a binary requires under one symbol-version prefix, such as GLIBC_. */
export function maxSymbolVersion(versions, prefix) {
  return versions
    .map((entry) => entry.version)
    .filter((version) => version.startsWith(prefix) && /^\d+(\.\d+)*$/.test(version.slice(prefix.length)))
    .map((version) => version.slice(prefix.length))
    .reduce((highest, version) => (highest === null || compareVersions(version, highest) > 0 ? version : highest), null);
}

/** Shared libraries a manylinux2014 executable may link (PEP 599). */
export const MANYLINUX2014_LIBRARIES = new Set([
  "libgcc_s.so.1",
  "libstdc++.so.6",
  "libm.so.6",
  "libdl.so.2",
  "librt.so.1",
  "libc.so.6",
  "libnsl.so.1",
  "libutil.so.1",
  "libpthread.so.0",
  "libresolv.so.2",
  "libX11.so.6",
  "libXext.so.6",
  "libXrender.so.1",
  "libICE.so.6",
  "libSM.so.6",
  "libGL.so.1",
  "libgobject-2.0.so.0",
  "libgthread-2.0.so.0",
  "libglib-2.0.so.0",
  "ld-linux-x86-64.so.2",
  "ld-linux-aarch64.so.1",
]);

// ---------------------------------------------------------------------------------------
// Mach-O

const MACHO_CPU = { 0x01000007: "x86_64", 0x0100000c: "aarch64" };

function machoVersion(value) {
  return `${value >>> 16}.${(value >> 8) & 0xff}${value & 0xff ? `.${value & 0xff}` : ""}`;
}

/** Parse a thin 64-bit Mach-O executable's CPU, deployment floor and linked dylibs. */
export function parseMachO(buffer) {
  if (buffer.length < 32) {
    return null;
  }
  const magic = buffer.readUInt32LE(0);
  if (magic === 0xbebafeca || buffer.readUInt32BE(0) === 0xcafebabe) {
    throw new Error("universal (fat) Mach-O binaries are not released; each archive is one architecture");
  }
  if (magic !== 0xfeedfacf) {
    return null;
  }
  const cpu = buffer.readUInt32LE(4);
  const commands = buffer.readUInt32LE(16);
  let offset = 32;
  let minimum = null;
  let platform = null;
  const dylibs = [];
  for (let index = 0; index < commands; index += 1) {
    const command = buffer.readUInt32LE(offset);
    const size = buffer.readUInt32LE(offset + 4);
    if (command === 0x32) {
      platform = buffer.readUInt32LE(offset + 8);
      minimum = machoVersion(buffer.readUInt32LE(offset + 12));
    } else if (command === 0x24 && minimum === null) {
      platform = 1;
      minimum = machoVersion(buffer.readUInt32LE(offset + 8));
    } else if (command === 0xc || command === 0x80000018 || command === 0x8000001f) {
      dylibs.push(cString(buffer, offset + buffer.readUInt32LE(offset + 8)));
    }
    if (size < 8) {
      throw new Error(`Mach-O load command ${index} has size ${size}`);
    }
    offset += size;
  }
  return { format: "macho", arch: MACHO_CPU[cpu] ?? `cpu-${cpu.toString(16)}`, platform, minimum, dylibs };
}

// ---------------------------------------------------------------------------------------
// PE

/** Parse a PE32+ executable's machine, subsystem and imported DLLs. */
export function parsePe(buffer) {
  if (buffer.length < 64 || buffer.toString("ascii", 0, 2) !== "MZ") {
    return null;
  }
  const pe = buffer.readUInt32LE(0x3c);
  if (buffer.toString("binary", pe, pe + 4) !== "PE\0\0") {
    throw new Error("MZ executable without a PE header");
  }
  const machine = buffer.readUInt16LE(pe + 4);
  const sectionCount = buffer.readUInt16LE(pe + 6);
  const optionalSize = buffer.readUInt16LE(pe + 20);
  const optional = pe + 24;
  if (buffer.readUInt16LE(optional) !== 0x20b) {
    throw new Error("only PE32+ executables are released");
  }
  const subsystem = buffer.readUInt16LE(optional + 68);
  const importRva = buffer.readUInt32LE(optional + 112 + 8);
  const sections = [];
  for (let index = 0; index < sectionCount; index += 1) {
    const header = optional + optionalSize + index * 40;
    sections.push({
      virtualSize: buffer.readUInt32LE(header + 8),
      virtualAddress: buffer.readUInt32LE(header + 12),
      rawSize: buffer.readUInt32LE(header + 16),
      rawOffset: buffer.readUInt32LE(header + 20),
    });
  }
  const toOffset = (rva) => {
    const section = sections.find((candidate) => rva >= candidate.virtualAddress && rva < candidate.virtualAddress + Math.max(candidate.virtualSize, candidate.rawSize));
    if (!section) {
      throw new Error(`PE address ${rva.toString(16)} is in no section`);
    }
    return rva - section.virtualAddress + section.rawOffset;
  };
  const imports = [];
  if (importRva !== 0) {
    for (let descriptor = toOffset(importRva); ; descriptor += 20) {
      const nameRva = buffer.readUInt32LE(descriptor + 12);
      if (nameRva === 0 && buffer.readUInt32LE(descriptor) === 0 && buffer.readUInt32LE(descriptor + 16) === 0) {
        break;
      }
      imports.push(cString(buffer, toOffset(nameRva)));
    }
  }
  return { format: "pe", arch: machine === 0x8664 ? "x86_64" : machine === 0xaa64 ? "aarch64" : `machine-${machine.toString(16)}`, subsystem, imports };
}

export function parseExecutable(buffer) {
  const parsed = parseElf(buffer) ?? parseMachO(buffer) ?? parsePe(buffer);
  if (!parsed) {
    throw new Error("not an ELF, Mach-O or PE executable");
  }
  return parsed;
}

/** Dynamic C runtime imports that would make a Windows executable need a redistributable. */
const DYNAMIC_CRT = /^(vcruntime\d*|msvcp\d*|msvcr\d*|ucrtbase|api-ms-win-crt-.*)\.dll$/i;

/**
 * Check one executable against the release contract for its artifact: format and CPU,
 * then the linkage floor (`static-musl`, `manylinux`, `macos` or `windows-static-crt`).
 * Returns `{facts, problems}`; facts are recorded in the validation record.
 */
export function checkExecutable(buffer, { os, arch, linkage }) {
  const problems = [];
  let parsed;
  try {
    parsed = parseExecutable(buffer);
  } catch (error) {
    return { facts: {}, problems: [error.message] };
  }
  const expectedFormat = { linux: "elf", macos: "macho", windows: "pe" }[os];
  if (parsed.format !== expectedFormat) {
    problems.push(`expected a ${expectedFormat} executable for ${os}, found ${parsed.format}`);
  }
  if (parsed.arch !== arch) {
    problems.push(`expected a ${arch} executable, found ${parsed.arch}`);
  }
  const facts = { format: parsed.format, arch: parsed.arch };
  if (linkage === "static-musl" && parsed.format === "elf") {
    facts.interpreter = parsed.interpreter;
    facts.needed = parsed.needed;
    if (parsed.interpreter !== null || parsed.needed.length > 0) {
      problems.push(`a static musl executable must have no interpreter or shared libraries; found ${[parsed.interpreter, ...parsed.needed].filter(Boolean).join(", ")}`);
    }
  }
  if (linkage === "manylinux" && parsed.format === "elf") {
    const glibc = maxSymbolVersion(parsed.versions, "GLIBC_");
    facts.interpreter = parsed.interpreter;
    facts.needed = parsed.needed;
    facts.max_glibc = glibc;
    facts.max_gcc = maxSymbolVersion(parsed.versions, "GCC_");
    if (parsed.interpreter === null) {
      problems.push("a manylinux executable must be dynamically linked against glibc");
    }
    const outside = parsed.needed.filter((library) => !MANYLINUX2014_LIBRARIES.has(library));
    if (outside.length > 0) {
      problems.push(`links libraries outside the manylinux2014 policy: ${outside.join(", ")}`);
    }
    if (glibc === null || compareVersions(glibc, MANYLINUX_GLIBC) > 0) {
      problems.push(`requires GLIBC_${glibc}, newer than the manylinux_2_17 floor of ${MANYLINUX_GLIBC}`);
    }
    if (facts.max_gcc !== null && compareVersions(facts.max_gcc, "4.8.0") > 0) {
      problems.push(`requires GCC_${facts.max_gcc}, newer than manylinux2014 allows`);
    }
  }
  if (linkage === "macos" && parsed.format === "macho") {
    facts.minimum_macos = parsed.minimum;
    facts.dylibs = parsed.dylibs;
    if (parsed.platform !== 1) {
      problems.push(`the Mach-O platform is ${parsed.platform}, not macOS`);
    }
    if (parsed.minimum !== MACOS_DEPLOYMENT_TARGET) {
      problems.push(`the macOS deployment floor is ${parsed.minimum}, expected ${MACOS_DEPLOYMENT_TARGET}`);
    }
    const foreign = parsed.dylibs.filter((dylib) => !dylib.startsWith("/usr/lib/") && !dylib.startsWith("/System/Library/"));
    if (foreign.length > 0) {
      problems.push(`links libraries outside the operating system: ${foreign.join(", ")}`);
    }
  }
  if (linkage === "windows-static-crt" && parsed.format === "pe") {
    facts.imports = parsed.imports;
    if (parsed.subsystem !== 3) {
      problems.push(`the PE subsystem is ${parsed.subsystem}, not a console program`);
    }
    const crt = parsed.imports.filter((name) => DYNAMIC_CRT.test(name));
    if (crt.length > 0) {
      problems.push(`imports the C runtime dynamically (${crt.join(", ")}); release builds link it statically`);
    }
  }
  return { facts, problems };
}

// ---------------------------------------------------------------------------------------
// Archives

/**
 * Validate a GitHub archive: exactly the executable, README.md and LICENSE as flat regular
 * files, the executable runnable and correct for its target, and the documents identical to
 * the release commit's. Returns `{executable, facts, problems}`.
 */
export function checkArchive(name, buffer, artifact, { readme, license }) {
  let entries;
  try {
    entries = readArchive(name, buffer);
  } catch (error) {
    return { executable: null, facts: {}, problems: [`${name} is not a readable archive: ${error.message}`] };
  }
  const problems = [];
  const expected = archiveMembers(artifact.os);
  const names = entries.map((entry) => entry.name);
  for (const entry of entries) {
    if (entry.type !== "file") {
      problems.push(`${name}: member ${entry.name} is a ${entry.type}, not a regular file`);
    }
    if (entry.name.includes("/") || entry.name.includes("\\")) {
      problems.push(`${name}: member ${entry.name} is nested; archive members are flat`);
    }
  }
  const missing = expected.filter((member) => !names.includes(member));
  const extra = names.filter((member) => !expected.includes(member));
  const duplicated = names.filter((member, index) => names.indexOf(member) !== index);
  if (missing.length > 0) {
    problems.push(`${name}: missing ${missing.join(", ")}`);
  }
  if (extra.length > 0) {
    problems.push(`${name}: unexpected ${extra.join(", ")}`);
  }
  if (duplicated.length > 0) {
    problems.push(`${name}: duplicated ${duplicated.join(", ")}`);
  }
  const member = (wanted) => entries.find((entry) => entry.name === wanted && entry.type === "file");
  const readmeEntry = member("README.md");
  const licenseEntry = member("LICENSE");
  if (readmeEntry && readme && !readmeEntry.data.equals(readme)) {
    problems.push(`${name}: README.md differs from the release commit's`);
  }
  if (licenseEntry && license && !licenseEntry.data.equals(license)) {
    problems.push(`${name}: LICENSE differs from the release commit's`);
  }
  const executable = member(executableName(artifact.os));
  let facts = {};
  if (executable) {
    if (artifact.os !== "windows" && (executable.mode & 0o111) !== 0o111) {
      problems.push(`${name}: ${executable.name} is not executable (mode ${executable.mode.toString(8)})`);
    }
    const checked = checkExecutable(executable.data, artifact);
    facts = { ...checked.facts, executable_sha256: sha256(executable.data), executable_size: executable.data.length };
    problems.push(...checked.problems.map((problem) => `${name}: ${problem}`));
  }
  return { executable: executable?.data ?? null, facts, problems };
}

// ---------------------------------------------------------------------------------------
// Wheels

/** Split a wheel file name (PEP 427) into its fields. */
export function parseWheelName(name) {
  const match = name.match(/^([A-Za-z0-9_.]+)-([^-]+)-([^-]+)-([^-]+)-([^-]+)\.whl$/);
  if (!match) {
    throw new Error(`${name} is not a wheel file name`);
  }
  const [, distribution, version, python, abi, platform] = match;
  return { distribution, version, python, abi, platform };
}

/** Parse RECORD (PEP 376 CSV with no quoted fields in a urollup wheel). */
export function parseRecord(text) {
  const rows = [];
  for (const line of text.split(/\r?\n/)) {
    if (line === "") {
      continue;
    }
    if (line.includes('"')) {
      throw new Error(`RECORD line ${JSON.stringify(line)} uses quoting a urollup wheel never needs`);
    }
    const fields = line.split(",");
    if (fields.length !== 3) {
      throw new Error(`RECORD line ${JSON.stringify(line)} does not have three fields`);
    }
    rows.push({ path: fields[0], hash: fields[1], size: fields[2] });
  }
  return rows;
}

function headerFields(text) {
  const fields = new Map();
  for (const line of text.split(/\r?\n/)) {
    if (line === "") {
      break;
    }
    const match = line.match(/^([A-Za-z0-9-]+):\s?(.*)$/);
    if (match) {
      const values = fields.get(match[1]) ?? [];
      values.push(match[2]);
      fields.set(match[1], values);
    }
  }
  return fields;
}

/**
 * Validate a urollup wheel from its bytes: the file name and tags, METADATA, WHEEL,
 * RECORD, and the one script payload, which must be the right executable for the target.
 * Returns `{executable, facts, problems}`.
 */
export function checkWheel(name, buffer, artifact, { version }) {
  const problems = [];
  const pep440 = pep440Version(version);
  let parts;
  try {
    parts = parseWheelName(name);
  } catch (error) {
    return { executable: null, facts: {}, problems: [error.message] };
  }
  if (parts.distribution !== PRODUCT || parts.version !== pep440 || parts.python !== "py3" || parts.abi !== "none" || parts.platform !== artifact.platformTag) {
    problems.push(`${name}: expected ${PRODUCT}-${pep440}-py3-none-${artifact.platformTag}.whl`);
  }
  let entries;
  try {
    entries = readArchive(name, buffer);
  } catch (error) {
    return { executable: null, facts: {}, problems: [...problems, `${name} is not a readable wheel: ${error.message}`] };
  }
  const distInfo = `${PRODUCT}-${pep440}.dist-info`;
  const script = `${PRODUCT}-${pep440}.data/scripts/${executableName(artifact.os)}`;
  const required = [script, `${distInfo}/METADATA`, `${distInfo}/WHEEL`, `${distInfo}/RECORD`];
  const byName = new Map(entries.map((entry) => [entry.name, entry]));
  for (const entry of entries) {
    const allowed = required.includes(entry.name) || (entry.name.startsWith(`${distInfo}/licenses/`) && entry.type === "file");
    if (!allowed) {
      problems.push(`${name}: unexpected member ${entry.name}; a urollup wheel holds only its executable and metadata`);
    }
    if (entry.type !== "file") {
      problems.push(`${name}: member ${entry.name} is a ${entry.type}`);
    }
  }
  if (byName.size !== entries.length) {
    problems.push(`${name}: a member name appears twice`);
  }
  for (const member of required) {
    if (!byName.has(member)) {
      problems.push(`${name}: missing ${member}`);
    }
  }

  const metadata = byName.get(`${distInfo}/METADATA`);
  if (metadata) {
    const fields = headerFields(metadata.data.toString("utf8"));
    if (fields.get("Name")?.[0] !== PRODUCT || fields.get("Version")?.[0] !== pep440) {
      problems.push(`${name}: METADATA names ${fields.get("Name")?.[0]} ${fields.get("Version")?.[0]}, expected ${PRODUCT} ${pep440}`);
    }
    if (fields.has("Requires-Dist")) {
      problems.push(`${name}: METADATA declares Python dependencies; the wheel carries only a Rust executable`);
    }
    if (fields.has("Requires-Python")) {
      problems.push(`${name}: METADATA declares Requires-Python; the executable loads no Python ABI`);
    }
  }
  const wheel = byName.get(`${distInfo}/WHEEL`);
  if (wheel) {
    const fields = headerFields(wheel.data.toString("utf8"));
    const tags = new Set(fields.get("Tag") ?? []);
    const expectedTags = new Set(artifact.platformTag.split(".").map((platform) => `py3-none-${platform}`));
    if (fields.get("Root-Is-Purelib")?.[0] !== "false") {
      problems.push(`${name}: WHEEL must declare Root-Is-Purelib: false`);
    }
    if (tags.size !== expectedTags.size || [...expectedTags].some((tag) => !tags.has(tag))) {
      problems.push(`${name}: WHEEL tags ${[...tags].join(", ")} do not match ${[...expectedTags].join(", ")}`);
    }
  }
  const record = byName.get(`${distInfo}/RECORD`);
  if (record) {
    let rows = [];
    try {
      rows = parseRecord(record.data.toString("utf8"));
    } catch (error) {
      problems.push(`${name}: ${error.message}`);
    }
    const listed = new Set();
    for (const row of rows) {
      if (listed.has(row.path)) {
        problems.push(`${name}: RECORD lists ${row.path} twice`);
      }
      listed.add(row.path);
      const entry = byName.get(row.path);
      if (!entry) {
        problems.push(`${name}: RECORD lists ${row.path}, which the wheel does not contain`);
      } else if (row.path === `${distInfo}/RECORD`) {
        if (row.hash !== "" || row.size !== "") {
          problems.push(`${name}: RECORD must not hash itself`);
        }
      } else if (row.hash !== recordDigest(entry.data) || row.size !== String(entry.data.length)) {
        problems.push(`${name}: RECORD entry for ${row.path} does not match its bytes`);
      }
    }
    for (const entry of entries) {
      if (!listed.has(entry.name)) {
        problems.push(`${name}: ${entry.name} has no RECORD entry`);
      }
    }
  }
  const payload = byName.get(script);
  let facts = {};
  if (payload) {
    if (payload.mode !== 0 && artifact.os !== "windows" && (payload.mode & 0o111) !== 0o111) {
      problems.push(`${name}: the script payload is not executable (mode ${payload.mode.toString(8)})`);
    }
    const checked = checkExecutable(payload.data, artifact);
    facts = { ...checked.facts, executable_sha256: sha256(payload.data), executable_size: payload.data.length };
    problems.push(...checked.problems.map((problem) => `${name}: ${problem}`));
  }
  return { executable: payload?.data ?? null, facts, problems };
}
