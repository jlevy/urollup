import assert from "node:assert/strict";
import test from "node:test";

import { declaresLints, membersWithoutLints, workspaceMembers } from "./check-lint-policy.mjs";

test("an inherited or local lint table counts, an inherited package field does not", () => {
  assert.equal(declaresLints("[package]\nname = \"a\"\n\n[lints]\nworkspace = true\n"), true);
  assert.equal(declaresLints("[lints.clippy]\npedantic = \"deny\"\n"), true);
  // The trap grep -L 'workspace = true' falls into: inherited fields are not a lint policy.
  assert.equal(declaresLints("[package]\nedition.workspace = true\n\n[dependencies]\nx = { workspace = true }\n"), false);
  assert.equal(declaresLints("[workspace.lints.rust]\nwarnings = \"deny\"\n"), false);
});

test("members come from cargo metadata, and none is an error", () => {
  const metadata = {
    workspace_members: ["a 0.1.0", "b 0.1.0"],
    packages: [
      { id: "a 0.1.0", name: "a", manifest_path: "/w/a/Cargo.toml" },
      { id: "b 0.1.0", name: "b", manifest_path: "/w/b/Cargo.toml" },
      { id: "dep 1.0.0", name: "dep", manifest_path: "/registry/dep/Cargo.toml" },
    ],
  };
  assert.deepEqual(workspaceMembers(metadata), [
    { name: "a", manifestPath: "/w/a/Cargo.toml" },
    { name: "b", manifestPath: "/w/b/Cargo.toml" },
  ]);
  assert.throws(() => workspaceMembers({ workspace_members: [], packages: [] }), /no workspace members/);
  assert.throws(() => workspaceMembers(undefined), /no workspace members/);
});

test("a member without a lint table is reported by name", () => {
  const manifests = {
    "/w/a/Cargo.toml": "[package]\nname = \"a\"\n[lints]\nworkspace = true\n",
    "/w/b/Cargo.toml": "[package]\nname = \"b\"\nedition.workspace = true\n",
  };
  const members = [
    { name: "a", manifestPath: "/w/a/Cargo.toml" },
    { name: "b", manifestPath: "/w/b/Cargo.toml" },
  ];
  assert.deepEqual(membersWithoutLints(members, (path) => manifests[path]), ["b"]);
});
