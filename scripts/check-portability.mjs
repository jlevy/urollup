#!/usr/bin/env node
// Refuse committed golden data that only reproduces on the machine that wrote it.
//
// Adapted from fdu `scripts/check-portability.mjs` at afbb2ee; see PROVENANCE.md.
//
// `tryscript run --update` rewrites expectations with the actual output, which expands
// named patterns into literals. A regenerated golden can then carry a home directory or a
// sandbox path and pass only on the recording machine, which is also the machine checking
// it. This check does not depend on where it runs. It also keeps private local paths,
// which urollup's fixtures must never contain, out of committed test data.

import { readdirSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = dirname(dirname(fileURLToPath(import.meta.url)));

// A path rooted in somebody's filesystem, a sandbox directory, or a Windows drive.
const MACHINE_SPECIFIC = [
  { pattern: /\/(?:Users|home)\/[A-Za-z0-9._-]+\//, why: 'a home directory from the recording machine' },
  { pattern: /\/(?:private\/)?(?:var|tmp)\/[A-Za-z0-9._\/-]*(?:tryscript|urollup)-[A-Za-z0-9]{6,}/, why: 'a sandbox directory that will not exist again' },
  { pattern: /\b[A-Z]:\\(?:Users|a)\\/, why: 'a Windows path from the recording machine' },
];

const goldenDir = join(root, 'tests', 'golden');
const targets = readdirSync(goldenDir)
  .filter((f) => f.endsWith('.tryscript.md'))
  .map((name) => [join('tests', 'golden', name), join(goldenDir, name)]);

const findings = [];
for (const [name, file] of targets) {
  readFileSync(file, 'utf8')
    .split('\n')
    .forEach((line, index) => {
      for (const { pattern, why } of MACHINE_SPECIFIC) {
        if (pattern.test(line)) {
          findings.push(`${name}:${index + 1}: ${why}\n      ${line.trim().slice(0, 120)}`);
        }
      }
    });
}

if (targets.length === 0) {
  findings.push('tests/golden: no *.tryscript.md sessions to check');
}

if (findings.length > 0) {
  console.error('committed test data must not name the machine that recorded it:\n');
  for (const finding of findings) {
    console.error(`  ${finding}`);
  }
  console.error('\nMask it with a named pattern. If `tryscript run --update` expanded one,');
  console.error('put the pattern back by hand: --update writes what it saw, not what varies.');
  process.exit(1);
}

console.log(`portability ok: no machine-specific paths in ${targets.length} files`);
