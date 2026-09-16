---
sandbox: ../../../../crates/urollup-core/tests/fixtures/claude-project/timestamp-precision
path:
  - $UROLLUP_BIN
env:
  CLAUDE_CONFIG_DIR: .
  CODEX_HOME: $GOLDEN_EMPTY_ROOT
  PI_CODING_AGENT_SESSION_DIR: $GOLDEN_EMPTY_ROOT/sessions
---
# E2E: claude-project/timestamp-precision

The fixture case
[`claude-project/timestamp-precision`](../../../../crates/urollup-core/tests/fixtures/claude-project/timestamp-precision/)
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
Requests  5
Owned     5
Ambiguous 0
Unknown   0
Uncached input 5
Cache read     10,000
Cache write    0
Output         65
Reasoning      -
Total tokens   10,070

COVERAGE
Status complete  Copies excluded 0  Limit observations 0
Unresolved 0  Possible 0  Requests without usage 0

REQUEST SIZES (inclusive input tokens)
Count 5  p50 2,001  p90 2,001  p99 2,001  max 2,001

ACCOUNT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
unknown | 5 | 10,005 | 65 | 10,070

EFFORT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
unknown | 5 | 10,005 | 65 | 10,070

MODEL BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
claude-sonnet-4-5 | 5 | 10,005 | 65 | 10,070

PROJECT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
project | 5 | 10,005 | 65 | 10,070
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
      "owned": 5,
      "ambiguous": 0,
      "unknown": 0
    },
    "tokens": {
      "uncached_input": 5,
      "cache_read": 10000,
      "cache_write": 0,
      "cache_write_5m": 0,
      "cache_write_1h": 0,
      "output": 65,
      "total": 10070
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
          "owned": 5,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 5,
          "cache_read": 10000,
          "cache_write": 0,
          "cache_write_5m": 0,
          "cache_write_1h": 0,
          "output": 65,
          "total": 10070
        }
      }
    ],
    "effort": [
      {
        "group": "effort",
        "value": "unknown",
        "requests": {
          "owned": 5,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 5,
          "cache_read": 10000,
          "cache_write": 0,
          "cache_write_5m": 0,
          "cache_write_1h": 0,
          "output": 65,
          "total": 10070
        }
      }
    ],
    "model": [
      {
        "group": "model",
        "value": "claude-sonnet-4-5",
        "requests": {
          "owned": 5,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 5,
          "cache_read": 10000,
          "cache_write": 0,
          "cache_write_5m": 0,
          "cache_write_1h": 0,
          "output": 65,
          "total": 10070
        }
      }
    ],
    "project": [
      {
        "group": "project",
        "value": "project",
        "requests": {
          "owned": 5,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 5,
          "cache_read": 10000,
          "cache_write": 0,
          "cache_write_5m": 0,
          "cache_write_1h": 0,
          "output": 65,
          "total": 10070
        }
      }
    ]
  },
  "sizes": {
    "count": 5,
    "p50": 2001,
    "p90": 2001,
    "p99": 2001,
    "max": 2001
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
2026-09-01 | 3 | 3 | 6,000 | 0 | 36 | 6,039
2026-09-02 | 2 | 2 | 4,000 | 0 | 29 | 4,031
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
        "uncached_input": 3,
        "cache_read": 6000,
        "cache_write": 0,
        "cache_write_5m": 0,
        "cache_write_1h": 0,
        "output": 36,
        "total": 6039
      }
    },
    {
      "date": "2026-09-02",
      "requests": {
        "owned": 2,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 2,
        "cache_read": 4000,
        "cache_write": 0,
        "cache_write_5m": 0,
        "cache_write_1h": 0,
        "output": 29,
        "total": 4031
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
thr-v1-2qfjyyfg473tmjm3n578jcvsbn | claude | project | 5 | 10,005 | 65 | 10,070
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
      "thread": "thr-v1-2qfjyyfg473tmjm3n578jcvsbn",
      "agent": "claude",
      "project": "project",
      "requests": {
        "owned": 5,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 5,
        "cache_read": 10000,
        "cache_write": 0,
        "cache_write_5m": 0,
        "cache_write_1h": 0,
        "output": 65,
        "total": 10070
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
