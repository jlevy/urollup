---
type: is
id: is-01m2ksyz8vzvrj9r0tj246h08q
title: Build and smoke-test release targets on native runners
kind: task
status: open
priority: 1
version: 6
spec_path: docs/project/specs/active/plan-2026-09-16-first-release-publishing.md
labels:
  - release
dependencies:
  - type: blocks
    target: is-01m2ksz138y5h2rwsesrv1eve7
parent_id: is-01m2ksyxh3c3qvgg8qhp5fgbsx
created_at: 2026-09-16T00:30:42.450Z
updated_at: 2026-09-16T18:22:59.800Z
---
Build and validate the first public release artifact matrix.\n\nAcceptance:\n- GitHub archives contain static-musl Linux x86_64 and arm64, macOS Intel and Apple-silicon, and Windows x86_64 binaries. Each uses locked inputs and is smoke-tested on a matching native host.\n- Both macOS archives and wheels set and verify a macOS 11.0 deployment floor. Linux wheels are genuine manylinux_2_17 x86_64 and arm64 builds, distinct from the static-musl archives.\n- PyPI wheels contain the real Rust executable through Maturin bindings=bin. Tests inspect the wheel script payload and RECORD, install it in an empty environment, verify urollup resolves to that wheel binary, and run --version plus an explicit-source JSON report.\n- Tests cannot read real agent logs or reach a preinstalled urollup. The musl build is benchmarked before choosing a global allocator. Musllinux wheels and Windows arm64 wait for demand.
