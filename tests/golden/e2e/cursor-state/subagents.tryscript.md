---
sandbox: ../../../../crates/urollup-core/tests/fixtures/cursor-state/subagents
path:
  - $UROLLUP_BIN
env:
  CLAUDE_CONFIG_DIR: $GOLDEN_EMPTY_ROOT
  CODEX_HOME: $GOLDEN_EMPTY_ROOT
  PI_CODING_AGENT_SESSION_DIR: $GOLDEN_EMPTY_ROOT/sessions
  UROLLUP_CURSOR_DIRS: .
---
# E2E: cursor-state/subagents

The fixture case
[`cursor-state/subagents`](../../../../crates/urollup-core/tests/fixtures/cursor-state/subagents/)
read through `UROLLUP_CURSOR_DIRS` from a sandbox copy, with every other discovery root
empty and HOME hermetic.
`make e2e-results` checks its reconciled results against `expected.json`; this session
records the complete output of each view for review.

## Report

```console
$ urollup report --all --timezone UTC
urollup report
Selection all  Scope self  Timezone UTC

TOTALS
Requests  3
Owned     3
Ambiguous 0
Unknown   0
Uncached input 13
Cache read     -
Cache write    -
Output         4
Reasoning      -
Total tokens   17

COVERAGE
Status complete  Copies excluded 0  Limit observations 0
Unresolved 0  Possible 0  Requests without usage 0

REQUEST SIZES (inclusive input tokens)
Count 3  p50 4  p90 6  p99 6  max 6

ACCOUNT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
unknown | 3 | 13 | 4 | 17

EFFORT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
style-2 | 1 | 4 | 1 | 5
unknown | 2 | 9 | 3 | 12

MODEL BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
claude-4.5-opus-high-thinking | 1 | 4 | 1 | 5
cursor-grok-4.6-xhigh-fast | 1 | 6 | 2 | 8
gemini-3-pro | 1 | 3 | 1 | 4

PROJECT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
unknown | 3 | 13 | 4 | 17
? 0
```

## Report JSON

```console
$ urollup report --all --format json --timezone UTC
{
  "schema_version": 1,
  "query": {
    "command": "report",
    "selection": "all",
    "scope": "self",
    "timezone": "UTC"
  },
  "coverage": {
    "copies_excluded": 0,
    "limit_observations": 0,
    "requests_without_usage": 0,
    "complete": true
  },
  "diagnostics": [],
  "totals": {
    "requests": {
      "owned": 3,
      "ambiguous": 0,
      "unknown": 0
    },
    "tokens": {
      "uncached_input": 13,
      "output": 4,
      "total": 17
    },
    "unresolved": {
      "requests": 0,
      "tokens": {}
    },
    "possible": {
      "requests": 0,
      "tokens": {}
    }
  },
  "breakdowns": {
    "account": [
      {
        "group": "account",
        "value": "unknown",
        "requests": {
          "owned": 3,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 13,
          "output": 4,
          "total": 17
        }
      }
    ],
    "effort": [
      {
        "group": "effort",
        "value": "style-2",
        "requests": {
          "owned": 1,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 4,
          "output": 1,
          "total": 5
        }
      },
      {
        "group": "effort",
        "value": "unknown",
        "requests": {
          "owned": 2,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 9,
          "output": 3,
          "total": 12
        }
      }
    ],
    "model": [
      {
        "group": "model",
        "value": "claude-4.5-opus-high-thinking",
        "requests": {
          "owned": 1,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 4,
          "output": 1,
          "total": 5
        }
      },
      {
        "group": "model",
        "value": "cursor-grok-4.6-xhigh-fast",
        "requests": {
          "owned": 1,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 6,
          "output": 2,
          "total": 8
        }
      },
      {
        "group": "model",
        "value": "gemini-3-pro",
        "requests": {
          "owned": 1,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 3,
          "output": 1,
          "total": 4
        }
      }
    ],
    "project": [
      {
        "group": "project",
        "value": "unknown",
        "requests": {
          "owned": 3,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 13,
          "output": 4,
          "total": 17
        }
      }
    ]
  },
  "sizes": {
    "count": 3,
    "p50": 4,
    "p90": 6,
    "p99": 6,
    "max": 6
  }
}
? 0
```

## Daily

```console
$ urollup daily --all --timezone UTC
urollup daily
Selection all  Scope self  Timezone UTC

DATE | REQUESTS | UNCACHED | CACHE READ | CACHE WRITE | OUTPUT | TOTAL
2024-09-19 | 3 | 13 | - | - | 4 | 17
? 0
```

## Daily JSON

```console
$ urollup daily --all --format json --timezone UTC
{
  "schema_version": 1,
  "query": {
    "command": "daily",
    "selection": "all",
    "scope": "self",
    "timezone": "UTC"
  },
  "rows": [
    {
      "date": "2024-09-19",
      "requests": {
        "owned": 3,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 13,
        "output": 4,
        "total": 17
      }
    }
  ],
  "diagnostics": []
}
? 0
```

## Sessions

```console
$ urollup sessions --all --timezone UTC
urollup sessions
Selection all  Scope self  Timezone UTC

THREAD | AGENT | PROJECT | REQUESTS | INPUT | OUTPUT | TOTAL
thr-v1-172efbmpfmvk15kjws2nmc0h6j | cursor | unknown | 1 | 4 | 1 | 5
thr-v1-24r2damwd05hvchfcymd1vhqd2 | cursor | unknown | 1 | 6 | 2 | 8
thr-v1-7vj0507xz9tr2svfst47rxmdf8 | cursor | unknown | 1 | 3 | 1 | 4
? 0
```

## Sessions JSON

```console
$ urollup sessions --all --format json --timezone UTC
{
  "schema_version": 1,
  "query": {
    "command": "sessions",
    "selection": "all",
    "scope": "self",
    "timezone": "UTC"
  },
  "rows": [
    {
      "thread": "thr-v1-172efbmpfmvk15kjws2nmc0h6j",
      "session": "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa",
      "agent": "cursor",
      "requests": {
        "owned": 1,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 4,
        "output": 1,
        "total": 5
      },
      "last_date": "2024-09-19",
      "undated_requests": 0
    },
    {
      "thread": "thr-v1-24r2damwd05hvchfcymd1vhqd2",
      "session": "99999999-9999-4999-8999-999999999999",
      "agent": "cursor",
      "requests": {
        "owned": 1,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 6,
        "output": 2,
        "total": 8
      },
      "last_date": "2024-09-19",
      "undated_requests": 0
    },
    {
      "thread": "thr-v1-7vj0507xz9tr2svfst47rxmdf8",
      "session": "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb",
      "agent": "cursor",
      "requests": {
        "owned": 1,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 3,
        "output": 1,
        "total": 4
      },
      "last_date": "2024-09-19",
      "undated_requests": 0
    }
  ],
  "diagnostics": []
}
? 0
```

## Sessions (explicit source)

```console
$ urollup sessions --source . --no-default-sources --timezone UTC
urollup sessions
Selection all  Scope self  Timezone UTC

THREAD | AGENT | PROJECT | REQUESTS | INPUT | OUTPUT | TOTAL
thr-v1-172efbmpfmvk15kjws2nmc0h6j | cursor | unknown | 1 | 4 | 1 | 5
thr-v1-24r2damwd05hvchfcymd1vhqd2 | cursor | unknown | 1 | 6 | 2 | 8
thr-v1-7vj0507xz9tr2svfst47rxmdf8 | cursor | unknown | 1 | 3 | 1 | 4
? 0
```

## Sessions JSON (explicit source)

```console
$ urollup sessions --source . --no-default-sources --format json --timezone UTC
{
  "schema_version": 1,
  "query": {
    "command": "sessions",
    "selection": "all",
    "scope": "self",
    "timezone": "UTC"
  },
  "rows": [
    {
      "thread": "thr-v1-172efbmpfmvk15kjws2nmc0h6j",
      "session": "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa",
      "agent": "cursor",
      "requests": {
        "owned": 1,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 4,
        "output": 1,
        "total": 5
      },
      "last_date": "2024-09-19",
      "undated_requests": 0
    },
    {
      "thread": "thr-v1-24r2damwd05hvchfcymd1vhqd2",
      "session": "99999999-9999-4999-8999-999999999999",
      "agent": "cursor",
      "requests": {
        "owned": 1,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 6,
        "output": 2,
        "total": 8
      },
      "last_date": "2024-09-19",
      "undated_requests": 0
    },
    {
      "thread": "thr-v1-7vj0507xz9tr2svfst47rxmdf8",
      "session": "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb",
      "agent": "cursor",
      "requests": {
        "owned": 1,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 3,
        "output": 1,
        "total": 4
      },
      "last_date": "2024-09-19",
      "undated_requests": 0
    }
  ],
  "diagnostics": []
}
? 0
```

## Parent session descendants

`--session` defaults to descendant scope, so the parent and both `subagentComposerIds`
count.

```console
$ urollup report --session 99999999-9999-4999-8999-999999999999 --timezone UTC
urollup report
Selection session  Scope descendants  Timezone UTC

TOTALS
Requests  3
Owned     3
Ambiguous 0
Unknown   0
Uncached input 13
Cache read     -
Cache write    -
Output         4
Reasoning      -
Total tokens   17

COVERAGE
Status complete  Copies excluded 0  Limit observations 0
Unresolved 0  Possible 0  Requests without usage 0

REQUEST SIZES (inclusive input tokens)
Count 3  p50 4  p90 6  p99 6  max 6

ACCOUNT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
unknown | 3 | 13 | 4 | 17

EFFORT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
style-2 | 1 | 4 | 1 | 5
unknown | 2 | 9 | 3 | 12

MODEL BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
claude-4.5-opus-high-thinking | 1 | 4 | 1 | 5
cursor-grok-4.6-xhigh-fast | 1 | 6 | 2 | 8
gemini-3-pro | 1 | 3 | 1 | 4

PROJECT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
unknown | 3 | 13 | 4 | 17
? 0
```

## Parent session self

```console
$ urollup report --session 99999999-9999-4999-8999-999999999999 --scope self --timezone UTC
urollup report
Selection session  Scope self  Timezone UTC

TOTALS
Requests  1
Owned     1
Ambiguous 0
Unknown   0
Uncached input 6
Cache read     -
Cache write    -
Output         2
Reasoning      -
Total tokens   8

COVERAGE
Status complete  Copies excluded 0  Limit observations 0
Unresolved 0  Possible 0  Requests without usage 0

REQUEST SIZES (inclusive input tokens)
Count 1  p50 6  p90 6  p99 6  max 6

ACCOUNT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
unknown | 1 | 6 | 2 | 8

EFFORT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
unknown | 1 | 6 | 2 | 8

MODEL BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
cursor-grok-4.6-xhigh-fast | 1 | 6 | 2 | 8

PROJECT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
unknown | 1 | 6 | 2 | 8
? 0
```

<!-- This document follows common-doc-guidelines.md.
See github.com/jlevy/practical-prose and review guidelines before editing.
-->
