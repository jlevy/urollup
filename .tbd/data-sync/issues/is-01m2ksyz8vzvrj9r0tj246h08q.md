---
type: is
id: is-01m2ksyz8vzvrj9r0tj246h08q
title: Build and smoke-test release targets on native runners
kind: task
status: open
priority: 1
version: 2
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - release
dependencies:
  - type: blocks
    target: is-01m2ksz138y5h2rwsesrv1eve7
parent_id: is-01m2ksyxh3c3qvgg8qhp5fgbsx
created_at: 2026-09-16T00:30:42.450Z
updated_at: 2026-09-16T00:30:44.327Z
---
Rollout: the platform matrix. Plan Rollout Plan; Decision 27; the baseline's targets, channels and versioning.

Acceptance:
- Static musl Linux x86_64 and arm64, macOS arm64 and x86_64, and Windows x86_64, each built with --locked and smoke-tested on a native runner.
- The musl build is benchmarked before choosing a global allocator; Windows arm64 waits for demand.
- The packaged-binary smoke test runs a real report over a fixture root, checks the version string, and runs the binary with the embedded web bundle.
- The release profile keeps unwinding so a panicking serve handler does not end the process.
