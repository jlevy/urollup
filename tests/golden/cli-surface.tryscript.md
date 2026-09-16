---
sandbox: true
path:
  - $UROLLUP_BIN
env:
  LANG: C
  LC_ALL: C
  NO_COLOR: "1"
  TZ: UTC
---
# CLI Surface

The milestone 0.1 scaffold defines the command surface and the exit contract (design
§6.5) before any report exists.
Run these sessions with `make golden`; never regenerate them without reading the diff.

## Version Is Exact and on Stdout

```console
$ urollup --version
urollup 0.1.0
? 0
```

## Help Lists the Milestone 0.1 Commands

```console
$ urollup --help
Trustworthy token, cost and usage rollups from Claude Code, Codex and Pi session logs

Usage: urollup <COMMAND>

Commands:
  report    Session report: totals, breakdowns, sizes, tools and limitations
  daily     Calendar rollup by day
  sessions  One row per session
  help      Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
? 0
```

## Unimplemented Commands Exit 2 With a Diagnostic and No Data

```console
$ urollup report
! error: `urollup report` is not implemented yet; this build is the repository scaffold
? 2
```

```console
$ urollup daily
! error: `urollup daily` is not implemented yet; this build is the repository scaffold
? 2
```

```console
$ urollup sessions
! error: `urollup sessions` is not implemented yet; this build is the repository scaffold
? 2
```

## Usage Errors Exit 2 on Stderr

```console
$ urollup --no-such-flag
! error: unexpected argument '--no-such-flag' found
!
! Usage: urollup <COMMAND>
!
! For more information, try '--help'.
? 2
```

A bare invocation is a usage error too: it prints help on stderr, not stdout.

```console
$ urollup
! Trustworthy token, cost and usage rollups from Claude Code, Codex and Pi session logs
!
! Usage: urollup <COMMAND>
!
! Commands:
!   report    Session report: totals, breakdowns, sizes, tools and limitations
!   daily     Calendar rollup by day
!   sessions  One row per session
!   help      Print this message or the help of the given subcommand(s)
!
! Options:
!   -h, --help     Print help
!   -V, --version  Print version
? 2
```

<!-- This document follows common-doc-guidelines.md.
See github.com/jlevy/practical-prose and review guidelines before editing.
-->
