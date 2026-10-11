---
sandbox: ../../crates/urollup-core/tests/fixtures/claude-project/resumed-subagent-copy
path:
  - $UROLLUP_BIN
env:
  CLAUDE_CONFIG_DIR: .
  CODEX_HOME: $GOLDEN_EMPTY_ROOT
  PI_CODING_AGENT_SESSION_DIR: $GOLDEN_EMPTY_ROOT/sessions
---
# Exact Selection over a Copied Subagent

The `claude-project/resumed-subagent-copy` case: session `…001800000002` resumes
`…001800000001` and holds a byte-identical copy of its subagent `a0000000000180001`,
whose lines still name the first session.
Exact selection reads each subagent transcript’s first named session, so selecting
either session reads the copy, and each selection’s rows match the whole-history
`sessions` rows of the same threads.

## The Original Session

The first session and its subagent, whose responses the original transcript and the copy
record once.

```console
$ urollup sessions --session 00000000-0000-4000-8000-001800000001 --timezone UTC
urollup sessions
Selection session  Scope descendants  Timezone UTC

THREAD | AGENT | PROJECT | REQUESTS | INPUT | OUTPUT | TOTAL
thr-v1-5j1sbk2y2typqdg54bfwx47x4z | claude | project | 2 | 3,202 | 110 | 3,312
thr-v1-6qzh9abkv691cp7yw3pqhnsqat | claude | project | 2 | 1,815 | 65 | 1,880
? 0
```

## The Resumed Session

The resumed session and its own subagent; the copied subagent belongs to the first
session, so it is not in this subtree.

```console
$ urollup sessions --session 00000000-0000-4000-8000-001800000002 --timezone UTC
urollup sessions
Selection session  Scope descendants  Timezone UTC

THREAD | AGENT | PROJECT | REQUESTS | INPUT | OUTPUT | TOTAL
thr-v1-14m8m1hawwrznm6yx254p3ccja | claude | project | 1 | 609 | 33 | 642
thr-v1-70p6m29demj516fc2ztwdg1cvv | claude | project | 2 | 4,070 | 90 | 4,160
? 0
```

## The Copy’s Path

The copy holds only the first session’s subagent records, so its path names that
subagent.

```console
$ urollup sessions --session projects/-Users-example-project/00000000-0000-4000-8000-001800000002/subagents/agent-a0000000000180001.jsonl --timezone UTC
urollup sessions
Selection session  Scope descendants  Timezone UTC

THREAD | AGENT | PROJECT | REQUESTS | INPUT | OUTPUT | TOTAL
thr-v1-6qzh9abkv691cp7yw3pqhnsqat | claude | project | 2 | 1,815 | 65 | 1,880
? 0
```

<!-- This document follows common-doc-guidelines.md.
See github.com/jlevy/practical-prose and review guidelines before editing.
-->
