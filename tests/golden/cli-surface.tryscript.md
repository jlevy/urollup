---
sandbox: true
path:
  - $UROLLUP_BIN
env:
  CLAUDE_CONFIG_DIR: $GOLDEN_EMPTY_ROOT
  CODEX_HOME: $GOLDEN_EMPTY_ROOT
  LANG: C
  LC_ALL: C
  NO_COLOR: "1"
  PI_CODING_AGENT_SESSION_DIR: $GOLDEN_EMPTY_ROOT/sessions
  TZ: UTC
---
# CLI Surface

The milestone 0.1 CLI defines the command surface, selection defaults and exit contract
(design §6.5). Run these sessions with `make golden`; never regenerate them without
reading the diff.

## Version Is Exact and on Stdout

```console
$ urollup --version
urollup 0.1.0
? 0
```

## Help Lists the Milestone 0.1 Commands

```console
$ urollup --help
Trustworthy token and usage rollups from Claude Code and Codex session logs

Usage: urollup [OPTIONS] <COMMAND>

Commands:
  report    Session report: totals, breakdowns, sizes, tools and limitations
  daily     Calendar rollup by day
  sessions  One row per session
  help      Print this message or the help of the given subcommand(s)

Options:
      --color <COLOR>
          Colorize human output: auto, always or never

          Possible values:
          - auto:   Style human output only when its destination is a terminal
          - always: Style human output even when its destination is redirected
          - never:  Never style output

          [default: auto]

      --no-progress
          Disable the interactive progress indicator

  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version
? 0
```

## Report Defaults to the Current Session

```console
$ urollup report
! error: current session was not detected; use --session or --all
? 2
```

With no current-agent environment variable, the default is a usage error rather than an
implicit all-sessions report.

## Rollups Default to All Sessions

```console
$ urollup daily --timezone UTC
urollup daily
Selection all  Scope self  Timezone UTC

DATE | REQUESTS | UNCACHED | CACHE READ | CACHE WRITE | OUTPUT | TOTAL
? 0
```

```console
$ urollup sessions --timezone UTC
urollup sessions
Selection all  Scope self  Timezone UTC

THREAD | AGENT | PROJECT | REQUESTS | INPUT | OUTPUT | TOTAL
? 0
```

## Usage Errors Exit 2 on Stderr

```console
$ urollup --no-such-flag
! error: unexpected argument '--no-such-flag' found
!
! Usage: urollup [OPTIONS] <COMMAND>
!
! For more information, try '--help'.
? 2
```

A bare invocation is a usage error too: it prints help on stderr, not stdout.

```console
$ urollup
! Trustworthy token and usage rollups from Claude Code and Codex session logs
!
! Usage: urollup [OPTIONS] <COMMAND>
!
! Commands:
!   report    Session report: totals, breakdowns, sizes, tools and limitations
!   daily     Calendar rollup by day
!   sessions  One row per session
!   help      Print this message or the help of the given subcommand(s)
!
! Options:
!       --color <COLOR>  Colorize human output: auto, always or never [default: auto] [possible values: auto, always, never]
!       --no-progress    Disable the interactive progress indicator
!   -h, --help           Print help (see more with '--help')
!   -V, --version        Print version
? 2
```

<!-- This document follows common-doc-guidelines.md.
See github.com/jlevy/practical-prose and review guidelines before editing.
-->
