// Deterministic release archives, written and read without any system tar or zip.
//
// A GitHub archive holds a flat set of regular files (release-targets.mjs `archiveMembers`).
// Writing it here, rather than with the runner's tar or PowerShell, gives every platform the
// same bytes for the same inputs: entries are sorted, owners are zero, and every time stamp
// is the release commit's time (SOURCE_DATE_EPOCH), so an archive's digest can be
// reproduced from its tag. Reading it here gives validation one parser for archives and
// wheels on every host, which refuses links, directories, nesting and unsafe names instead
// of extracting them.

import zlib from "node:zlib";

function fail(message) {
  throw new Error(message);
}

/** A safe archive member name: one plain path component, never absolute or `..`. */
export function checkMemberName(name) {
  if (typeof name !== "string" || name === "" || name === "." || name === ".." || /[\\/\0]/.test(name) || /^[a-zA-Z]:/.test(name)) {
    fail(`archive member ${JSON.stringify(name)} must be one plain file name`);
  }
}

function sortedEntries(entries) {
  const names = new Set();
  for (const entry of entries) {
    checkMemberName(entry.name);
    if (names.has(entry.name)) {
      fail(`archive member ${entry.name} appears twice`);
    }
    names.add(entry.name);
    if (!Buffer.isBuffer(entry.data)) {
      fail(`archive member ${entry.name} has no data`);
    }
  }
  return [...entries].sort((a, b) => (a.name < b.name ? -1 : a.name > b.name ? 1 : 0));
}

function checkEpoch(mtime) {
  if (!Number.isInteger(mtime) || mtime < 315532800 || mtime > 8589934591) {
    fail(`archive time ${mtime} must be whole seconds between 1980 and the ustar limit`);
  }
}

// ---------------------------------------------------------------------------------------
// gzip (RFC 1952), with a fixed header so the bytes do not depend on the host's zlib build

const GZIP_HEADER = Buffer.from([0x1f, 0x8b, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0xff]);

export function gzipDeterministic(data) {
  const body = zlib.deflateRawSync(data, { level: 9 });
  const trailer = Buffer.alloc(8);
  trailer.writeUInt32LE(zlib.crc32(data) >>> 0, 0);
  trailer.writeUInt32LE(data.length % 2 ** 32, 4);
  return Buffer.concat([GZIP_HEADER, body, trailer]);
}

// ---------------------------------------------------------------------------------------
// ustar (POSIX.1-1988)

function octal(value, width) {
  const text = value.toString(8);
  if (text.length > width - 1) {
    fail(`value ${value} does not fit a ${width}-byte tar field`);
  }
  return `${text.padStart(width - 1, "0")}\0`;
}

function tarHeader({ name, size, mode, mtime }) {
  const header = Buffer.alloc(512, 0);
  header.write(name, 0, 100, "utf8");
  header.write(octal(mode, 8), 100, "ascii");
  header.write(octal(0, 8), 108, "ascii");
  header.write(octal(0, 8), 116, "ascii");
  header.write(octal(size, 12), 124, "ascii");
  header.write(octal(mtime, 12), 136, "ascii");
  header.write("        ", 148, "ascii");
  header.write("0", 156, "ascii");
  header.write("ustar\0", 257, "ascii");
  header.write("00", 263, "ascii");
  let sum = 0;
  for (const byte of header) {
    sum += byte;
  }
  header.write(`${sum.toString(8).padStart(6, "0")}\0 `, 148, "ascii");
  return header;
}

/**
 * Write a .tar.gz of flat regular files. `entries` is `[{name, data, mode}]`; every entry
 * gets owner 0 and time `mtime`, and entries are written in name order.
 */
export function writeTarGz(entries, { mtime }) {
  checkEpoch(mtime);
  const blocks = [];
  for (const entry of sortedEntries(entries)) {
    if (Buffer.byteLength(entry.name) > 100) {
      fail(`archive member ${entry.name} is longer than a ustar name`);
    }
    blocks.push(tarHeader({ name: entry.name, size: entry.data.length, mode: entry.mode ?? 0o644, mtime }));
    blocks.push(entry.data);
    const padding = (512 - (entry.data.length % 512)) % 512;
    blocks.push(Buffer.alloc(padding, 0));
  }
  blocks.push(Buffer.alloc(1024, 0));
  return gzipDeterministic(Buffer.concat(blocks));
}

function tarString(block, offset, length) {
  const end = block.indexOf(0, offset);
  return block.toString("utf8", offset, end >= 0 && end < offset + length ? end : offset + length);
}

function tarNumber(block, offset, length, what) {
  const text = tarString(block, offset, length).trim();
  if (!/^[0-7]+$/.test(text)) {
    fail(`tar header ${what} ${JSON.stringify(text)} is not octal`);
  }
  return Number.parseInt(text, 8);
}

/**
 * Read a .tar.gz into `[{name, type, mode, size, mtime, data}]`, in archive order.
 * Fails on a corrupt stream or header; it does not judge membership, which is the
 * validator's job, so a link or directory entry is returned with its type for refusal.
 */
export function readTarGz(buffer) {
  let tar;
  try {
    tar = zlib.gunzipSync(buffer);
  } catch (error) {
    fail(`not a gzip stream: ${error.message}`);
  }
  const entries = [];
  let offset = 0;
  while (offset + 512 <= tar.length) {
    const block = tar.subarray(offset, offset + 512);
    if (block.every((byte) => byte === 0)) {
      return entries;
    }
    let sum = 0;
    for (let index = 0; index < 512; index += 1) {
      sum += index >= 148 && index < 156 ? 32 : block[index];
    }
    if (tarNumber(block, 148, 8, "checksum") !== sum) {
      fail(`tar header at byte ${offset} has a bad checksum`);
    }
    const prefix = tarString(block, 345, 155);
    const name = prefix ? `${prefix}/${tarString(block, 0, 100)}` : tarString(block, 0, 100);
    const size = tarNumber(block, 124, 12, "size");
    const typeFlag = String.fromCharCode(block[156] || 0x30);
    const type = typeFlag === "0" ? "file" : typeFlag === "5" ? "directory" : typeFlag === "2" ? "symlink" : typeFlag === "1" ? "hardlink" : `type-${typeFlag}`;
    const start = offset + 512;
    if (start + size > tar.length) {
      fail(`tar member ${name} is truncated`);
    }
    entries.push({
      name,
      type,
      mode: tarNumber(block, 100, 8, "mode"),
      size,
      mtime: tarNumber(block, 136, 12, "mtime"),
      data: Buffer.from(tar.subarray(start, start + size)),
    });
    offset = start + Math.ceil(size / 512) * 512;
  }
  fail("tar stream ends without its end-of-archive blocks");
}

// ---------------------------------------------------------------------------------------
// zip (PKWARE APPNOTE), deflate only

/** MS-DOS date and time for a UTC instant, as zip stores it. */
function dosDateTime(epochSeconds) {
  const date = new Date(epochSeconds * 1000);
  const time = (date.getUTCHours() << 11) | (date.getUTCMinutes() << 5) | Math.floor(date.getUTCSeconds() / 2);
  const day = ((date.getUTCFullYear() - 1980) << 9) | ((date.getUTCMonth() + 1) << 5) | date.getUTCDate();
  return { time, day };
}

/** A zip member's MS-DOS date and time as UTC seconds, to the format's two-second precision. */
export function dosToEpoch(time, day) {
  return Date.UTC((day >> 9) + 1980, ((day >> 5) & 15) - 1, day & 31, time >> 11, (time >> 5) & 63, (time & 31) * 2) / 1000;
}

/**
 * Write a .zip of flat regular files, deflated, with Unix modes and the given time.
 * `entries` is `[{name, data, mode}]`.
 */
export function writeZip(entries, { mtime }) {
  checkEpoch(mtime);
  const { time, day } = dosDateTime(mtime);
  const locals = [];
  const centrals = [];
  let offset = 0;
  for (const entry of sortedEntries(entries)) {
    const name = Buffer.from(entry.name, "utf8");
    const compressed = zlib.deflateRawSync(entry.data, { level: 9 });
    const crc = zlib.crc32(entry.data) >>> 0;
    const local = Buffer.alloc(30);
    local.writeUInt32LE(0x04034b50, 0);
    local.writeUInt16LE(20, 4);
    local.writeUInt16LE(0x0800, 6);
    local.writeUInt16LE(8, 8);
    local.writeUInt16LE(time, 10);
    local.writeUInt16LE(day, 12);
    local.writeUInt32LE(crc, 14);
    local.writeUInt32LE(compressed.length, 18);
    local.writeUInt32LE(entry.data.length, 22);
    local.writeUInt16LE(name.length, 26);
    local.writeUInt16LE(0, 28);
    const central = Buffer.alloc(46);
    central.writeUInt32LE(0x02014b50, 0);
    central.writeUInt16LE((3 << 8) | 20, 4);
    central.writeUInt16LE(20, 6);
    central.writeUInt16LE(0x0800, 8);
    central.writeUInt16LE(8, 10);
    central.writeUInt16LE(time, 12);
    central.writeUInt16LE(day, 14);
    central.writeUInt32LE(crc, 16);
    central.writeUInt32LE(compressed.length, 20);
    central.writeUInt32LE(entry.data.length, 24);
    central.writeUInt16LE(name.length, 28);
    central.writeUInt32LE(((0o100000 | (entry.mode ?? 0o644)) << 16) >>> 0, 38);
    central.writeUInt32LE(offset, 42);
    locals.push(local, name, compressed);
    centrals.push(central, name);
    offset += local.length + name.length + compressed.length;
  }
  const directory = Buffer.concat(centrals);
  const end = Buffer.alloc(22);
  end.writeUInt32LE(0x06054b50, 0);
  end.writeUInt16LE(centrals.length / 2, 8);
  end.writeUInt16LE(centrals.length / 2, 10);
  end.writeUInt32LE(directory.length, 12);
  end.writeUInt32LE(offset, 16);
  return Buffer.concat([...locals, directory, end]);
}

/**
 * Read a .zip (an archive or a wheel) into `[{name, type, mode, size, data}]` from its
 * central directory, checking each member's CRC-32. Zip64 and encryption are refused.
 */
export function readZip(buffer) {
  const minimum = Math.max(0, buffer.length - 22 - 0xffff);
  let endOffset = -1;
  for (let index = buffer.length - 22; index >= minimum; index -= 1) {
    if (buffer.readUInt32LE(index) === 0x06054b50) {
      endOffset = index;
      break;
    }
  }
  if (endOffset < 0) {
    fail("not a zip archive: no end of central directory record");
  }
  const count = buffer.readUInt16LE(endOffset + 10);
  const directorySize = buffer.readUInt32LE(endOffset + 12);
  let offset = buffer.readUInt32LE(endOffset + 16);
  if (count === 0xffff || offset === 0xffffffff || offset + directorySize > endOffset) {
    fail("zip64 or a corrupt central directory is not supported");
  }
  const entries = [];
  for (let index = 0; index < count; index += 1) {
    if (buffer.readUInt32LE(offset) !== 0x02014b50) {
      fail(`zip central directory entry ${index} is corrupt`);
    }
    const madeBy = buffer.readUInt16LE(offset + 4) >> 8;
    const flags = buffer.readUInt16LE(offset + 8);
    const method = buffer.readUInt16LE(offset + 10);
    const dosTime = buffer.readUInt16LE(offset + 12);
    const dosDate = buffer.readUInt16LE(offset + 14);
    const crc = buffer.readUInt32LE(offset + 16);
    const compressedSize = buffer.readUInt32LE(offset + 20);
    const size = buffer.readUInt32LE(offset + 24);
    const nameLength = buffer.readUInt16LE(offset + 28);
    const extraLength = buffer.readUInt16LE(offset + 30);
    const commentLength = buffer.readUInt16LE(offset + 32);
    const external = buffer.readUInt32LE(offset + 38);
    const localOffset = buffer.readUInt32LE(offset + 42);
    const name = buffer.toString("utf8", offset + 46, offset + 46 + nameLength);
    offset += 46 + nameLength + extraLength + commentLength;
    if (flags & 0x1) {
      fail(`zip member ${name} is encrypted`);
    }
    if (buffer.readUInt32LE(localOffset) !== 0x04034b50) {
      fail(`zip member ${name} has no local header`);
    }
    const dataStart = localOffset + 30 + buffer.readUInt16LE(localOffset + 26) + buffer.readUInt16LE(localOffset + 28);
    const raw = buffer.subarray(dataStart, dataStart + compressedSize);
    let data;
    if (method === 0) {
      data = Buffer.from(raw);
    } else if (method === 8) {
      try {
        data = zlib.inflateRawSync(raw);
      } catch (error) {
        fail(`zip member ${name} does not inflate: ${error.message}`);
      }
    } else {
      fail(`zip member ${name} uses unsupported compression method ${method}`);
    }
    if (data.length !== size || (zlib.crc32(data) >>> 0) !== crc) {
      fail(`zip member ${name} fails its size or CRC-32 check`);
    }
    const unixMode = madeBy === 3 ? external >>> 16 : 0;
    const fileType = unixMode & 0o170000;
    const type = name.endsWith("/") || fileType === 0o040000 ? "directory" : fileType === 0o120000 ? "symlink" : "file";
    entries.push({ name, type, mode: unixMode & 0o7777, size, data, mtime: dosToEpoch(dosTime, dosDate) });
  }
  return entries;
}

/** Read an archive by its file name's extension. */
export function readArchive(name, buffer) {
  if (name.endsWith(".tar.gz")) {
    return readTarGz(buffer);
  }
  if (name.endsWith(".zip") || name.endsWith(".whl")) {
    return readZip(buffer);
  }
  fail(`${name} is not a .tar.gz, .zip or .whl archive`);
}
