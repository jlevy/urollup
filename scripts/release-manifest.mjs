// The release manifest, SHA256SUMS and draft release notes, assembled from validated
// artifacts.
//
// The manifest is immutable artifact identity: version, tag, commit, toolchain, and for
// every artifact its target, features, size, digest, build host and validation result.
// Mutable progress (which channels are published) lives in channel actions and, after
// publication, release-evidence.json; the two are never mixed in one file (plan, Artifact
// Matrix).

import { CARGO_PACKAGES, PRODUCT, RELEASE_TARGETS, plannedArtifacts, plannedCrates } from "./release-targets.mjs";

export const MANIFEST_SCHEMA = "urollup-release-manifest/v1";

function fail(message) {
  throw new Error(message);
}

const HEX64 = /^[0-9a-f]{64}$/;

/** Format a SHA256SUMS file (`<digest>  <name>`, sorted by name, as sha256sum -c reads it). */
export function formatChecksums(files) {
  const names = new Set();
  for (const file of files) {
    if (!HEX64.test(file.sha256 ?? "")) {
      fail(`${file.name} has no SHA-256 digest`);
    }
    if (names.has(file.name) || /[\s/\\]/.test(file.name)) {
      fail(`checksum entry ${JSON.stringify(file.name)} is duplicated or not a plain file name`);
    }
    names.add(file.name);
  }
  if (files.length === 0) {
    fail("refusing to write SHA256SUMS for an empty artifact set");
  }
  return [...files]
    .sort((a, b) => (a.name < b.name ? -1 : a.name > b.name ? 1 : 0))
    .map((file) => `${file.sha256}  ${file.name}\n`)
    .join("");
}

/** Parse a SHA256SUMS file into a name-to-digest map, refusing duplicates and malformed lines. */
export function parseChecksums(text) {
  const digests = new Map();
  for (const line of text.split(/\r?\n/)) {
    if (line === "") {
      continue;
    }
    const match = line.match(/^([0-9a-f]{64}) [ *](\S+)$/);
    if (!match) {
      fail(`SHA256SUMS line ${JSON.stringify(line)} is malformed`);
    }
    if (digests.has(match[2])) {
      fail(`SHA256SUMS lists ${match[2]} twice`);
    }
    digests.set(match[2], match[1]);
  }
  return digests;
}

/**
 * Compare files against a SHA256SUMS map: every listed file present with its digest, and no
 * unlisted file. A different digest under the same name is a conflict, never a pass.
 */
export function compareChecksums(expected, files) {
  const problems = [];
  const seen = new Set();
  for (const file of files) {
    seen.add(file.name);
    if (!expected.has(file.name)) {
      problems.push(`${file.name} is not listed in SHA256SUMS`);
    } else if (expected.get(file.name) !== file.sha256) {
      problems.push(`checksum conflict for ${file.name}: SHA256SUMS has ${expected.get(file.name)}, the file is ${file.sha256}`);
    }
  }
  for (const name of expected.keys()) {
    if (!seen.has(name)) {
      problems.push(`${name} is listed in SHA256SUMS but missing`);
    }
  }
  return problems;
}

/**
 * Check a set of built and validated artifacts against the plan: exactly the planned
 * archives and wheels, each validated on its native host with a matching digest, plus
 * exactly the planned Cargo packages. Returns problems; an empty set is itself a problem.
 */
export function checkCompleteness({ version, artifacts, validations, crates, targets = RELEASE_TARGETS }) {
  const problems = [];
  if (!Array.isArray(artifacts) || artifacts.length === 0) {
    return ["the release produced no artifacts; an empty set is never a release"];
  }
  const planned = plannedArtifacts(version, targets);
  const plannedNames = new Set(planned.map((artifact) => artifact.name));
  const names = artifacts.map((artifact) => artifact.name);
  for (const artifact of planned) {
    if (!names.includes(artifact.name)) {
      problems.push(`missing ${artifact.kind} ${artifact.name} (${artifact.rustTarget})`);
    }
  }
  for (const name of names) {
    if (!plannedNames.has(name)) {
      problems.push(`unexpected artifact ${name}`);
    }
  }
  const duplicates = names.filter((name, index) => names.indexOf(name) !== index);
  if (duplicates.length > 0) {
    problems.push(`duplicated artifacts: ${[...new Set(duplicates)].join(", ")}`);
  }
  for (const artifact of artifacts) {
    if (!HEX64.test(artifact.sha256 ?? "") || !(artifact.size > 0)) {
      problems.push(`${artifact.name} has no digest or is empty`);
      continue;
    }
    const record = validations.find((validation) => validation.artifact === artifact.name);
    if (!record) {
      problems.push(`${artifact.name} has no native validation record`);
    } else if (record.sha256 !== artifact.sha256) {
      problems.push(`${artifact.name} was validated as ${record.sha256}, but the assembled file is ${artifact.sha256}`);
    } else if (record.status !== "passed") {
      problems.push(`${artifact.name} did not pass validation on ${record.host}`);
    } else {
      const plan = planned.find((candidate) => candidate.name === artifact.name);
      if (plan && record.host !== plan.host) {
        problems.push(`${artifact.name} was validated on ${record.host}, not its native ${plan.host} host`);
      }
    }
  }
  const plannedCrateNames = plannedCrates(version).map((crate) => crate.name);
  const crateNames = (crates ?? []).map((crate) => crate.name);
  for (const name of plannedCrateNames) {
    if (!crateNames.includes(name)) {
      problems.push(`missing Cargo package ${name}`);
    }
  }
  for (const crate of crates ?? []) {
    if (!plannedCrateNames.includes(crate.name)) {
      problems.push(`unexpected Cargo package ${crate.name}`);
    }
    if (!HEX64.test(crate.sha256 ?? "")) {
      problems.push(`Cargo package ${crate.name} has no digest`);
    }
  }
  return problems;
}

/**
 * Build the immutable release manifest. `artifacts` carry `{name, sha256, size}` plus their
 * build record; `crates` carry `{name, crate, sha256, size}`.
 */
export function buildManifest({ identity, toolchain, sourceDateEpoch, workflowRun, artifacts, builds, validations, crates }) {
  const planned = plannedArtifacts(identity.version);
  const entries = planned.map((plan) => {
    const artifact = artifacts.find((candidate) => candidate.name === plan.name);
    const build = builds.find((candidate) => candidate.artifact === plan.name) ?? {};
    const validation = validations.find((candidate) => candidate.artifact === plan.name) ?? {};
    return {
      name: plan.name,
      kind: plan.kind,
      target: plan.rustTarget,
      platform: plan.host,
      ...(plan.platformTag ? { wheel_platform_tag: plan.platformTag } : {}),
      features: ["default"],
      size: artifact.size,
      sha256: artifact.sha256,
      executable_sha256: validation.facts?.executable_sha256 ?? null,
      build: {
        runner: plan.runner,
        runner_image: build.runner_image ?? null,
        container: plan.container ?? null,
        rustc: build.rustc ?? null,
        node: build.node ?? null,
        builder: build.builder ?? null,
      },
      validation: {
        host: validation.host ?? null,
        runner_image: validation.runner_image ?? null,
        status: validation.status ?? null,
        floor: validation.facts?.minimum_macos
          ? `macOS ${validation.facts.minimum_macos}`
          : validation.facts?.max_glibc
            ? `glibc ${validation.facts.max_glibc}`
            : plan.linkage,
      },
    };
  });
  return {
    schema: MANIFEST_SCHEMA,
    product: PRODUCT,
    version: identity.version,
    tag: identity.tag,
    pep440_version: identity.pep440,
    commit: identity.commit,
    mode: identity.mode,
    source_date_epoch: sourceDateEpoch,
    rust_toolchain: toolchain,
    workflow_run: workflowRun,
    artifacts: entries,
    cargo_packages: CARGO_PACKAGES.map((crateName) => {
      const crate = crates.find((candidate) => candidate.crate === crateName);
      return { name: crateName, version: identity.version, file: crate.name, size: crate.size, sha256: crate.sha256 };
    }),
  };
}

// ---------------------------------------------------------------------------------------
// Release notes

/**
 * The CHANGELOG.md section for one version: from its `## <version>` heading to the next
 * `## ` heading. Returns `{heading, body, released}`; `released` is false for a heading
 * that says Unreleased, which a tag publication refuses.
 */
export function changelogSection(changelog, version) {
  const lines = changelog.split(/\r?\n/);
  const escaped = version.replace(/[.]/g, "\\.");
  const start = lines.findIndex((line) => new RegExp(`^## \\[?${escaped}\\]?(?:\\s|$)`).test(line));
  if (start < 0) {
    fail(`CHANGELOG.md has no "## ${version}" section`);
  }
  let end = lines.findIndex((line, index) => index > start && /^## /.test(line));
  if (end < 0) {
    end = lines.length;
  }
  const heading = lines[start];
  const body = lines.slice(start + 1, end).join("\n").trim();
  const previous = lines.slice(end).some((line) => /^## \[?\d+\.\d+\.\d+/.test(line));
  return { heading, body, released: !/unreleased/i.test(heading), first: !previous };
}

/**
 * Problems with a version's notes. The first release describes the product as it ships, so
 * it must not list fixes to defects that were introduced and fixed before it existed.
 */
export function checkReleaseNotes(section, { mode }) {
  const problems = [];
  if (section.body === "") {
    problems.push(`the CHANGELOG.md section ${section.heading} is empty`);
  }
  if (section.first && /^#{3,4} (Fixed|Bug Fixes|Fixes)\b/im.test(section.body)) {
    problems.push("the first release's notes list fixes; describe the shipped feature set, not defects fixed before release");
  }
  if (mode === "publish" && !section.released) {
    problems.push(`the CHANGELOG.md heading "${section.heading}" still says Unreleased; date it before tagging`);
  }
  return problems;
}

/** Draft GitHub release notes: the CHANGELOG section, installation and verification. */
export function renderReleaseNotes({ identity, section, artifacts, repository }) {
  const archives = artifacts.filter((artifact) => artifact.kind === "archive");
  const wheels = artifacts.filter((artifact) => artifact.kind === "wheel");
  const lines = [
    section.body,
    "",
    "## Install",
    "",
    "```bash",
    `uvx --isolated urollup@${identity.pep440} --version`,
    `uv tool install urollup==${identity.pep440}`,
    `cargo install --locked urollup@${identity.version}`,
    "```",
    "",
    "The PyPI package is the compiled Rust executable in a binary wheel; it contains no Python code.",
    "",
    "## Verify a Download",
    "",
    "Check the checksum, then the build-provenance attestation, which must come from this release's own workflow, tag and commit:",
    "",
    "```bash",
    "sha256sum --check --ignore-missing SHA256SUMS",
    `gh attestation verify <file> --repo ${repository} \\`,
    `  --signer-workflow ${repository}/.github/workflows/release.yml \\`,
    `  --source-ref refs/tags/${identity.tag} --source-digest ${identity.commit} \\`,
    "  --deny-self-hosted-runners",
    "```",
    "",
    "## Assets",
    "",
    "| File | Target | SHA-256 |",
    "| --- | --- | --- |",
    ...[...archives, ...wheels].map((artifact) => `| \`${artifact.name}\` | \`${artifact.target}\` | \`${artifact.sha256}\` |`),
    "",
    `Built from commit \`${identity.commit}\`.`,
    "",
  ];
  return lines.join("\n");
}
