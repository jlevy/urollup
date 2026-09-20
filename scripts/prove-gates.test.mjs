import assert from "node:assert/strict";
import { mkdirSync, mkdtempSync, readFileSync, rmSync, statSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

import { PROBE_DIR, applyEdits, checkPrerequisites, judge, parseArgs, validateManifest } from "./prove-gates.mjs";

const ROOT = dirname(dirname(fileURLToPath(import.meta.url)));

const MAKEFILE = `check: toolchain fmt-check \\
\tclippy test

fmt-check:
\tcargo fmt --all --check
`;

const probe = (overrides = {}) => ({
  id: "rustfmt",
  gate: "fmt-check",
  violation: "misformatted Rust",
  expect: "^Diff in",
  edits: [{ path: "src/lib.rs", append: "files/x.probe" }],
  ...overrides,
});

test("check prerequisites are read across continuation lines", () => {
  assert.deepEqual(checkPrerequisites(MAKEFILE), ["toolchain", "fmt-check", "clippy", "test"]);
  assert.throws(() => checkPrerequisites("build:\n\tcargo build\n"), /no check target/);
  assert.throws(() => checkPrerequisites("check:\n"), /no prerequisites/);
});

test("every gate needs a probe or a recorded reason", () => {
  const gates = ["fmt-check", "npm-audit"];
  assert.deepEqual(
    validateManifest({ probes: [probe()], unprobed: { "npm-audit": "needs a vulnerable package" } }, gates).length,
    1,
  );
  assert.throws(() => validateManifest({ probes: [probe()] }, gates), /gate npm-audit has no probe/);
  assert.throws(
    () => validateManifest({ probes: [probe()], unprobed: { "npm-audit": "x", "fmt-check": "y" } }, gates),
    /has a probe but is also listed as unprobed/,
  );
  assert.throws(
    () => validateManifest({ probes: [probe()], unprobed: { "npm-audit": "x", "lint": "y" } }, gates),
    /unprobed lists lint/,
  );
});

test("malformed probes are rejected before anything runs", () => {
  const gates = ["fmt-check"];
  assert.throws(() => validateManifest({ probes: [] }, gates), /no probes/);
  assert.throws(() => validateManifest({ probes: [probe(), probe()] }, gates), /duplicate probe id/);
  assert.throws(() => validateManifest({ probes: [probe({ gate: "lint" })] }, gates), /not a prerequisite/);
  assert.throws(() => validateManifest({ probes: [probe({ expect: "" })] }, gates), /non-empty expect/);
  assert.throws(() => validateManifest({ probes: [probe({ edits: [] })] }, gates), /has no edits/);
  assert.throws(
    () => validateManifest({ probes: [probe({ edits: [{ path: "a", append: "x", create: "y" }] })] }, gates),
    /exactly one of/,
  );
  assert.throws(
    () => validateManifest({ probes: [probe({ edits: [{ path: "../outside", append: "x" }] })] }, gates),
    /outside the repository copy/,
  );
});

test("the committed manifest covers every gate in the Makefile", () => {
  const manifest = JSON.parse(readFileSync(join(ROOT, PROBE_DIR, "probes.json"), "utf8"));
  const gates = checkPrerequisites(readFileSync(join(ROOT, "Makefile"), "utf8"));
  const probes = validateManifest(manifest, gates);
  for (const item of probes) {
    for (const edit of item.edits) {
      for (const kind of ["append", "create", "replace"]) {
        if (kind in edit) {
          assert.ok(statSync(join(ROOT, PROBE_DIR, edit[kind])).isFile(), `${item.id}: ${edit[kind]}`);
        }
      }
    }
  }
});

test("a gate that passes, or fails for another reason, does not prove the probe", () => {
  const item = probe();
  assert.equal(judge(item, { status: 1, stdout: "Diff in /x/src/lib.rs:3:\n", stderr: "" }).ok, true);
  assert.match(judge(item, { status: 0, stdout: "", stderr: "" }).reason, /passed despite the violation/);
  // A missing tool is a failure, but not the one the probe is about.
  assert.match(
    judge(item, { status: 127, stdout: "", stderr: "make: cargo: No such file or directory\n" }).reason,
    /without the expected diagnostic/,
  );
  assert.match(judge(item, { error: new Error("spawn make ENOENT") }).reason, /could not run the gate/);
});

test("edits apply exactly and refuse stale probes", () => {
  const scratch = mkdtempSync(join(tmpdir(), "urollup-prove-gates-test-"));
  try {
    const copy = join(scratch, "copy");
    const probes = join(scratch, "probes");
    mkdirSync(join(copy, "src"), { recursive: true });
    mkdirSync(probes);
    writeFileSync(join(copy, "src", "lib.rs"), "//! lib\n");
    writeFileSync(join(copy, "Cargo.toml"), "[lints]\nworkspace = true\n");
    writeFileSync(join(copy, "gone.md"), "x\n");
    writeFileSync(join(probes, "tail.probe"), "fn probe() {}\n");
    writeFileSync(join(probes, "tool.probe"), "#!/bin/sh\necho old\n");

    applyEdits(copy, probes, {
      id: "combined",
      edits: [
        { path: "src/lib.rs", append: "tail.probe" },
        { path: "bin/tool", create: "tool.probe", executable: true },
        { path: "Cargo.toml", substitute: { from: "[lints]\nworkspace = true\n", to: "" } },
        { path: "gone.md", delete: true },
      ],
    });
    assert.equal(readFileSync(join(copy, "src", "lib.rs"), "utf8"), "//! lib\nfn probe() {}\n");
    assert.equal(statSync(join(copy, "bin", "tool")).mode & 0o111, 0o111);
    assert.equal(readFileSync(join(copy, "Cargo.toml"), "utf8"), "");
    assert.throws(() => statSync(join(copy, "gone.md")), /ENOENT/);

    assert.throws(
      () => applyEdits(copy, probes, { id: "stale", edits: [{ path: "Cargo.toml", substitute: { from: "[lints]", to: "" } }] }),
      /expected exactly one "\[lints\]" in Cargo.toml, found 0/,
    );
    assert.throws(
      () => applyEdits(copy, probes, { id: "exists", edits: [{ path: "src/lib.rs", create: "tail.probe" }] }),
      /already exists/,
    );
    assert.throws(
      () => applyEdits(copy, probes, { id: "missing", edits: [{ path: "nope.rs", append: "tail.probe" }] }),
      /does not exist/,
    );
  } finally {
    rmSync(scratch, { recursive: true, force: true });
  }
});

test("arguments select probes explicitly", () => {
  assert.deepEqual(parseArgs([]), { only: [], list: false, keep: false });
  assert.deepEqual(parseArgs(["--only", "a", "--only", "b", "--keep"]), { only: ["a", "b"], list: false, keep: true });
  assert.throws(() => parseArgs(["--only"]), /usage/);
  assert.throws(() => parseArgs(["--all"]), /usage/);
});
