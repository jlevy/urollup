---
sandbox: ../../../../crates/urollup-core/tests/fixtures/claude-project/btw-side-question
path:
  - $UROLLUP_BIN
env:
  CLAUDE_CONFIG_DIR: .
  CODEX_HOME: $GOLDEN_EMPTY_ROOT
  PI_CODING_AGENT_SESSION_DIR: $GOLDEN_EMPTY_ROOT/sessions
---
# E2E: claude-project/btw-side-question

The fixture case
[`claude-project/btw-side-question`](../../../../crates/urollup-core/tests/fixtures/claude-project/btw-side-question/)
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
Requests  3
Owned     3
Ambiguous 0
Unknown   0
Uncached input 10
Cache read     37,100
Cache write    550
Output         62
Reasoning      -
Total tokens   37,722

COVERAGE
Status complete  Copies excluded 1  Limit observations 0
Unresolved 0  Possible 0  Requests without usage 0

REQUEST SIZES (inclusive input tokens)
Count 3  p50 18,403  p90 18,552  p99 18,552  max 18,552

ACCOUNT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
unknown | 3 | 37,660 | 62 | 37,722

EFFORT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
unknown | 3 | 37,660 | 62 | 37,722

MODEL BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
claude-sonnet-4-5 | 3 | 37,660 | 62 | 37,722

PROJECT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
project | 3 | 37,660 | 62 | 37,722
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
    "copies_excluded": 1,
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
      "cache_read": 37100,
      "cache_write": 550,
      "cache_write_5m": 550,
      "cache_write_1h": 0,
      "output": 62,
      "total": 37722
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
          "cache_read": 37100,
          "cache_write": 550,
          "cache_write_5m": 550,
          "cache_write_1h": 0,
          "output": 62,
          "total": 37722
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
          "cache_read": 37100,
          "cache_write": 550,
          "cache_write_5m": 550,
          "cache_write_1h": 0,
          "output": 62,
          "total": 37722
        }
      }
    ],
    "model": [
      {
        "group": "model",
        "value": "claude-sonnet-4-5",
        "requests": {
          "owned": 3,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 10,
          "cache_read": 37100,
          "cache_write": 550,
          "cache_write_5m": 550,
          "cache_write_1h": 0,
          "output": 62,
          "total": 37722
        }
      }
    ],
    "project": [
      {
        "group": "project",
        "value": "project",
        "requests": {
          "owned": 3,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 10,
          "cache_read": 37100,
          "cache_write": 550,
          "cache_write_5m": 550,
          "cache_write_1h": 0,
          "output": 62,
          "total": 37722
        }
      }
    ]
  },
  "sizes": {
    "count": 3,
    "p50": 18403,
    "p90": 18552,
    "p99": 18552,
    "max": 18552
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
2026-09-01 | 3 | 10 | 37,100 | 550 | 62 | 37,722
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
        "owned": 3,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 10,
        "cache_read": 37100,
        "cache_write": 550,
        "cache_write_5m": 550,
        "cache_write_1h": 0,
        "output": 62,
        "total": 37722
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
thr-v1-01xc4q47fw48wvrq96dg7ste2f | claude | project | 1 | 705 | 30 | 735
thr-v1-50ahyfdthb1cyyaa81sg3q9st6 | claude | project | 2 | 36,955 | 32 | 36,987
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
      "thread": "thr-v1-01xc4q47fw48wvrq96dg7ste2f",
      "agent": "claude",
      "project": "project",
      "requests": {
        "owned": 1,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 5,
        "cache_read": 700,
        "cache_write": 0,
        "cache_write_5m": 0,
        "cache_write_1h": 0,
        "output": 30,
        "total": 735
      }
    },
    {
      "thread": "thr-v1-50ahyfdthb1cyyaa81sg3q9st6",
      "agent": "claude",
      "project": "project",
      "requests": {
        "owned": 2,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 5,
        "cache_read": 36400,
        "cache_write": 550,
        "cache_write_5m": 550,
        "cache_write_1h": 0,
        "output": 32,
        "total": 36987
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
