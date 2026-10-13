// The release artifact matrix: which Rust target each channel ships, which native runner
// builds and validates it, and every name derived from them
// (docs/project/specs/active/plan-2026-09-16-first-release-publishing.md, Artifact Matrix).
//
// This table is the one place the matrix is written down. The release workflow takes its
// build and validation matrices from `buildMatrix`, the assemble step requires exactly
// `plannedArtifacts`, and the supply-chain gate verifies the container images and apt
// packages pinned here (supply-chain-policy.json).

import { pep440Version } from "./release-identity.mjs";

export const PRODUCT = "urollup";

/**
 * PyPA manylinux2014 images (glibc 2.17), pinned by digest. Linux wheels are built inside
 * them so they are genuine manylinux_2_17 builds rather than relabeled host binaries.
 */
export const MANYLINUX_IMAGES = {
  x86_64: "quay.io/pypa/manylinux2014_x86_64@sha256:6f74cabeac2432570aa4bfdb29f7c1f30313d4d6654d764c44e574b6ffdd4ed5",
  aarch64: "quay.io/pypa/manylinux2014_aarch64@sha256:d36e257b4f7b1130a1442a5cd28022307645a92b8093ab65543205426354cfd2",
};

/**
 * Ubuntu 24.04 packages for the static musl archives: `musl-gcc` compiles the bundled zstd
 * C sources against musl. Exact versions from the noble release pocket.
 */
export const MUSL_APT_PACKAGES = ["musl=1.2.4-2", "musl-dev=1.2.4-2", "musl-tools=1.2.4-2"];

/** The macOS floor for both architectures, archives and wheels alike. */
export const MACOS_DEPLOYMENT_TARGET = "11.0";

/** The newest glibc symbol version a manylinux_2_17 executable may require. */
export const MANYLINUX_GLIBC = "2.17";

/**
 * One row per host. `archive` is the GitHub Release archive; `wheel` is the PyPI wheel,
 * which on Linux is a separate glibc build. Both are built and validated on `runner`.
 */
export const RELEASE_TARGETS = [
  {
    id: "linux-x86_64",
    os: "linux",
    arch: "x86_64",
    runner: "ubuntu-24.04",
    archive: { rustTarget: "x86_64-unknown-linux-musl", format: "tar.gz", linkage: "static-musl" },
    wheel: {
      rustTarget: "x86_64-unknown-linux-gnu",
      platformTag: "manylinux_2_17_x86_64.manylinux2014_x86_64",
      container: MANYLINUX_IMAGES.x86_64,
      linkage: "manylinux",
    },
  },
  {
    id: "linux-aarch64",
    os: "linux",
    arch: "aarch64",
    runner: "ubuntu-24.04-arm",
    archive: { rustTarget: "aarch64-unknown-linux-musl", format: "tar.gz", linkage: "static-musl" },
    wheel: {
      rustTarget: "aarch64-unknown-linux-gnu",
      platformTag: "manylinux_2_17_aarch64.manylinux2014_aarch64",
      container: MANYLINUX_IMAGES.aarch64,
      linkage: "manylinux",
    },
  },
  {
    id: "macos-aarch64",
    os: "macos",
    arch: "aarch64",
    runner: "macos-15",
    archive: { rustTarget: "aarch64-apple-darwin", format: "tar.gz", linkage: "macos" },
    wheel: { rustTarget: "aarch64-apple-darwin", platformTag: "macosx_11_0_arm64", linkage: "macos" },
  },
  {
    id: "macos-x86_64",
    os: "macos",
    arch: "x86_64",
    runner: "macos-15-intel",
    archive: { rustTarget: "x86_64-apple-darwin", format: "tar.gz", linkage: "macos" },
    wheel: { rustTarget: "x86_64-apple-darwin", platformTag: "macosx_11_0_x86_64", linkage: "macos" },
  },
  {
    id: "windows-x86_64",
    os: "windows",
    arch: "x86_64",
    runner: "windows-2025",
    archive: { rustTarget: "x86_64-pc-windows-msvc", format: "zip", linkage: "windows-static-crt" },
    wheel: { rustTarget: "x86_64-pc-windows-msvc", platformTag: "win_amd64", linkage: "windows-static-crt" },
  },
];

/** The archive targets the plan requires; a table that drops one is refused. */
export const REQUIRED_ARCHIVE_TARGETS = [
  "x86_64-unknown-linux-musl",
  "aarch64-unknown-linux-musl",
  "aarch64-apple-darwin",
  "x86_64-apple-darwin",
  "x86_64-pc-windows-msvc",
];

/** The wheel platform tags the plan requires. */
export const REQUIRED_WHEEL_PLATFORMS = [
  "manylinux_2_17_x86_64.manylinux2014_x86_64",
  "manylinux_2_17_aarch64.manylinux2014_aarch64",
  "macosx_11_0_arm64",
  "macosx_11_0_x86_64",
  "win_amd64",
];

/** The crates `cargo package --workspace` must produce, in publication order. */
export const CARGO_PACKAGES = ["urollup-core", "urollup"];

export function executableName(os) {
  return os === "windows" ? `${PRODUCT}.exe` : PRODUCT;
}

export function archiveName(version, rustTarget, format) {
  return `${PRODUCT}-${version}-${rustTarget}.${format}`;
}

export function wheelName(version, platformTag) {
  return `${PRODUCT}-${pep440Version(version)}-py3-none-${platformTag}.whl`;
}

export function crateName(name, version) {
  return `${name}-${version}.crate`;
}

/** The files every archive holds, and nothing else: no directory, no nesting. */
export function archiveMembers(os) {
  return [executableName(os), "README.md", "LICENSE"].sort();
}

function fail(message) {
  throw new Error(message);
}

/** Refuse a target table that differs from the plan's required matrix. */
export function validateTargets(targets = RELEASE_TARGETS) {
  const problems = [];
  const ids = new Set();
  for (const target of targets) {
    if (ids.has(target.id)) {
      problems.push(`duplicate release target ${target.id}`);
    }
    ids.add(target.id);
    if (!["linux", "macos", "windows"].includes(target.os)) {
      problems.push(`${target.id}: unknown os ${target.os}`);
    }
    if (target.archive?.format !== (target.os === "windows" ? "zip" : "tar.gz")) {
      problems.push(`${target.id}: ${target.os} archives must be ${target.os === "windows" ? "zip" : "tar.gz"}`);
    }
    if (target.os === "linux" && !target.archive.rustTarget.endsWith("-linux-musl")) {
      problems.push(`${target.id}: Linux archives must be static musl builds`);
    }
    if (target.os === "linux" && (!target.wheel.rustTarget.endsWith("-linux-gnu") || !target.wheel.container)) {
      problems.push(`${target.id}: Linux wheels must be glibc builds inside a pinned manylinux container`);
    }
    if (target.wheel?.container && !/@sha256:[0-9a-f]{64}$/.test(target.wheel.container)) {
      problems.push(`${target.id}: the manylinux container must be pinned by digest`);
    }
  }
  const archives = targets.map((target) => target.archive?.rustTarget).sort();
  const wheels = targets.map((target) => target.wheel?.platformTag).sort();
  if (JSON.stringify(archives) !== JSON.stringify([...REQUIRED_ARCHIVE_TARGETS].sort())) {
    problems.push(`release archives must be exactly ${REQUIRED_ARCHIVE_TARGETS.join(", ")}; found ${archives.join(", ")}`);
  }
  if (JSON.stringify(wheels) !== JSON.stringify([...REQUIRED_WHEEL_PLATFORMS].sort())) {
    problems.push(`release wheels must be exactly ${REQUIRED_WHEEL_PLATFORMS.join(", ")}; found ${wheels.join(", ")}`);
  }
  return problems;
}

/**
 * Every artifact a release must produce, with its build and validation host. The assemble
 * step requires exactly this set: a missing or extra artifact stops the release.
 */
export function plannedArtifacts(version, targets = RELEASE_TARGETS) {
  const problems = validateTargets(targets);
  if (problems.length > 0) {
    fail(problems.join("\n"));
  }
  const artifacts = [];
  for (const target of targets) {
    artifacts.push({
      kind: "archive",
      id: `archive-${target.id}`,
      host: target.id,
      os: target.os,
      arch: target.arch,
      runner: target.runner,
      rustTarget: target.archive.rustTarget,
      linkage: target.archive.linkage,
      name: archiveName(version, target.archive.rustTarget, target.archive.format),
      executable: executableName(target.os),
    });
    artifacts.push({
      kind: "wheel",
      id: `wheel-${target.id}`,
      host: target.id,
      os: target.os,
      arch: target.arch,
      runner: target.runner,
      rustTarget: target.wheel.rustTarget,
      linkage: target.wheel.linkage,
      platformTag: target.wheel.platformTag,
      container: target.wheel.container ?? null,
      name: wheelName(version, target.wheel.platformTag),
      executable: executableName(target.os),
    });
  }
  return artifacts;
}

/** The planned Cargo packages, in the order crates.io must receive them. */
export function plannedCrates(version) {
  return CARGO_PACKAGES.map((name) => ({ kind: "crate", id: `crate-${name}`, crate: name, name: crateName(name, version) }));
}

/** The GitHub Actions matrix for build and validation jobs: one entry per artifact. */
export function buildMatrix(version, targets = RELEASE_TARGETS) {
  return {
    include: plannedArtifacts(version, targets).map((artifact) => ({
      id: artifact.id,
      kind: artifact.kind,
      runner: artifact.runner,
      rust_target: artifact.rustTarget,
      artifact: artifact.name,
      container: artifact.container ?? "",
      apt_packages: artifact.kind === "archive" && artifact.os === "linux" ? MUSL_APT_PACKAGES.join(" ") : "",
    })),
  };
}

/** Find a planned artifact by its id, or fail. */
export function artifactById(version, id, targets = RELEASE_TARGETS) {
  const artifact = plannedArtifacts(version, targets).find((candidate) => candidate.id === id);
  if (!artifact) {
    fail(`no planned release artifact has id ${JSON.stringify(id)}`);
  }
  return artifact;
}

/** The C compiler environment variable cc-rs reads for one target, such as CC_x86_64_unknown_linux_musl. */
export function ccVariable(rustTarget) {
  return `CC_${rustTarget.replace(/-/g, "_")}`;
}

/**
 * Compiler flags and environment for one artifact's Rust build.
 *
 * - Every build remaps the Cargo home, so registry paths in panic locations do not name the
 *   build machine and two builds of one commit can match.
 * - macOS sets and later verifies the 11.0 deployment floor.
 * - Windows links the C runtime statically, so the executable needs no Visual C++
 *   redistributable, and asks the linker for a reproducible header timestamp.
 * - musl archives compile the bundled C sources with musl-gcc.
 */
export function buildEnvironment(artifact, { cargoHome }) {
  const rustflags = [`--remap-path-prefix=${cargoHome}=/cargo`];
  const env = {};
  if (artifact.os === "macos") {
    env.MACOSX_DEPLOYMENT_TARGET = MACOS_DEPLOYMENT_TARGET;
  }
  if (artifact.os === "windows") {
    rustflags.push("-Ctarget-feature=+crt-static", "-Clink-arg=/Brepro");
  }
  if (artifact.linkage === "static-musl") {
    env[ccVariable(artifact.rustTarget)] = "musl-gcc";
  }
  // The encoded form survives a Cargo home path with spaces, and takes precedence over any
  // RUSTFLAGS a runner sets.
  env.CARGO_ENCODED_RUSTFLAGS = rustflags.join("\x1f");
  return env;
}
