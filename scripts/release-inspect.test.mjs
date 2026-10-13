import assert from "node:assert/strict";
import test from "node:test";

import { writeTarGz, writeZip } from "./release-archive.mjs";
import { checkArchive, checkExecutable, checkMemberTimes, checkWheel, maxSymbolVersion, parseElf, parseMachO, parsePe, parseRecord, parseWheelName, recordDigest } from "./release-inspect.mjs";
import { makeElf, makeMachO, makePe, makeZip, wheelMembers } from "./release-test-binaries.mjs";
import { artifactById } from "./release-targets.mjs";

const MTIME = 1791753852;
const README = Buffer.from("# urollup\n");
const LICENSE = Buffer.from("MIT License\n");

const STATIC_MUSL = makeElf({ machine: 62 });
const MANYLINUX = makeElf({
  machine: 62,
  interpreter: "/lib64/ld-linux-x86-64.so.2",
  needed: ["libgcc_s.so.1", "libc.so.6"],
  versions: [
    { file: "libgcc_s.so.1", version: "GCC_4.2.0" },
    { file: "libc.so.6", version: "GLIBC_2.2.5" },
    { file: "libc.so.6", version: "GLIBC_2.17" },
  ],
});
const MAC_ARM = makeMachO({ cpu: 0x0100000c, minimum: "11.0", dylibs: ["/usr/lib/libSystem.B.dylib", "/usr/lib/libiconv.2.dylib"] });
const WINDOWS = makePe({ imports: ["KERNEL32.dll", "ntdll.dll", "bcryptprimitives.dll"] });

function archive(artifactId, members) {
  const artifact = artifactById("0.1.0", artifactId);
  const data = artifact.os === "windows" ? writeZip(members, { mtime: MTIME }) : writeTarGz(members, { mtime: MTIME });
  return { artifact, data };
}

function goodMembers(executable, name = "urollup") {
  return [
    { name, data: executable, mode: 0o755 },
    { name: "README.md", data: README, mode: 0o644 },
    { name: "LICENSE", data: LICENSE, mode: 0o644 },
  ];
}

test("ELF parsing reads the interpreter, libraries and symbol versions", () => {
  const parsed = parseElf(MANYLINUX);
  assert.equal(parsed.arch, "x86_64");
  assert.equal(parsed.interpreter, "/lib64/ld-linux-x86-64.so.2");
  assert.deepEqual(parsed.needed, ["libgcc_s.so.1", "libc.so.6"]);
  assert.equal(maxSymbolVersion(parsed.versions, "GLIBC_"), "2.17");
  assert.equal(maxSymbolVersion(parsed.versions, "GCC_"), "4.2.0");
  assert.equal(parseElf(makeElf({ machine: 183 })).arch, "aarch64");
  assert.equal(parseElf(Buffer.from("MZ not elf")), null);
});

test("a static musl archive executable has no interpreter and no shared library", () => {
  const artifact = artifactById("0.1.0", "archive-linux-x86_64");
  assert.deepEqual(checkExecutable(STATIC_MUSL, artifact).problems, []);
  assert.match(checkExecutable(MANYLINUX, artifact).problems.join("\n"), /static musl executable must have no interpreter/);
  assert.match(checkExecutable(makeElf({ machine: 183 }), artifact).problems.join("\n"), /expected a x86_64 executable, found aarch64/);
  assert.match(checkExecutable(MAC_ARM, artifact).problems.join("\n"), /expected a elf executable for linux, found macho/);
});

test("a manylinux wheel executable needs at most GLIBC_2.17 and policy libraries", () => {
  const artifact = artifactById("0.1.0", "wheel-linux-x86_64");
  const passed = checkExecutable(MANYLINUX, artifact);
  assert.deepEqual(passed.problems, []);
  assert.equal(passed.facts.max_glibc, "2.17");
  const tooNew = makeElf({ machine: 62, interpreter: "/lib64/ld-linux-x86-64.so.2", needed: ["libc.so.6"], versions: [{ file: "libc.so.6", version: "GLIBC_2.34" }] });
  assert.match(checkExecutable(tooNew, artifact).problems.join("\n"), /requires GLIBC_2\.34, newer than the manylinux_2_17 floor/);
  const foreign = makeElf({ machine: 62, interpreter: "/lib64/ld-linux-x86-64.so.2", needed: ["libc.so.6", "libzstd.so.1"], versions: [{ file: "libc.so.6", version: "GLIBC_2.17" }] });
  assert.match(checkExecutable(foreign, artifact).problems.join("\n"), /outside the manylinux2014 policy: libzstd\.so\.1/);
  assert.match(checkExecutable(STATIC_MUSL, artifact).problems.join("\n"), /must be dynamically linked against glibc/);
});

test("macOS executables must carry the 11.0 floor and link only system libraries", () => {
  const artifact = artifactById("0.1.0", "archive-macos-aarch64");
  assert.deepEqual(checkExecutable(MAC_ARM, artifact).problems, []);
  assert.equal(parseMachO(MAC_ARM).minimum, "11.0");
  assert.match(checkExecutable(makeMachO({ minimum: "10.12" }), artifact).problems.join("\n"), /deployment floor is 10\.12, expected 11\.0/);
  assert.match(checkExecutable(makeMachO({ minimum: "12.0" }), artifact).problems.join("\n"), /floor is 12\.0/);
  assert.match(checkExecutable(makeMachO({ dylibs: ["/opt/homebrew/lib/libzstd.1.dylib"] }), artifact).problems.join("\n"), /outside the operating system/);
  assert.match(checkExecutable(makeMachO({ cpu: 0x01000007 }), artifact).problems.join("\n"), /expected a aarch64 executable, found x86_64/);
  assert.match(checkExecutable(makeMachO({ fat: true }), artifact).problems.join("\n"), /universal/);
  assert.match(checkExecutable(makeMachO({ platform: 2 }), artifact).problems.join("\n"), /platform is 2, not macOS/);
});

test("Windows executables are x86_64 console programs with a static C runtime", () => {
  const artifact = artifactById("0.1.0", "archive-windows-x86_64");
  assert.deepEqual(parsePe(WINDOWS).imports, ["KERNEL32.dll", "ntdll.dll", "bcryptprimitives.dll"]);
  assert.deepEqual(checkExecutable(WINDOWS, artifact).problems, []);
  assert.match(checkExecutable(makePe({ imports: ["KERNEL32.dll", "VCRUNTIME140.dll"] }), artifact).problems.join("\n"), /C runtime dynamically \(VCRUNTIME140\.dll\)/);
  assert.match(checkExecutable(makePe({ imports: ["api-ms-win-crt-runtime-l1-1-0.dll"] }), artifact).problems.join("\n"), /C runtime dynamically/);
  assert.match(checkExecutable(makePe({ subsystem: 2 }), artifact).problems.join("\n"), /not a console program/);
  assert.match(checkExecutable(makePe({ machine: 0xaa64 }), artifact).problems.join("\n"), /found aarch64/);
});

test("a well-formed archive passes and records the executable digest", () => {
  const { artifact, data } = archive("archive-linux-x86_64", goodMembers(STATIC_MUSL));
  const result = checkArchive(artifact.name, data, artifact, { readme: README, license: LICENSE });
  assert.deepEqual(result.problems, []);
  assert.match(result.facts.executable_sha256, /^[0-9a-f]{64}$/);
  const windows = archive("archive-windows-x86_64", goodMembers(WINDOWS, "urollup.exe"));
  assert.deepEqual(checkArchive(windows.artifact.name, windows.data, windows.artifact, { readme: README, license: LICENSE }).problems, []);
});

test("malformed archives fail: missing, extra, nested, linked, stale or unrunnable members", () => {
  const cases = [
    [goodMembers(STATIC_MUSL).filter((member) => member.name !== "LICENSE"), /missing LICENSE/],
    [[...goodMembers(STATIC_MUSL), { name: "install.sh", data: Buffer.from("#!/bin/sh\n"), mode: 0o755 }], /unexpected install\.sh/],
    [goodMembers(STATIC_MUSL).map((member) => (member.name === "README.md" ? { ...member, data: Buffer.from("old\n") } : member)), /README\.md differs/],
    [goodMembers(STATIC_MUSL).map((member) => (member.name === "urollup" ? { ...member, mode: 0o644 } : member)), /is not executable/],
    [goodMembers(MANYLINUX), /static musl executable/],
  ];
  for (const [members, expected] of cases) {
    const { artifact, data } = archive("archive-linux-x86_64", members);
    assert.match(checkArchive(artifact.name, data, artifact, { readme: README, license: LICENSE }).problems.join("\n"), expected);
  }
  const artifact = artifactById("0.1.0", "archive-linux-x86_64");
  assert.match(checkArchive(artifact.name, Buffer.from("garbage"), artifact, {}).problems.join("\n"), /not a readable archive/);
  const nested = makeZip([{ name: "urollup-0.1.0/urollup.exe", data: WINDOWS, mode: 0o755 }, { name: "README.md", data: README }, { name: "LICENSE", data: LICENSE }]);
  const windows = artifactById("0.1.0", "archive-windows-x86_64");
  assert.match(checkArchive(windows.name, nested, windows, {}).problems.join("\n"), /nested.*\n?|missing urollup\.exe/);
  const linked = makeZip([{ name: "urollup.exe", data: Buffer.from("/bin/sh"), mode: 0o777, type: 0o120000 }, { name: "README.md", data: README }, { name: "LICENSE", data: LICENSE }]);
  assert.match(checkArchive(windows.name, linked, windows, {}).problems.join("\n"), /is a symlink, not a regular file/);
});

test("wheel names and RECORD rows parse strictly", () => {
  assert.deepEqual(parseWheelName("urollup-0.1.0-py3-none-win_amd64.whl"), { distribution: "urollup", version: "0.1.0", python: "py3", abi: "none", platform: "win_amd64" });
  assert.throws(() => parseWheelName("urollup-0.1.0.tar.gz"), /not a wheel file name/);
  assert.deepEqual(parseRecord("a,sha256=x,1\nb,,\n"), [{ path: "a", hash: "sha256=x", size: "1" }, { path: "b", hash: "", size: "" }]);
  assert.throws(() => parseRecord('"a,b",sha256=x,1\n'), /quoting/);
  assert.throws(() => parseRecord("a,b\n"), /three fields/);
  assert.equal(recordDigest(Buffer.from("")), "sha256=47DEQpj8HBSa-_TImW-5JCeuQeRkm5NMpJWZG3hSuFU");
});

function wheel(artifactId, options = {}) {
  const artifact = artifactById("0.1.0", artifactId);
  const executable = { linux: artifact.linkage === "manylinux" ? MANYLINUX : STATIC_MUSL, macos: MAC_ARM, windows: WINDOWS }[artifact.os];
  const members = wheelMembers({ platformTag: artifact.platformTag, executable, script: artifact.executable, ...options });
  return { artifact, data: makeZip(members) };
}

test("a well-formed wheel passes: one executable, matching tags, metadata and RECORD", () => {
  for (const id of ["wheel-linux-x86_64", "wheel-macos-aarch64", "wheel-windows-x86_64"]) {
    const { artifact, data } = wheel(id);
    const result = checkWheel(artifact.name, data, artifact, { version: "0.1.0" });
    assert.deepEqual(result.problems, [], id);
    assert.match(result.facts.executable_sha256, /^[0-9a-f]{64}$/);
  }
});

test("a wheel with a missing or wrong script payload or RECORD entry fails", () => {
  const scenarios = [
    [{ edit: (members) => members.filter((member) => !member.name.includes(".data/scripts/")) }, /missing urollup-0\.1\.0\.data\/scripts\/urollup/],
    [{ script: "urollup-cli" }, /unexpected member urollup-0\.1\.0\.data\/scripts\/urollup-cli/],
    [{ edit: (members) => members.map((member) => (member.name.includes(".data/scripts/") ? { ...member, data: STATIC_MUSL } : member)) }, /must be dynamically linked against glibc/],
    [{ edit: (members) => [...members, { name: "urollup/__init__.py", data: Buffer.from("") }] }, /unexpected member urollup\/__init__\.py/],
    [{ editRecord: (lines) => lines.filter((line) => !line.includes(".data/scripts/")) }, /has no RECORD entry/],
    [{ editRecord: (lines) => lines.map((line) => (line.includes(".data/scripts/") ? line.replace(/sha256=[^,]+/, "sha256=AAAA") : line)) }, /RECORD entry for .*scripts\/urollup does not match its bytes/],
    [{ editRecord: (lines) => [...lines, "urollup-0.1.0.data/scripts/extra,sha256=AAAA,1"] }, /RECORD lists urollup-0\.1\.0\.data\/scripts\/extra, which the wheel does not contain/],
    [{ edit: (members) => members.map((member) => (member.name.endsWith("/WHEEL") ? { ...member, data: Buffer.from("Wheel-Version: 1.0\nRoot-Is-Purelib: false\nTag: py3-none-linux_x86_64\n") } : member)) }, /WHEEL tags/],
    [{ edit: (members) => members.map((member) => (member.name.endsWith("/METADATA") ? { ...member, data: Buffer.from("Name: urollup\nVersion: 0.1.0\nRequires-Dist: click\n") } : member)) }, /declares Python dependencies/],
    [{ edit: (members) => members.map((member) => (member.name.endsWith("/METADATA") ? { ...member, data: Buffer.from("Name: urollup\nVersion: 0.1.1\n") } : member)) }, /METADATA names urollup 0\.1\.1/],
  ];
  for (const [options, expected] of scenarios) {
    const { artifact, data } = wheel("wheel-linux-x86_64", options);
    assert.match(checkWheel(artifact.name, data, artifact, { version: "0.1.0" }).problems.join("\n"), expected);
  }
  const { artifact, data } = wheel("wheel-linux-x86_64");
  assert.match(checkWheel("urollup-0.1.0-py3-none-linux_x86_64.whl", data, artifact, { version: "0.1.0" }).problems.join("\n"), /expected urollup-0\.1\.0-py3-none-manylinux_2_17_x86_64/);
  assert.match(checkWheel(artifact.name, Buffer.from("x"), artifact, { version: "0.1.0" }).problems.join("\n"), /not a readable wheel/);
});

test("every archive and wheel member must carry the release commit's time", () => {
  const tar = archive("archive-linux-x86_64", goodMembers(STATIC_MUSL));
  assert.deepEqual(checkArchive(tar.artifact.name, tar.data, tar.artifact, { readme: README, license: LICENSE, sourceDateEpoch: MTIME }).problems, []);
  assert.match(
    checkArchive(tar.artifact.name, tar.data, tar.artifact, { readme: README, license: LICENSE, sourceDateEpoch: MTIME + 60 }).problems.join("\n"),
    /not stamped with the commit time/,
  );
  const zip = archive("archive-windows-x86_64", goodMembers(WINDOWS, "urollup.exe"));
  assert.deepEqual(checkArchive(zip.artifact.name, zip.data, zip.artifact, { readme: README, license: LICENSE, sourceDateEpoch: MTIME }).problems, [], "zip times hold to two seconds");
  assert.deepEqual(checkMemberTimes("w.whl", [{ name: "a", mtime: MTIME - 1 }], MTIME), [], "zip stores even seconds");
  assert.match(checkMemberTimes("w.whl", [{ name: "a", mtime: 315532800 }], MTIME).join(""), /w\.whl: a not stamped with the commit time/);
  assert.deepEqual(checkMemberTimes("w.whl", [{ name: "a", mtime: 315532800 }], undefined), [], "no planned time, no check");
  const { artifact, data } = wheel("wheel-linux-x86_64");
  assert.match(checkWheel(artifact.name, data, artifact, { version: "0.1.0", sourceDateEpoch: MTIME }).problems.join("\n"), /not stamped with the commit time/, "a wheel left at the zip epoch is refused");
});
