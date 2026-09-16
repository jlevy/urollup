---
sandbox: ../../../../crates/urollup-core/tests/fixtures/claude-project/block-record-selection
path:
  - $UROLLUP_BIN
env:
  CLAUDE_CONFIG_DIR: .
  CODEX_HOME: $GOLDEN_EMPTY_ROOT
  PI_CODING_AGENT_SESSION_DIR: $GOLDEN_EMPTY_ROOT/sessions
---
# E2E: claude-project/block-record-selection

The fixture case
[`claude-project/block-record-selection`](../../../../crates/urollup-core/tests/fixtures/claude-project/block-record-selection/)
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
Uncached input 7
Cache read     93,300
Cache write    1,210
Output         470
Reasoning      -
Total tokens   94,987

COVERAGE
Status complete  Copies excluded 0  Limit observations 0
Unresolved 0  Possible 0  Requests without usage 0

REQUEST SIZES (inclusive input tokens)
Count 3  p50 31,212  p90 32,101  p99 32,101  max 32,101

ACCOUNT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
unknown | 3 | 94,517 | 470 | 94,987

EFFORT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
unknown | 3 | 94,517 | 470 | 94,987

MODEL BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
claude-sonnet-4-5 | 3 | 94,517 | 470 | 94,987

PROJECT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
project | 3 | 94,517 | 470 | 94,987

DIAGNOSTICS
claude-block-usage-conflict x2: claude-largest-output: input or cache fields differ across Claude block records
claude-block-usage-conflict x2: claude-largest-output: input or cache fields differ across Claude block records
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
      "code": "claude-block-usage-conflict",
      "count": 2,
      "detail": "claude-largest-output: input or cache fields differ across Claude block records"
    },
    {
      "code": "claude-block-usage-conflict",
      "count": 2,
      "detail": "claude-largest-output: input or cache fields differ across Claude block records"
    }
  ],
  "totals": {
    "requests": {
      "owned": 3,
      "ambiguous": 0,
      "unknown": 0
    },
    "tokens": {
      "uncached_input": 7,
      "cache_read": 93300,
      "cache_write": 1210,
      "cache_write_5m": 1210,
      "cache_write_1h": 0,
      "output": 470,
      "total": 94987
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
          "uncached_input": 7,
          "cache_read": 93300,
          "cache_write": 1210,
          "cache_write_5m": 1210,
          "cache_write_1h": 0,
          "output": 470,
          "total": 94987
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
          "uncached_input": 7,
          "cache_read": 93300,
          "cache_write": 1210,
          "cache_write_5m": 1210,
          "cache_write_1h": 0,
          "output": 470,
          "total": 94987
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
          "uncached_input": 7,
          "cache_read": 93300,
          "cache_write": 1210,
          "cache_write_5m": 1210,
          "cache_write_1h": 0,
          "output": 470,
          "total": 94987
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
          "uncached_input": 7,
          "cache_read": 93300,
          "cache_write": 1210,
          "cache_write_5m": 1210,
          "cache_write_1h": 0,
          "output": 470,
          "total": 94987
        }
      }
    ]
  },
  "sizes": {
    "count": 3,
    "p50": 31212,
    "p90": 32101,
    "p99": 32101,
    "max": 32101
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
2026-09-01 | 3 | 7 | 93,300 | 1,210 | 470 | 94,987

DIAGNOSTICS
claude-block-usage-conflict x2: claude-largest-output: input or cache fields differ across Claude block records
claude-block-usage-conflict x2: claude-largest-output: input or cache fields differ across Claude block records
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
        "uncached_input": 7,
        "cache_read": 93300,
        "cache_write": 1210,
        "cache_write_5m": 1210,
        "cache_write_1h": 0,
        "output": 470,
        "total": 94987
      }
    }
  ],
  "diagnostics": [
    {
      "code": "claude-block-usage-conflict",
      "count": 2,
      "detail": "claude-largest-output: input or cache fields differ across Claude block records"
    },
    {
      "code": "claude-block-usage-conflict",
      "count": 2,
      "detail": "claude-largest-output: input or cache fields differ across Claude block records"
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
thr-v1-0576b28q452kd5px45f9pmm8f0 | claude | project | 3 | 94,517 | 470 | 94,987

DIAGNOSTICS
claude-block-usage-conflict x2: claude-largest-output: input or cache fields differ across Claude block records
claude-block-usage-conflict x2: claude-largest-output: input or cache fields differ across Claude block records
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
      "thread": "thr-v1-0576b28q452kd5px45f9pmm8f0",
      "agent": "claude",
      "project": "project",
      "requests": {
        "owned": 3,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 7,
        "cache_read": 93300,
        "cache_write": 1210,
        "cache_write_5m": 1210,
        "cache_write_1h": 0,
        "output": 470,
        "total": 94987
      }
    }
  ],
  "diagnostics": [
    {
      "code": "claude-block-usage-conflict",
      "count": 2,
      "detail": "claude-largest-output: input or cache fields differ across Claude block records"
    },
    {
      "code": "claude-block-usage-conflict",
      "count": 2,
      "detail": "claude-largest-output: input or cache fields differ across Claude block records"
    }
  ]
}
? 0
```

<!-- This document follows common-doc-guidelines.md.
See github.com/jlevy/practical-prose and review guidelines before editing.
-->
