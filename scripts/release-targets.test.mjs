import assert from "node:assert/strict";
import test from "node:test";

import {
  MACOS_DEPLOYMENT_TARGET,
  RELEASE_TARGETS,
  archiveMembers,
  archiveName,
  artifactById,
  buildEnvironment,
  buildMatrix,
  ccVariable,
  plannedArtifacts,
  plannedCrates,
  validateTargets,
  wheelName,
} from "./release-targets.mjs";

test("the matrix is exactly the plan's five archives and five wheels on native runners", () => {
  assert.deepEqual(validateTargets(), []);
  const artifacts = plannedArtifacts("0.1.0");
  assert.deepEqual(
    artifacts.map((artifact) => [artifact.name, artifact.runner]),
    [
      ["urollup-0.1.0-x86_64-unknown-linux-musl.tar.gz", "ubuntu-24.04"],
      ["urollup-0.1.0-py3-none-manylinux_2_17_x86_64.manylinux2014_x86_64.whl", "ubuntu-24.04"],
      ["urollup-0.1.0-aarch64-unknown-linux-musl.tar.gz", "ubuntu-24.04-arm"],
      ["urollup-0.1.0-py3-none-manylinux_2_17_aarch64.manylinux2014_aarch64.whl", "ubuntu-24.04-arm"],
      ["urollup-0.1.0-aarch64-apple-darwin.tar.gz", "macos-15"],
      ["urollup-0.1.0-py3-none-macosx_11_0_arm64.whl", "macos-15"],
      ["urollup-0.1.0-x86_64-apple-darwin.tar.gz", "macos-15-intel"],
      ["urollup-0.1.0-py3-none-macosx_11_0_x86_64.whl", "macos-15-intel"],
      ["urollup-0.1.0-x86_64-pc-windows-msvc.zip", "windows-2025"],
      ["urollup-0.1.0-py3-none-win_amd64.whl", "windows-2025"],
    ],
  );
  assert.equal(new Set(artifacts.map((artifact) => artifact.name)).size, 10);
  assert.deepEqual(plannedCrates("0.1.0").map((crate) => crate.name), ["urollup-core-0.1.0.crate", "urollup-0.1.0.crate"]);
});

test("a missing, renamed or relabeled target is refused", () => {
  const withoutWindows = RELEASE_TARGETS.filter((target) => target.os !== "windows");
  assert.match(validateTargets(withoutWindows).join("\n"), /release archives must be exactly/);
  assert.throws(() => plannedArtifacts("0.1.0", withoutWindows), /release wheels must be exactly/);
  const relabeled = RELEASE_TARGETS.map((target) =>
    target.id === "linux-x86_64" ? { ...target, wheel: { ...target.wheel, rustTarget: "x86_64-unknown-linux-musl" } } : target,
  );
  assert.match(validateTargets(relabeled).join("\n"), /genuine|glibc builds inside a pinned manylinux container/);
  const unpinned = RELEASE_TARGETS.map((target) =>
    target.id === "linux-aarch64" ? { ...target, wheel: { ...target.wheel, container: "quay.io/pypa/manylinux2014_aarch64:latest" } } : target,
  );
  assert.match(validateTargets(unpinned).join("\n"), /pinned by digest/);
  const duplicated = [...RELEASE_TARGETS, RELEASE_TARGETS[0]];
  assert.match(validateTargets(duplicated).join("\n"), /duplicate release target linux-x86_64/);
  const tarOnWindows = RELEASE_TARGETS.map((target) => (target.os === "windows" ? { ...target, archive: { ...target.archive, format: "tar.gz" } } : target));
  assert.match(validateTargets(tarOnWindows).join("\n"), /windows archives must be zip/);
});

test("names, members and executables follow the target", () => {
  assert.equal(archiveName("0.1.0", "x86_64-pc-windows-msvc", "zip"), "urollup-0.1.0-x86_64-pc-windows-msvc.zip");
  assert.equal(wheelName("0.2.0-rc.1", "win_amd64"), "urollup-0.2.0rc1-py3-none-win_amd64.whl");
  assert.deepEqual(archiveMembers("windows"), ["LICENSE", "README.md", "urollup.exe"]);
  assert.deepEqual(archiveMembers("linux"), ["LICENSE", "README.md", "urollup"]);
  assert.equal(artifactById("0.1.0", "wheel-windows-x86_64").executable, "urollup.exe");
  assert.throws(() => artifactById("0.1.0", "wheel-linux-riscv64"), /no planned release artifact/);
});

test("the build matrix carries one entry per artifact and pins musl only for Linux archives", () => {
  const matrix = buildMatrix("0.1.0");
  assert.equal(matrix.include.length, 10);
  const linuxArchive = matrix.include.find((entry) => entry.id === "archive-linux-x86_64");
  assert.equal(linuxArchive.apt_packages, "musl=1.2.4-2 musl-dev=1.2.4-2 musl-tools=1.2.4-2");
  assert.ok(matrix.include.filter((entry) => entry.id !== "archive-linux-x86_64" && entry.id !== "archive-linux-aarch64").every((entry) => entry.apt_packages === ""));
  assert.match(matrix.include.find((entry) => entry.id === "wheel-linux-aarch64").container, /^quay\.io\/pypa\/manylinux2014_aarch64@sha256:[0-9a-f]{64}$/);
});

test("build environments enforce the macOS floor, a static Windows C runtime and musl-gcc", () => {
  const split = (env) => env.CARGO_ENCODED_RUSTFLAGS.split("\x1f");
  const mac = buildEnvironment(artifactById("0.1.0", "archive-macos-x86_64"), { cargoHome: "/home/u/.cargo" });
  assert.equal(mac.MACOSX_DEPLOYMENT_TARGET, MACOS_DEPLOYMENT_TARGET);
  assert.deepEqual(split(mac), ["--remap-path-prefix=/home/u/.cargo=/cargo"]);
  const windows = buildEnvironment(artifactById("0.1.0", "wheel-windows-x86_64"), { cargoHome: "C:\\Users\\runner admin\\.cargo" });
  assert.deepEqual(split(windows), ["--remap-path-prefix=C:\\Users\\runner admin\\.cargo=/cargo", "-Ctarget-feature=+crt-static", "-Clink-arg=/Brepro"]);
  const musl = buildEnvironment(artifactById("0.1.0", "archive-linux-aarch64"), { cargoHome: "/c" });
  assert.equal(musl[ccVariable("aarch64-unknown-linux-musl")], "musl-gcc");
  assert.equal(ccVariable("aarch64-unknown-linux-musl"), "CC_aarch64_unknown_linux_musl");
  const manylinux = buildEnvironment(artifactById("0.1.0", "wheel-linux-x86_64"), { cargoHome: "/c" });
  assert.equal(Object.keys(manylinux).some((name) => name.startsWith("CC_")), false);
});
