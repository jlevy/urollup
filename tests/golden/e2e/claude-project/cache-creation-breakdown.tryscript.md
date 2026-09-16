---
sandbox: ../../../../crates/urollup-core/tests/fixtures/claude-project/cache-creation-breakdown
path:
  - $UROLLUP_BIN
env:
  CLAUDE_CONFIG_DIR: .
  CODEX_HOME: $GOLDEN_EMPTY_ROOT
  PI_CODING_AGENT_SESSION_DIR: $GOLDEN_EMPTY_ROOT/sessions
---
# E2E: claude-project/cache-creation-breakdown

The fixture case
[`claude-project/cache-creation-breakdown`](../../../../crates/urollup-core/tests/fixtures/claude-project/cache-creation-breakdown/)
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
Uncached input 6
Cache read     14,000
Cache write    9,700
Output         115
Reasoning      -
Total tokens   23,821

COVERAGE
Status complete  Copies excluded 0  Limit observations 0
Unresolved 0  Possible 0  Requests without usage 0

REQUEST SIZES (inclusive input tokens)
Count 3  p50 9,001  p90 9,703  p99 9,703  max 9,703

ACCOUNT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
unknown | 3 | 23,706 | 115 | 23,821

EFFORT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
unknown | 3 | 23,706 | 115 | 23,821

MODEL BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
claude-sonnet-4-5 | 3 | 23,706 | 115 | 23,821

PROJECT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
project | 3 | 23,706 | 115 | 23,821

DIAGNOSTICS
claude-cache-creation-breakdown-mismatch x1: Claude cache creation total 5000 differs from its lifetime breakdown 5500
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
  "diagnostics": [
    {
      "code": "claude-cache-creation-breakdown-mismatch",
      "count": 1,
      "detail": "Claude cache creation total 5000 differs from its lifetime breakdown 5500"
    }
  ],
  "totals": {
    "requests": {
      "owned": 3,
      "ambiguous": 0,
      "unknown": 0
    },
    "tokens": {
      "uncached_input": 6,
      "cache_read": 14000,
      "cache_write": 9700,
      "cache_write_5m": 0,
      "cache_write_1h": 4000,
      "cache_write_unspecified": 5700,
      "output": 115,
      "total": 23821
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
          "uncached_input": 6,
          "cache_read": 14000,
          "cache_write": 9700,
          "cache_write_5m": 0,
          "cache_write_1h": 4000,
          "cache_write_unspecified": 5700,
          "output": 115,
          "total": 23821
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
          "uncached_input": 6,
          "cache_read": 14000,
          "cache_write": 9700,
          "cache_write_5m": 0,
          "cache_write_1h": 4000,
          "cache_write_unspecified": 5700,
          "output": 115,
          "total": 23821
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
          "uncached_input": 6,
          "cache_read": 14000,
          "cache_write": 9700,
          "cache_write_5m": 0,
          "cache_write_1h": 4000,
          "cache_write_unspecified": 5700,
          "output": 115,
          "total": 23821
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
          "uncached_input": 6,
          "cache_read": 14000,
          "cache_write": 9700,
          "cache_write_5m": 0,
          "cache_write_1h": 4000,
          "cache_write_unspecified": 5700,
          "output": 115,
          "total": 23821
        }
      }
    ]
  },
  "sizes": {
    "count": 3,
    "p50": 9001,
    "p90": 9703,
    "p99": 9703,
    "max": 9703
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
2026-09-01 | 3 | 6 | 14,000 | 9,700 | 115 | 23,821

DIAGNOSTICS
claude-cache-creation-breakdown-mismatch x1: Claude cache creation total 5000 differs from its lifetime breakdown 5500
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
        "uncached_input": 6,
        "cache_read": 14000,
        "cache_write": 9700,
        "cache_write_5m": 0,
        "cache_write_1h": 4000,
        "cache_write_unspecified": 5700,
        "output": 115,
        "total": 23821
      }
    }
  ],
  "diagnostics": [
    {
      "code": "claude-cache-creation-breakdown-mismatch",
      "count": 1,
      "detail": "Claude cache creation total 5000 differs from its lifetime breakdown 5500"
    }
  ]
}
? 0
```

## Sessions

```console
$ urollup sessions --all --timezone UTC
urollup sessions
Selection all  Scope self  Timezone UTC

THREAD | AGENT | PROJECT | REQUESTS | INPUT | OUTPUT | TOTAL
thr-v1-5a0r839nnebytqctcqj8tp2f72 | claude | project | 3 | 23,706 | 115 | 23,821

DIAGNOSTICS
claude-cache-creation-breakdown-mismatch x1: Claude cache creation total 5000 differs from its lifetime breakdown 5500
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
      "thread": "thr-v1-5a0r839nnebytqctcqj8tp2f72",
      "agent": "claude",
      "project": "project",
      "requests": {
        "owned": 3,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 6,
        "cache_read": 14000,
        "cache_write": 9700,
        "cache_write_5m": 0,
        "cache_write_1h": 4000,
        "cache_write_unspecified": 5700,
        "output": 115,
        "total": 23821
      }
    }
  ],
  "diagnostics": [
    {
      "code": "claude-cache-creation-breakdown-mismatch",
      "count": 1,
      "detail": "Claude cache creation total 5000 differs from its lifetime breakdown 5500"
    }
  ]
}
? 0
```

## Sessions (explicit source)

```console
$ urollup sessions --source . --no-default-sources --timezone UTC
urollup sessions
Selection all  Scope self  Timezone UTC

THREAD | AGENT | PROJECT | REQUESTS | INPUT | OUTPUT | TOTAL
thr-v1-5a0r839nnebytqctcqj8tp2f72 | claude | project | 3 | 23,706 | 115 | 23,821

DIAGNOSTICS
claude-cache-creation-breakdown-mismatch x1: Claude cache creation total 5000 differs from its lifetime breakdown 5500
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
      "thread": "thr-v1-5a0r839nnebytqctcqj8tp2f72",
      "agent": "claude",
      "project": "project",
      "requests": {
        "owned": 3,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 6,
        "cache_read": 14000,
        "cache_write": 9700,
        "cache_write_5m": 0,
        "cache_write_1h": 4000,
        "cache_write_unspecified": 5700,
        "output": 115,
        "total": 23821
      }
    }
  ],
  "diagnostics": [
    {
      "code": "claude-cache-creation-breakdown-mismatch",
      "count": 1,
      "detail": "Claude cache creation total 5000 differs from its lifetime breakdown 5500"
    }
  ]
}
? 0
```

<!-- This document follows common-doc-guidelines.md.
See github.com/jlevy/practical-prose and review guidelines before editing.
-->
