#!/usr/bin/env node
// Prove the serving boundary held: no HTTP or async-runtime crate in the core's dependency
// tree or in the executable built without default features (design §7.1, Decision 21).
//
// The `serve` feature is kept deletable rather than merely optional. Its HTTP, async and
// web-asset dependencies may enter only as `optional` dependencies enabled by that
// feature, so a crate from the denylist appearing in either tree below means the split
// has silently come undone. The core must also stay free of CLI crates (design §8.1).
//
// The trees are captured before they are tested, never piped into grep: a failing
// `cargo tree` would otherwise hand grep empty input and report success having checked
// nothing (fdu `make lib-only`, ci-and-gates-rules). Each tree must also contain its own
// root package, so an empty or truncated tree fails rather than passes.

import { spawnSync } from "node:child_process";
import { pathToFileURL } from "node:url";

//: Crates that must never be in a tree the guard inspects. A family entry ends in `-`
//: and matches every crate named with that prefix (`tokio-` matches `tokio-util`).
export const HTTP_AND_ASYNC = [
  "actix",
  "actix-",
  "async-executor",
  "async-io",
  "async-std",
  "axum",
  "axum-",
  "futures-executor",
  "h2",
  "http",
  "http-body",
  "hyper",
  "hyper-",
  "isahc",
  "mio",
  "poem",
  "reqwest",
  "rocket",
  "smol",
  "surf",
  "tide",
  "tokio",
  "tokio-",
  "tower",
  "tower-",
  "ureq",
  "warp",
];

export const CLI_CRATES = ["anyhow", "clap", "clap_builder", "clap_derive", "clap_lex"];

//: The trees the guard inspects, each with the cargo arguments that produce it.
//
// `-e normal` keeps build-only and dev-only crates out of the question: they never reach
// the shipped binary. `--target all` includes platform-gated dependencies, which a tree
// for the host target alone would miss.
export const TREES = [
  {
    label: "urollup-core (all features)",
    root: "urollup-core",
    args: ["tree", "--locked", "-p", "urollup-core", "--all-features"],
    denied: [...HTTP_AND_ASYNC, ...CLI_CRATES],
  },
  {
    label: "urollup --no-default-features",
    root: "urollup",
    args: ["tree", "--locked", "-p", "urollup", "--no-default-features"],
    denied: HTTP_AND_ASYNC,
  },
];

const COMMON_TREE_ARGS = ["-e", "normal", "--target", "all", "--prefix", "none"];

/** Parse `cargo tree --prefix none` output into unique crate names. */
export function parseTree(text) {
  const names = new Set();
  for (const line of text.split("\n")) {
    const match = line.trim().match(/^([A-Za-z0-9_-]+) v\S+/);
    if (match) {
      names.add(match[1]);
    }
  }
  return names;
}

function isDenied(name, denied) {
  return denied.some((entry) =>
    entry.endsWith("-") ? name.startsWith(entry) : name === entry,
  );
}

/** Return the denied crates in one tree, or throw if the tree cannot be trusted. */
export function findViolations(tree, text) {
  const names = parseTree(text);
  if (!names.has(tree.root)) {
    throw new Error(
      `${tree.label}: cargo tree output does not contain ${tree.root}; refusing to pass an empty or unrelated tree`,
    );
  }
  return [...names].filter((name) => isDenied(name, tree.denied)).sort();
}

function cargoTree(tree, cargo) {
  const result = spawnSync(cargo, [...tree.args, ...COMMON_TREE_ARGS], {
    encoding: "utf8",
    maxBuffer: 16 * 1024 * 1024,
  });
  if (result.error) {
    throw new Error(`${tree.label}: could not run ${cargo}: ${result.error.message}`);
  }
  if (result.status !== 0) {
    throw new Error(`${tree.label}: cargo tree failed (exit ${result.status}):\n${result.stderr}`);
  }
  return result.stdout;
}

function main() {
  const cargo = process.env.CARGO ?? "cargo";
  const failures = [];
  for (const tree of TREES) {
    const violations = findViolations(tree, cargoTree(tree, cargo));
    if (violations.length > 0) {
      failures.push(`${tree.label}: ${violations.join(", ")}`);
    } else {
      console.log(`dependency-guard: ${tree.label} is clear`);
    }
  }
  if (failures.length > 0) {
    console.error("dependency-guard: denied crates in a tree that must stay free of them:");
    for (const failure of failures) {
      console.error(`  ${failure}`);
    }
    console.error(
      "HTTP and async-runtime crates belong only behind the optional `serve` feature " +
        "(`dep:` entries); CLI crates never belong in urollup-core.",
    );
    process.exitCode = 1;
  }
}

if (import.meta.url === pathToFileURL(process.argv[1] ?? "").href) {
  try {
    main();
  } catch (error) {
    console.error(`dependency-guard: ${error.message}`);
    process.exitCode = 1;
  }
}
