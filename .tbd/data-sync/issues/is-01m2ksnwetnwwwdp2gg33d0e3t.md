---
type: is
id: is-01m2ksnwetnwwwdp2gg33d0e3t
title: Verify snapshot change detection on Windows
kind: task
status: open
priority: 3
version: 1
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - milestone-0.5
  - testing
dependencies: []
parent_id: is-01m2ke5h6e0v5vq32nfbmy6rs2
created_at: 2026-09-16T00:25:44.619Z
updated_at: 2026-09-16T00:25:44.619Z
---
The reader in uro-26dh identifies a file by device and inode on Unix. Windows exposes the volume serial and file index only through the unstable windows_by_handle metadata, so on Windows FileIdentity carries None for both and a replacement is detected only by length, modification time and the first record's fingerprint.

Decide whether that is acceptable and documented, or whether the reader should call GetFileInformationByHandle through a small platform module, and run the reader's change-detection tests on a Windows CI runner: replacement, truncation, in-place mutation, a briefly absent path, and a vanished source. See crates/urollup-core/src/sources/reader.rs (device_of and inode_of).
