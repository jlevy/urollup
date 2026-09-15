#!/usr/bin/env node
// Confirm every workspace member declares a lint policy (rust-lint-format-rules, "Break
// Each Floor Rule Once to Prove It Runs").
//
// Workspace lints do not apply on their own: a member without a `[lints]` table is held to
// nothing, with a green build. This enumerates members from `cargo metadata` rather than a
// `crates/*` glob, which would miss a root package or an explicitly placed member, and it
// fails on zero members, which is what a broken metadata call looks like. Whether the
// declared floor is actually live is proved separately by the violation probes in
// `make gate-proofs`.

import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { pathToFileURL } from "node:url";

/** True when a manifest declares `[lints]` or one of its tool tables. */
export function declaresLints(manifestText) {
  return /^\s*\[lints(?:\.[A-Za-z0-9_-]+)?\]\s*(?:#.*)?$/m.test(manifestText);
}

/** Return workspace members, as `{ name, manifestPath }`, from `cargo metadata` JSON. */
export function workspaceMembers(metadata) {
  const members = new Set(metadata?.workspace_members ?? []);
  const packages = (metadata?.packages ?? []).filter((pkg) => members.has(pkg.id));
  if (packages.length === 0) {
    throw new Error("cargo metadata reported no workspace members; refusing to pass an empty workspace");
  }
  return packages.map((pkg) => ({ name: pkg.name, manifestPath: pkg.manifest_path }));
}

/** Return the names of members whose manifest declares no lint policy. */
export function membersWithoutLints(members, readManifest) {
  return members.filter((member) => !declaresLints(readManifest(member.manifestPath))).map((m) => m.name);
}

function main() {
  const cargo = process.env.CARGO ?? "cargo";
  const result = spawnSync(cargo, ["metadata", "--locked", "--no-deps", "--format-version", "1"], {
    encoding: "utf8",
    maxBuffer: 64 * 1024 * 1024,
  });
  if (result.error || result.status !== 0) {
    throw new Error(`cargo metadata failed: ${result.error?.message ?? result.stderr}`);
  }
  const members = workspaceMembers(JSON.parse(result.stdout));
  const missing = membersWithoutLints(members, (path) => readFileSync(path, "utf8"));
  if (missing.length > 0) {
    console.error(`lint-policy: members with no [lints] table: ${missing.join(", ")}`);
    console.error("Add `[lints]\\nworkspace = true` to each, so the workspace lint floor applies.");
    process.exitCode = 1;
    return;
  }
  console.log(`lint-policy: all ${members.length} workspace members declare a lint policy`);
}

if (import.meta.url === pathToFileURL(process.argv[1] ?? "").href) {
  try {
    main();
  } catch (error) {
    console.error(`lint-policy: ${error.message}`);
    process.exitCode = 1;
  }
}
