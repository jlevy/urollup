#!/usr/bin/env node
// Run the tryscript CLI golden corpus against the built binary, and prove which one ran.
//
// Adapted from fdu `scripts/run-golden.mjs` at afbb2ee; see PROVENANCE.md. urollup has one
// surface (the Rust binary), honors CARGO_TARGET_DIR, and refuses an empty corpus.
//
// Sessions invoke a bare `urollup`, resolved through `path: [$UROLLUP_BIN]` front matter,
// because a bare command name is the only form /bin/sh and cmd.exe read the same way.
// tryscript's `path:` prepends to PATH, so this runner preflights the binary first: a
// missing build must be a diagnosed failure, not a pass against an installed urollup
// (check-golden-invocations.mjs keeps every session on $UROLLUP_BIN).

import { spawnSync } from 'node:child_process';
import { accessSync, constants, readdirSync, statSync } from 'node:fs';
import { dirname, isAbsolute, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = dirname(dirname(fileURLToPath(import.meta.url)));
const exe = process.platform === 'win32' ? '.exe' : '';
const targetDir = process.env.CARGO_TARGET_DIR
  ? (isAbsolute(process.env.CARGO_TARGET_DIR)
      ? process.env.CARGO_TARGET_DIR
      : resolve(root, process.env.CARGO_TARGET_DIR))
  : join(root, 'target');
const binary = join(targetDir, 'debug', `urollup${exe}`);

try {
  if (!statSync(binary).isFile()) {
    throw new Error('not a regular file');
  }
  if (process.platform !== 'win32') {
    accessSync(binary, constants.X_OK);
  }
} catch (error) {
  console.error(`run-golden: the urollup binary is not runnable at ${binary}`);
  console.error(`run-golden: ${error.message}`);
  console.error('run-golden: build it with `make build`');
  process.exit(2);
}

// A corpus that matches nothing would let tryscript report success for zero sessions,
// which is indistinguishable from a passing run. A missing golden fails here instead.
const corpus = 'tests/golden';
const sessions = readdirSync(join(root, corpus)).filter((name) => name.endsWith('.tryscript.md'));
if (sessions.length === 0) {
  console.error(`run-golden: no *.tryscript.md sessions in ${corpus}; the golden corpus is missing`);
  process.exit(1);
}

console.log(`run-golden: ${sessions.length} sessions against ${binary}`);

// Resolved from the locked tree rather than from PATH, so the harness cannot run a
// tryscript nobody pinned.
const tryscript = join(root, 'node_modules', '.bin', `tryscript${process.platform === 'win32' ? '.cmd' : ''}`);
// Forward slashes even on Windows: this is a glob for tryscript, not a path for the OS.
const args = ['run', ...process.argv.slice(2), `${corpus}/*.tryscript.md`];
const result = spawnSync(tryscript, args, {
  cwd: root,
  stdio: 'inherit',
  shell: process.platform === 'win32',
  env: {
    ...process.env,
    UROLLUP_BIN: dirname(binary),
    UROLLUP: binary,
  },
});

if (result.error) {
  console.error(`run-golden: could not run ${tryscript}: ${result.error.message}`);
  console.error('run-golden: install the locked Node tools with `npm ci --ignore-scripts`');
  process.exit(1);
}
process.exit(result.status ?? 1);
