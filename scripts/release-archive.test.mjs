import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import test from "node:test";
import zlib from "node:zlib";

import { checkMemberName, dosToEpoch, gzipDeterministic, readArchive, readTarGz, readZip, writeTarGz, writeZip } from "./release-archive.mjs";

const MTIME = 1791753852;
const ENTRIES = [
  { name: "urollup", data: Buffer.from("\x7fELF binary"), mode: 0o755 },
  { name: "README.md", data: Buffer.from("# urollup\n"), mode: 0o644 },
  { name: "LICENSE", data: Buffer.from("MIT\n"), mode: 0o644 },
];

test("tar.gz archives are deterministic, sorted, owner-free and readable", () => {
  const first = writeTarGz(ENTRIES, { mtime: MTIME });
  const second = writeTarGz([...ENTRIES].reverse(), { mtime: MTIME });
  assert.ok(first.equals(second), "the same files in any order give the same bytes");
  assert.deepEqual([...first.subarray(0, 10)], [0x1f, 0x8b, 8, 0, 0, 0, 0, 0, 2, 0xff]);
  const entries = readTarGz(first);
  assert.deepEqual(entries.map((entry) => [entry.name, entry.type, entry.mode, entry.mtime]), [
    ["LICENSE", "file", 0o644, MTIME],
    ["README.md", "file", 0o644, MTIME],
    ["urollup", "file", 0o755, MTIME],
  ]);
  assert.equal(entries[2].data.toString(), "\x7fELF binary");
  assert.ok(!writeTarGz(ENTRIES, { mtime: MTIME + 1 }).equals(first), "the time stamp is part of the bytes");
});

test("zip archives are deterministic and readable, with Unix modes", () => {
  const first = writeZip(ENTRIES, { mtime: MTIME });
  assert.ok(first.equals(writeZip([...ENTRIES].reverse(), { mtime: MTIME })));
  const entries = readZip(first);
  assert.deepEqual(entries.map((entry) => [entry.name, entry.type, entry.mode]), [
    ["LICENSE", "file", 0o644],
    ["README.md", "file", 0o644],
    ["urollup", "file", 0o755],
  ]);
  assert.deepEqual(readArchive("x.zip", first).map((entry) => entry.data.toString()), ["MIT\n", "# urollup\n", "\x7fELF binary"]);
});

test("the system tar and unzip read the archives", { skip: process.platform === "win32" }, () => {
  const dir = mkdtempSync(path.join(tmpdir(), "urollup-archive-test-"));
  try {
    writeFileSync(path.join(dir, "a.tar.gz"), writeTarGz(ENTRIES, { mtime: MTIME }));
    const listing = spawnSync("tar", ["-tzf", path.join(dir, "a.tar.gz")], { encoding: "utf8" });
    assert.equal(listing.status, 0, listing.stderr);
    assert.deepEqual(listing.stdout.trim().split("\n"), ["LICENSE", "README.md", "urollup"]);
    const extract = spawnSync("tar", ["-xzf", path.join(dir, "a.tar.gz"), "-C", dir], { encoding: "utf8" });
    assert.equal(extract.status, 0, extract.stderr);
    assert.equal(readFileSync(path.join(dir, "README.md"), "utf8"), "# urollup\n");
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("archive members are single plain names", () => {
  for (const bad of ["", ".", "..", "dir/urollup", "..\\x", "/etc/passwd", "C:evil", "a\0b"]) {
    assert.throws(() => checkMemberName(bad), /one plain file name/, bad);
  }
  assert.throws(() => writeTarGz([...ENTRIES, ENTRIES[0]], { mtime: MTIME }), /appears twice/);
  assert.throws(() => writeTarGz(ENTRIES, { mtime: 12 }), /between 1980/);
});

test("corrupt or truncated archives are refused rather than half-read", () => {
  const tar = writeTarGz(ENTRIES, { mtime: MTIME });
  assert.throws(() => readTarGz(tar.subarray(0, tar.length - 30)), /gzip stream|truncated|end-of-archive/);
  assert.throws(() => readTarGz(Buffer.from("not gzip")), /not a gzip stream/);
  const raw = zlib.gunzipSync(tar);
  raw[3] ^= 0x01;
  assert.throws(() => readTarGz(gzipDeterministic(raw)), /bad checksum/);
  const zip = writeZip(ENTRIES, { mtime: MTIME });
  assert.throws(() => readZip(zip.subarray(0, zip.length - 5)), /no end of central directory/);
  const flipped = Buffer.from(zip);
  flipped[40] ^= 0xff;
  assert.throws(() => readZip(flipped), /does not inflate|CRC-32|corrupt/);
  assert.throws(() => readArchive("x.rar", zip), /not a .tar.gz, .zip or .whl/);
});

test("links and directories are reported for the validator to refuse", () => {
  const raw = zlib.gunzipSync(writeTarGz(ENTRIES, { mtime: MTIME }));
  raw[156] = "2".charCodeAt(0);
  let sum = 0;
  for (let index = 0; index < 512; index += 1) {
    sum += index >= 148 && index < 156 ? 32 : raw[index];
  }
  raw.write(`${sum.toString(8).padStart(6, "0")}\0 `, 148, "ascii");
  assert.equal(readTarGz(gzipDeterministic(raw))[0].type, "symlink");
});

test("zip member times read back as UTC seconds, to two-second precision", () => {
  assert.deepEqual(readZip(writeZip(ENTRIES, { mtime: MTIME })).map((entry) => entry.mtime), [MTIME, MTIME, MTIME]);
  assert.deepEqual(readZip(writeZip(ENTRIES, { mtime: MTIME + 1 })).map((entry) => entry.mtime), [MTIME, MTIME, MTIME]);
  assert.equal(dosToEpoch(0, (0 << 9) | (1 << 5) | 1), 315532800, "the zip epoch is 1980-01-01");
});
