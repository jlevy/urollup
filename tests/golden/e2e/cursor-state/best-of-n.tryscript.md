---
sandbox: ../../../../crates/urollup-core/tests/fixtures/cursor-state/best-of-n
path:
  - $UROLLUP_BIN
env:
  CLAUDE_CONFIG_DIR: $GOLDEN_EMPTY_ROOT
  CODEX_HOME: $GOLDEN_EMPTY_ROOT
  PI_CODING_AGENT_SESSION_DIR: $GOLDEN_EMPTY_ROOT/sessions
  UROLLUP_CURSOR_DIRS: .
---
# E2E: cursor-state/best-of-n

The fixture case
[`cursor-state/best-of-n`](../../../../crates/urollup-core/tests/fixtures/cursor-state/best-of-n/)
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
Uncached input 10
Cache read     -
Cache write    -
Output         3
Reasoning      -
Total tokens   13

COVERAGE
Status complete  Copies excluded 0  Limit observations 0
Unresolved 0  Possible 0  Requests without usage 0

REQUEST SIZES (inclusive input tokens)
Count 3  p50 3  p90 5  p99 5  max 5

ACCOUNT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
unknown | 3 | 10 | 3 | 13

EFFORT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
unknown | 3 | 10 | 3 | 13

MODEL BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
claude-4.5-opus-high-thinking | 1 | 3 | 1 | 4
cursor-grok-4.6-xhigh-fast | 1 | 5 | 1 | 6
gemini-3-pro | 1 | 2 | 1 | 3

PROJECT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
unknown | 3 | 10 | 3 | 13
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
      "uncached_input": 10,
      "output": 3,
      "total": 13
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
          "uncached_input": 10,
          "output": 3,
          "total": 13
        }
      }
    ],
    "effort": [
      {
        "group": "effort",
        "value": "unknown",
        "requests": {
          "owned": 3,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 10,
          "output": 3,
          "total": 13
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
          "uncached_input": 3,
          "output": 1,
          "total": 4
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
          "uncached_input": 5,
          "output": 1,
          "total": 6
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
          "uncached_input": 2,
          "output": 1,
          "total": 3
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
          "uncached_input": 10,
          "output": 3,
          "total": 13
        }
      }
    ]
  },
  "sizes": {
    "count": 3,
    "p50": 3,
    "p90": 5,
    "p99": 5,
    "max": 5
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
2024-09-19 | 3 | 10 | - | - | 3 | 13
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
        "uncached_input": 10,
        "output": 3,
        "total": 13
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
thr-v1-447jdx7db7b6ne30w60pgfsffx | cursor | unknown | 1 | 5 | 1 | 6
thr-v1-4dv3jb4gwecb0mjs30d1dx5y4v | cursor | unknown | 1 | 3 | 1 | 4
thr-v1-4phaax104ctjz8pkr3c41zss7s | cursor | unknown | 1 | 2 | 1 | 3
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
      "thread": "thr-v1-447jdx7db7b6ne30w60pgfsffx",
      "session": "55555555-5555-4555-8555-555555555555",
      "agent": "cursor",
      "requests": {
        "owned": 1,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 5,
        "output": 1,
        "total": 6
      }
    },
    {
      "thread": "thr-v1-4dv3jb4gwecb0mjs30d1dx5y4v",
      "session": "cccccccc-cccc-4ccc-8ccc-cccccccccccc",
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
      }
    },
    {
      "thread": "thr-v1-4phaax104ctjz8pkr3c41zss7s",
      "session": "dddddddd-dddd-4ddd-8ddd-dddddddddddd",
      "agent": "cursor",
      "requests": {
        "owned": 1,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 2,
        "output": 1,
        "total": 3
      }
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
thr-v1-447jdx7db7b6ne30w60pgfsffx | cursor | unknown | 1 | 5 | 1 | 6
thr-v1-4dv3jb4gwecb0mjs30d1dx5y4v | cursor | unknown | 1 | 3 | 1 | 4
thr-v1-4phaax104ctjz8pkr3c41zss7s | cursor | unknown | 1 | 2 | 1 | 3
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
      "thread": "thr-v1-447jdx7db7b6ne30w60pgfsffx",
      "session": "55555555-5555-4555-8555-555555555555",
      "agent": "cursor",
      "requests": {
        "owned": 1,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 5,
        "output": 1,
        "total": 6
      }
    },
    {
      "thread": "thr-v1-4dv3jb4gwecb0mjs30d1dx5y4v",
      "session": "cccccccc-cccc-4ccc-8ccc-cccccccccccc",
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
      }
    },
    {
      "thread": "thr-v1-4phaax104ctjz8pkr3c41zss7s",
      "session": "dddddddd-dddd-4ddd-8ddd-dddddddddddd",
      "agent": "cursor",
      "requests": {
        "owned": 1,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 2,
        "output": 1,
        "total": 3
      }
    }
  ],
  "diagnostics": []
}
? 0
```

## Parent session descendants

`--session` defaults to descendant scope.
Best-of-N siblings stay on Spawn edges, so their independently recorded tokens still
count.

```console
$ urollup report --session 55555555-5555-4555-8555-555555555555 --timezone UTC --group-by provider,model
urollup report
Selection session  Scope descendants  Timezone UTC

TOTALS
Requests  3
Owned     3
Ambiguous 0
Unknown   0
Uncached input 10
Cache read     -
Cache write    -
Output         3
Reasoning      -
Total tokens   13

COVERAGE
Status complete  Copies excluded 0  Limit observations 0
Unresolved 0  Possible 0  Requests without usage 0

REQUEST SIZES (inclusive input tokens)
Count 3  p50 3  p90 5  p99 5  max 5

MODEL BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
claude-4.5-opus-high-thinking | 1 | 3 | 1 | 4
cursor-grok-4.6-xhigh-fast | 1 | 5 | 1 | 6
gemini-3-pro | 1 | 2 | 1 | 3

PROVIDER BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
anthropic | 1 | 3 | 1 | 4
cursor | 1 | 5 | 1 | 6
google | 1 | 2 | 1 | 3
? 0
```

## Parent session self

```console
$ urollup report --session 55555555-5555-4555-8555-555555555555 --scope self --timezone UTC --group-by provider,model
urollup report
Selection session  Scope self  Timezone UTC

TOTALS
Requests  1
Owned     1
Ambiguous 0
Unknown   0
Uncached input 5
Cache read     -
Cache write    -
Output         1
Reasoning      -
Total tokens   6

COVERAGE
Status complete  Copies excluded 0  Limit observations 0
Unresolved 0  Possible 0  Requests without usage 0

REQUEST SIZES (inclusive input tokens)
Count 1  p50 5  p90 5  p99 5  max 5

MODEL BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
cursor-grok-4.6-xhigh-fast | 1 | 5 | 1 | 6

PROVIDER BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
cursor | 1 | 5 | 1 | 6
? 0
```

<!-- This document follows common-doc-guidelines.md.
See github.com/jlevy/practical-prose and review guidelines before editing.
-->
