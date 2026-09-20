import assert from "node:assert/strict";
import test from "node:test";

import { TREES, findViolations, parseTree } from "./check-dependency-guard.mjs";

const [CORE, CLI] = TREES;

const CLEAN_CLI_TREE = `urollup v0.1.0 (/work/crates/urollup)
clap v4.6.6
anstyle v1.0.14
clap_builder v4.6.6
clap_derive v4.6.4 (proc-macro)
urollup-core v0.1.0 (/work/crates/urollup-core)
`;

test("parses crate names from cargo tree output", () => {
  assert.deepEqual(
    [...parseTree(CLEAN_CLI_TREE)].sort(),
    ["anstyle", "clap", "clap_builder", "clap_derive", "urollup", "urollup-core"],
  );
});

test("a clean executable tree passes", () => {
  assert.deepEqual(findViolations(CLI, CLEAN_CLI_TREE), []);
});

test("an async runtime or HTTP crate in the no-default-features tree fails", () => {
  const tree = `${CLEAN_CLI_TREE}tokio v1.47.1\ntokio-macros v2.5.0 (proc-macro)\nhyper-util v0.1.16\n`;
  assert.deepEqual(findViolations(CLI, tree), ["hyper-util", "tokio", "tokio-macros"]);
});

test("family prefixes do not match unrelated crates", () => {
  const tree = `${CLEAN_CLI_TREE}httparse v1.10.0\ntowers v0.1.0\nmiow v0.6.0\n`;
  assert.deepEqual(findViolations(CLI, tree), []);
});

test("CLI crates are denied in the core but allowed in the executable", () => {
  const coreTree = "urollup-core v0.1.0 (/work/crates/urollup-core)\nclap v4.6.6\n";
  assert.deepEqual(findViolations(CORE, coreTree), ["clap"]);
  assert.deepEqual(findViolations(CLI, CLEAN_CLI_TREE), []);
});

test("an empty or unrelated tree fails instead of passing", () => {
  assert.throws(() => findViolations(CORE, ""), /does not contain urollup-core/);
  assert.throws(() => findViolations(CLI, "error: package ID specification `urollup` did not match\n"), /does not contain urollup/);
});
