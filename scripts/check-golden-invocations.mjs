#!/usr/bin/env node
// Keep the golden corpus from resolving urollup anywhere but the build under test.
//
// Adapted from fdu `scripts/check-golden-invocations.mjs` at afbb2ee; see PROVENANCE.md.
//
// tryscript's `path:` front matter prepends to the inherited PATH rather than replacing
// it. If a session's entry fails to resolve, lookup continues into PATH and finds whatever
// urollup is installed, and the suite passes while testing a different binary. So every
// session declares exactly one path entry, the variable run-golden.mjs sets after
// preflighting the binary.

import { readdirSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = dirname(dirname(fileURLToPath(import.meta.url)));
const goldenDir = join(root, 'tests', 'golden');

const findings = [];
const sessions = readdirSync(goldenDir).filter((f) => f.endsWith('.tryscript.md'));
for (const name of sessions) {
  const file = join('tests', 'golden', name);
  const lines = readFileSync(join(goldenDir, name), 'utf8').split(/\r?\n/);
  const start = lines.indexOf('path:');
  if (start === -1) {
    findings.push(`${file}: no \`path:\` entry, so urollup resolves from the inherited PATH`);
    continue;
  }
  const entries = [];
  for (let i = start + 1; i < lines.length && lines[i].startsWith('  - '); i += 1) {
    entries.push(lines[i].slice(4).trim());
  }
  if (entries.length !== 1 || entries[0] !== '$UROLLUP_BIN') {
    findings.push(`${file}:${start + 1}: path must be exactly [$UROLLUP_BIN], found [${entries.join(', ')}]`);
  }
}

if (sessions.length === 0) {
  findings.push('tests/golden: no *.tryscript.md sessions; the golden corpus is missing');
}

if (findings.length > 0) {
  console.error('golden corpus must resolve urollup only from the build under test:\n');
  for (const finding of findings) {
    console.error(`  ${finding}`);
  }
  process.exit(1);
}

console.log(`golden invocations ok: ${sessions.length} sessions pin $UROLLUP_BIN`);
