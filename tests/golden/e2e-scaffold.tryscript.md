---
sandbox: samples/fixtures/claude-project/block-records
path:
  - $UROLLUP_BIN
env:
  CLAUDE_CONFIG_DIR: .
  CODEX_HOME: $GOLDEN_EMPTY_ROOT
  PI_CODING_AGENT_SESSION_DIR: $GOLDEN_EMPTY_ROOT/sessions
---
# E2E Invocation Scaffold

Per-case goldens under `tests/golden/e2e/` run in exactly this shape: a sandbox copy of
one case directory, the case’s native discovery variable pointing at it, every other
agent’s root empty, and HOME hermetic ([README](README.md)). This session runs that
shape against a harness sample case, so the sandbox copy and front matter expansion are
proven on every CI platform before any report exists.

Until `report`, `daily` and `sessions` parse their format flags (bead uro-d135), the
end-to-end invocations are usage errors that exit 2 and print no data.
When they land, this session is replaced by the per-case goldens.

## Report JSON Is Not Available Yet

```console
$ urollup report --all --format json --timezone UTC
! error: unexpected argument '--format' found
!
! Usage: urollup report --all
!
! For more information, try '--help'.
? 2
```

## Daily JSON Is Not Available Yet

```console
$ urollup daily --all --format json --timezone UTC
! error: unexpected argument '--format' found
!
! Usage: urollup daily --all
!
! For more information, try '--help'.
? 2
```

## Sessions JSON Is Not Available Yet

```console
$ urollup sessions --all --format json --timezone UTC
! error: unexpected argument '--format' found
!
! Usage: urollup sessions --all
!
! For more information, try '--help'.
? 2
```

<!-- This document follows common-doc-guidelines.md.
See github.com/jlevy/practical-prose and review guidelines before editing.
-->
