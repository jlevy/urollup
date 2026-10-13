// Synthetic executables and archives for the release tests.
//
// Each builder writes the smallest file the release inspectors parse: an ELF64 with
// program and section headers, dynamic entries and symbol-version requirements; a thin
// Mach-O with a build-version command and linked dylibs; and a PE32+ with one section of
// import descriptors. They let the tests prove every linkage rule without a compiler,
// cross toolchain or real release build.

import { createHash } from "node:crypto";
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import zlib from "node:zlib";

import { normalizeExpected } from "./check-e2e-results.mjs";
import { writeTarGz } from "./release-archive.mjs";
import { recordDigest } from "./release-inspect.mjs";
import { formatChecksums } from "./release-manifest.mjs";
import { fixtureCaseDir } from "./release-smoke.mjs";
import { plannedArtifacts, plannedCrates } from "./release-targets.mjs";

const sha256 = (data) => createHash("sha256").update(data).digest("hex");

function cstrings(strings) {
  const offsets = new Map();
  const parts = [Buffer.from([0])];
  let size = 1;
  for (const value of strings) {
    if (!offsets.has(value)) {
      offsets.set(value, size);
      const bytes = Buffer.from(`${value}\0`, "utf8");
      parts.push(bytes);
      size += bytes.length;
    }
  }
  return { buffer: Buffer.concat(parts), offsets };
}

/**
 * An ELF64 little-endian executable. `versions` is `[{file, version}]`, grouped by file
 * into the version-requirement section the dynamic linker reads.
 */
export function makeElf({ machine = 62, interpreter = null, needed = [], versions = [] } = {}) {
  const files = [...new Set(versions.map((entry) => entry.file))];
  const strings = cstrings([...needed, ...files, ...versions.map((entry) => entry.version)]);
  const interp = interpreter ? Buffer.from(`${interpreter}\0`) : Buffer.alloc(0);
  const dynamic = Buffer.alloc((needed.length + 1) * 16);
  needed.forEach((library, index) => {
    dynamic.writeBigInt64LE(1n, index * 16);
    dynamic.writeBigUInt64LE(BigInt(strings.offsets.get(library)), index * 16 + 8);
  });
  const verneedParts = [];
  files.forEach((file, fileIndex) => {
    const entries = versions.filter((entry) => entry.file === file);
    const need = Buffer.alloc(16);
    need.writeUInt16LE(1, 0);
    need.writeUInt16LE(entries.length, 2);
    need.writeUInt32LE(strings.offsets.get(file), 4);
    need.writeUInt32LE(16, 8);
    need.writeUInt32LE(fileIndex === files.length - 1 ? 0 : 16 + entries.length * 16, 12);
    verneedParts.push(need);
    entries.forEach((entry, index) => {
      const aux = Buffer.alloc(16);
      aux.writeUInt32LE(strings.offsets.get(entry.version), 8);
      aux.writeUInt32LE(index === entries.length - 1 ? 0 : 16, 12);
      verneedParts.push(aux);
    });
  });
  const verneed = Buffer.concat(verneedParts);

  const programCount = interpreter ? 1 : 0;
  const dataStart = 64 + programCount * 56;
  const interpOffset = dataStart;
  const strOffset = interpOffset + interp.length;
  const dynOffset = strOffset + strings.buffer.length;
  const verOffset = dynOffset + dynamic.length;
  const sectionOffset = verOffset + verneed.length;
  const sections = [
    { type: 0, offset: 0, size: 0, link: 0 },
    { type: 3, offset: strOffset, size: strings.buffer.length, link: 0 },
    { type: 6, offset: dynOffset, size: dynamic.length, link: 1 },
    ...(verneed.length > 0 ? [{ type: 0x6ffffffe, offset: verOffset, size: verneed.length, link: 1 }] : []),
  ];

  const header = Buffer.alloc(64);
  header.writeUInt32BE(0x7f454c46, 0);
  header[4] = 2;
  header[5] = 1;
  header[6] = 1;
  header.writeUInt16LE(3, 16);
  header.writeUInt16LE(machine, 18);
  header.writeUInt32LE(1, 20);
  header.writeBigUInt64LE(BigInt(programCount ? 64 : 0), 32);
  header.writeBigUInt64LE(BigInt(sectionOffset), 40);
  header.writeUInt16LE(64, 52);
  header.writeUInt16LE(56, 54);
  header.writeUInt16LE(programCount, 56);
  header.writeUInt16LE(64, 58);
  header.writeUInt16LE(sections.length, 60);
  const program = Buffer.alloc(programCount * 56);
  if (interpreter) {
    program.writeUInt32LE(3, 0);
    program.writeBigUInt64LE(BigInt(interpOffset), 8);
    program.writeBigUInt64LE(BigInt(interp.length), 32);
  }
  const sectionTable = Buffer.alloc(sections.length * 64);
  sections.forEach((section, index) => {
    sectionTable.writeUInt32LE(section.type, index * 64 + 4);
    sectionTable.writeBigUInt64LE(BigInt(section.offset), index * 64 + 24);
    sectionTable.writeBigUInt64LE(BigInt(section.size), index * 64 + 32);
    sectionTable.writeUInt32LE(section.link, index * 64 + 40);
  });
  return Buffer.concat([header, program, interp, strings.buffer, dynamic, verneed, sectionTable]);
}

/** A thin 64-bit Mach-O with an LC_BUILD_VERSION floor and LC_LOAD_DYLIB commands. */
export function makeMachO({ cpu = 0x0100000c, minimum = "11.0", platform = 1, dylibs = ["/usr/lib/libSystem.B.dylib"], fat = false } = {}) {
  if (fat) {
    const fatHeader = Buffer.alloc(32);
    fatHeader.writeUInt32BE(0xcafebabe, 0);
    return fatHeader;
  }
  const [major, minor = 0, patch = 0] = minimum.split(".").map(Number);
  const build = Buffer.alloc(24);
  build.writeUInt32LE(0x32, 0);
  build.writeUInt32LE(24, 4);
  build.writeUInt32LE(platform, 8);
  build.writeUInt32LE((major << 16) | (minor << 8) | patch, 12);
  build.writeUInt32LE(0x000e0000, 16);
  const commands = [build];
  for (const dylib of dylibs) {
    const name = Buffer.from(`${dylib}\0`);
    const size = Math.ceil((24 + name.length) / 8) * 8;
    const command = Buffer.alloc(size);
    command.writeUInt32LE(0xc, 0);
    command.writeUInt32LE(size, 4);
    command.writeUInt32LE(24, 8);
    name.copy(command, 24);
    commands.push(command);
  }
  const body = Buffer.concat(commands);
  const header = Buffer.alloc(32);
  header.writeUInt32LE(0xfeedfacf, 0);
  header.writeUInt32LE(cpu, 4);
  header.writeUInt32LE(2, 12);
  header.writeUInt32LE(commands.length, 16);
  header.writeUInt32LE(body.length, 20);
  return Buffer.concat([header, body]);
}

/** A PE32+ console executable whose one section holds import descriptors for `imports`. */
export function makePe({ machine = 0x8664, subsystem = 3, imports = ["KERNEL32.dll"] } = {}) {
  const sectionRva = 0x1000;
  const rawOffset = 0x200;
  const descriptors = Buffer.alloc((imports.length + 1) * 20);
  const names = [];
  let nameOffset = descriptors.length;
  imports.forEach((name, index) => {
    const bytes = Buffer.from(`${name}\0`);
    descriptors.writeUInt32LE(1, index * 20);
    descriptors.writeUInt32LE(sectionRva + nameOffset, index * 20 + 12);
    descriptors.writeUInt32LE(1, index * 20 + 16);
    names.push(bytes);
    nameOffset += bytes.length;
  });
  const section = Buffer.concat([descriptors, ...names]);
  const file = Buffer.alloc(rawOffset + section.length);
  file.write("MZ", 0, "ascii");
  file.writeUInt32LE(0x40, 0x3c);
  file.write("PE\0\0", 0x40, "binary");
  const coff = 0x44;
  file.writeUInt16LE(machine, coff);
  file.writeUInt16LE(1, coff + 2);
  file.writeUInt16LE(240, coff + 16);
  const optional = coff + 20;
  file.writeUInt16LE(0x20b, optional);
  file.writeUInt16LE(subsystem, optional + 68);
  file.writeUInt32LE(16, optional + 108);
  file.writeUInt32LE(sectionRva, optional + 112 + 8);
  file.writeUInt32LE(descriptors.length, optional + 112 + 12);
  const header = optional + 240;
  file.write(".idata", header, "ascii");
  file.writeUInt32LE(section.length, header + 8);
  file.writeUInt32LE(sectionRva, header + 12);
  file.writeUInt32LE(section.length, header + 16);
  file.writeUInt32LE(rawOffset, header + 20);
  section.copy(file, rawOffset);
  return file;
}

/** A stored (uncompressed) zip with nested names, Unix modes and optional raw tweaks, for wheels. */
export function makeZip(entries) {
  const locals = [];
  const centrals = [];
  let offset = 0;
  for (const entry of entries) {
    const name = Buffer.from(entry.name, "utf8");
    const data = entry.data;
    const crc = zlib.crc32(data) >>> 0;
    const local = Buffer.alloc(30);
    local.writeUInt32LE(0x04034b50, 0);
    local.writeUInt16LE(20, 4);
    local.writeUInt32LE(crc, 14);
    local.writeUInt32LE(data.length, 18);
    local.writeUInt32LE(data.length, 22);
    local.writeUInt16LE(name.length, 26);
    const central = Buffer.alloc(46);
    central.writeUInt32LE(0x02014b50, 0);
    central.writeUInt16LE((3 << 8) | 20, 4);
    central.writeUInt16LE(20, 6);
    central.writeUInt32LE(crc, 16);
    central.writeUInt32LE(data.length, 20);
    central.writeUInt32LE(data.length, 24);
    central.writeUInt16LE(name.length, 28);
    central.writeUInt32LE((((entry.type ?? 0o100000) | (entry.mode ?? 0o644)) << 16) >>> 0, 38);
    central.writeUInt32LE(offset, 42);
    locals.push(local, name, data);
    centrals.push(central, name);
    offset += local.length + name.length + data.length;
  }
  const directory = Buffer.concat(centrals);
  const end = Buffer.alloc(22);
  end.writeUInt32LE(0x06054b50, 0);
  end.writeUInt16LE(entries.length, 8);
  end.writeUInt16LE(entries.length, 10);
  end.writeUInt32LE(directory.length, 12);
  end.writeUInt32LE(offset, 16);
  return Buffer.concat([...locals, directory, end]);
}

/**
 * The members of a well-formed urollup wheel for `platformTag` carrying `executable`, with a
 * RECORD computed from them. `edit(members)` may change members before RECORD is written,
 * and `editRecord(lines)` may change RECORD itself.
 */
export function wheelMembers({ version = "0.1.0", platformTag, executable, script = "urollup", edit = (members) => members, editRecord = (lines) => lines }) {
  const distInfo = `urollup-${version}.dist-info`;
  const tags = platformTag.split(".").map((platform) => `Tag: py3-none-${platform}`);
  let members = [
    { name: `urollup-${version}.data/scripts/${script}`, data: executable, mode: 0o755 },
    { name: `${distInfo}/METADATA`, data: Buffer.from(`Metadata-Version: 2.4\nName: urollup\nVersion: ${version}\nSummary: test\n\n# urollup\n`) },
    { name: `${distInfo}/WHEEL`, data: Buffer.from(`Wheel-Version: 1.0\nGenerator: maturin (1.15.0)\nRoot-Is-Purelib: false\n${tags.join("\n")}\n`) },
  ];
  members = edit(members);
  const record = editRecord([...members.map((member) => `${member.name},${recordDigest(member.data)},${member.data.length}`), `${distInfo}/RECORD,,`]);
  return [...members, { name: `${distInfo}/RECORD`, data: Buffer.from(`${record.join("\n")}\n`) }];
}

/** A gzip tar with nested names, as `cargo package` writes a .crate. */
export function makeTarGz(entries) {
  const blocks = [];
  for (const entry of entries) {
    const header = Buffer.alloc(512, 0);
    header.write(entry.name, 0, 100, "utf8");
    header.write(`${(entry.mode ?? 0o644).toString(8).padStart(7, "0")}\0`, 100, "ascii");
    header.write("0000000\0", 108, "ascii");
    header.write("0000000\0", 116, "ascii");
    header.write(`${entry.data.length.toString(8).padStart(11, "0")}\0`, 124, "ascii");
    header.write("00000000000\0", 136, "ascii");
    header.write("        ", 148, "ascii");
    header.write(entry.type ?? "0", 156, "ascii");
    header.write("ustar\0", 257, "ascii");
    header.write("00", 263, "ascii");
    let sum = 0;
    for (const byte of header) {
      sum += byte;
    }
    header.write(`${sum.toString(8).padStart(6, "0")}\0 `, 148, "ascii");
    blocks.push(header, entry.data, Buffer.alloc((512 - (entry.data.length % 512)) % 512, 0));
  }
  blocks.push(Buffer.alloc(1024, 0));
  return zlib.gzipSync(Buffer.concat(blocks));
}

/** A report document whose extracted results are exactly what a fixture case's expected.json states. */
export function fixtureReportFor(fixtureCase) {
  const raw = JSON.parse(readFileSync(path.join(fixtureCaseDir(fixtureCase), "expected.json"), "utf8"));
  const { reconciled } = normalizeExpected(raw);
  const rows = (counts) => Object.entries(counts ?? {}).map(([value, requests]) => ({ value, requests }));
  return {
    schema_version: 1,
    coverage: { copies_excluded: reconciled.copies_excluded, limit_observations: reconciled.limit_observations },
    diagnostics: raw.diagnostics ?? [],
    totals: { requests: reconciled.ownership, tokens: reconciled.tokens, unresolved: reconciled.unresolved, possible: reconciled.possible },
    breakdowns: { model: rows(reconciled.models), effort: rows(reconciled.efforts) },
  };
}

/** A self-consistent publish-mode bundle with real archives (so probes can extract them). */
export function makeReleaseBundle({ commit, mode = "publish", firstRelease = true } = {}) {
  const dir = mkdtempSync(path.join(tmpdir(), "urollup-publish-test-"));
  mkdirSync(path.join(dir, "assets"));
  mkdirSync(path.join(dir, "crates"));
  const executable = Buffer.from("fake urollup executable");
  const artifacts = plannedArtifacts("0.1.0").map((artifact) => {
    const data =
      artifact.kind === "archive" && artifact.os !== "windows"
        ? writeTarGz(
            [
              { name: "urollup", data: executable, mode: 0o755 },
              { name: "README.md", data: Buffer.from("# urollup\n") },
              { name: "LICENSE", data: Buffer.from("MIT\n") },
            ],
            { mtime: 1791884182 },
          )
        : Buffer.from(`fake ${artifact.name}`);
    writeFileSync(path.join(dir, "assets", artifact.name), data);
    return {
      name: artifact.name,
      kind: artifact.kind,
      target: artifact.rustTarget,
      platform: artifact.host,
      size: data.length,
      sha256: sha256(data),
      executable_sha256: sha256(executable),
    };
  });
  const cargo = plannedCrates("0.1.0").map((crate) => {
    const data = Buffer.from(`fake ${crate.name}`);
    writeFileSync(path.join(dir, "crates", crate.name), data);
    return { name: crate.crate, version: "0.1.0", file: crate.name, size: data.length, sha256: sha256(data) };
  });
  const identity = { version: "0.1.0", tag: "v0.1.0", commit, mode };
  writeFileSync(path.join(dir, "SHA256SUMS"), formatChecksums(artifacts));
  writeFileSync(path.join(dir, "release-manifest.json"), JSON.stringify({ ...identity, pep440_version: "0.1.0", artifacts, cargo_packages: cargo }));
  writeFileSync(path.join(dir, "release-plan.json"), JSON.stringify({ ...identity, pep440: "0.1.0", first_release: firstRelease }));
  writeFileSync(path.join(dir, "release-notes.md"), "First release.\n");
  return { dir, artifacts, cargo, executable, cleanup: () => rmSync(dir, { recursive: true, force: true }) };
}
