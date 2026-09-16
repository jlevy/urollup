---
sandbox: ../../../../crates/urollup-core/tests/fixtures/claude-project/missing-request-id
path:
  - $UROLLUP_BIN
env:
  CLAUDE_CONFIG_DIR: .
  CODEX_HOME: $GOLDEN_EMPTY_ROOT
  PI_CODING_AGENT_SESSION_DIR: $GOLDEN_EMPTY_ROOT/sessions
---
# E2E: claude-project/missing-request-id

The fixture case
[`claude-project/missing-request-id`](../../../../crates/urollup-core/tests/fixtures/claude-project/missing-request-id/)
read through `CLAUDE_CONFIG_DIR` from a sandbox copy, with every other discovery root
empty and HOME hermetic.
`make e2e-results` checks its reconciled results against `expected.json`; this session
records the complete output of each view for review.

## Report

```console
$ urollup report --all --timezone UTC
urollup report
Selection all  Scope self  Timezone UTC

TOTALS
Requests  2
Owned     2
Ambiguous 0
Unknown   0
Uncached input 8
Cache read     1,800
Cache write    60
Output         117
Reasoning      -
Total tokens   1,985

COVERAGE
Status complete  Copies excluded 0  Limit observations 0
Unresolved 0  Possible 0  Requests without usage 0

REQUEST SIZES (inclusive input tokens)
Count 2  p50 907  p90 961  p99 961  max 961

ACCOUNT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
unknown | 2 | 1,868 | 117 | 1,985

EFFORT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
unknown | 2 | 1,868 | 117 | 1,985

MODEL BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
claude-sonnet-4-5 | 2 | 1,868 | 117 | 1,985

PROJECT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
project | 2 | 1,868 | 117 | 1,985
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
      "owned": 2,
      "ambiguous": 0,
      "unknown": 0
    },
    "tokens": {
      "uncached_input": 8,
      "cache_read": 1800,
      "cache_write": 60,
      "cache_write_5m": 60,
      "cache_write_1h": 0,
      "output": 117,
      "total": 1985
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
          "owned": 2,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 8,
          "cache_read": 1800,
          "cache_write": 60,
          "cache_write_5m": 60,
          "cache_write_1h": 0,
          "output": 117,
          "total": 1985
        }
      }
    ],
    "effort": [
      {
        "group": "effort",
        "value": "unknown",
        "requests": {
          "owned": 2,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 8,
          "cache_read": 1800,
          "cache_write": 60,
          "cache_write_5m": 60,
          "cache_write_1h": 0,
          "output": 117,
          "total": 1985
        }
      }
    ],
    "model": [
      {
        "group": "model",
        "value": "claude-sonnet-4-5",
        "requests": {
          "owned": 2,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 8,
          "cache_read": 1800,
          "cache_write": 60,
          "cache_write_5m": 60,
          "cache_write_1h": 0,
          "output": 117,
          "total": 1985
        }
      }
    ],
    "project": [
      {
        "group": "project",
        "value": "project",
        "requests": {
          "owned": 2,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 8,
          "cache_read": 1800,
          "cache_write": 60,
          "cache_write_5m": 60,
          "cache_write_1h": 0,
          "output": 117,
          "total": 1985
        }
      }
    ]
  },
  "sizes": {
    "count": 2,
    "p50": 907,
    "p90": 961,
    "p99": 961,
    "max": 961
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
2026-09-01 | 2 | 8 | 1,800 | 60 | 117 | 1,985
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
      "date": "2026-09-01",
      "requests": {
        "owned": 2,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 8,
        "cache_read": 1800,
        "cache_write": 60,
        "cache_write_5m": 60,
        "cache_write_1h": 0,
        "output": 117,
        "total": 1985
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
thr-v1-0m1rwwga5sn1qw4nbwfg48n7g5 | claude | project | 2 | 1,868 | 117 | 1,985
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
      "thread": "thr-v1-0m1rwwga5sn1qw4nbwfg48n7g5",
      "agent": "claude",
      "project": "project",
      "requests": {
        "owned": 2,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 8,
        "cache_read": 1800,
        "cache_write": 60,
        "cache_write_5m": 60,
        "cache_write_1h": 0,
        "output": 117,
        "total": 1985
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
thr-v1-0m1rwwga5sn1qw4nbwfg48n7g5 | claude | project | 2 | 1,868 | 117 | 1,985
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
      "thread": "thr-v1-0m1rwwga5sn1qw4nbwfg48n7g5",
      "agent": "claude",
      "project": "project",
      "requests": {
        "owned": 2,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 8,
        "cache_read": 1800,
        "cache_write": 60,
        "cache_write_5m": 60,
        "cache_write_1h": 0,
        "output": 117,
        "total": 1985
      }
    }
  ],
  "diagnostics": []
}
? 0
```

<!-- This document follows common-doc-guidelines.md.
See github.com/jlevy/practical-prose and review guidelines before editing.
-->
