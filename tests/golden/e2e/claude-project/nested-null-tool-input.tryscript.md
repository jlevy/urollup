---
sandbox: ../../../../crates/urollup-core/tests/fixtures/claude-project/nested-null-tool-input
path:
  - $UROLLUP_BIN
env:
  CLAUDE_CONFIG_DIR: .
  CODEX_HOME: $GOLDEN_EMPTY_ROOT
  PI_CODING_AGENT_SESSION_DIR: $GOLDEN_EMPTY_ROOT/sessions
---
# E2E: claude-project/nested-null-tool-input

The fixture case
[`claude-project/nested-null-tool-input`](../../../../crates/urollup-core/tests/fixtures/claude-project/nested-null-tool-input/)
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
Cache read     10,100
Cache write    10
Output         220
Reasoning      -
Total tokens   10,338

COVERAGE
Status complete  Copies excluded 0  Limit observations 0
Unresolved 0  Possible 0  Requests without usage 0

REQUEST SIZES (inclusive input tokens)
Count 2  p50 5,006  p90 5,112  p99 5,112  max 5,112

ACCOUNT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
unknown | 2 | 10,118 | 220 | 10,338

EFFORT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
unknown | 2 | 10,118 | 220 | 10,338

MODEL BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
claude-sonnet-4-5 | 2 | 10,118 | 220 | 10,338

PROJECT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
project | 2 | 10,118 | 220 | 10,338
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
      "cache_read": 10100,
      "cache_write": 10,
      "cache_write_5m": 10,
      "cache_write_1h": 0,
      "output": 220,
      "total": 10338
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
          "cache_read": 10100,
          "cache_write": 10,
          "cache_write_5m": 10,
          "cache_write_1h": 0,
          "output": 220,
          "total": 10338
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
          "cache_read": 10100,
          "cache_write": 10,
          "cache_write_5m": 10,
          "cache_write_1h": 0,
          "output": 220,
          "total": 10338
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
          "cache_read": 10100,
          "cache_write": 10,
          "cache_write_5m": 10,
          "cache_write_1h": 0,
          "output": 220,
          "total": 10338
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
          "cache_read": 10100,
          "cache_write": 10,
          "cache_write_5m": 10,
          "cache_write_1h": 0,
          "output": 220,
          "total": 10338
        }
      }
    ]
  },
  "sizes": {
    "count": 2,
    "p50": 5006,
    "p90": 5112,
    "p99": 5112,
    "max": 5112
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
2026-09-01 | 2 | 8 | 10,100 | 10 | 220 | 10,338
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
        "cache_read": 10100,
        "cache_write": 10,
        "cache_write_5m": 10,
        "cache_write_1h": 0,
        "output": 220,
        "total": 10338
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
thr-v1-6maedv4wfedwka5meh5db3a4qg | claude | project | 2 | 10,118 | 220 | 10,338
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
      "thread": "thr-v1-6maedv4wfedwka5meh5db3a4qg",
      "agent": "claude",
      "project": "project",
      "requests": {
        "owned": 2,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 8,
        "cache_read": 10100,
        "cache_write": 10,
        "cache_write_5m": 10,
        "cache_write_1h": 0,
        "output": 220,
        "total": 10338
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
thr-v1-6maedv4wfedwka5meh5db3a4qg | claude | project | 2 | 10,118 | 220 | 10,338
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
      "thread": "thr-v1-6maedv4wfedwka5meh5db3a4qg",
      "agent": "claude",
      "project": "project",
      "requests": {
        "owned": 2,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 8,
        "cache_read": 10100,
        "cache_write": 10,
        "cache_write_5m": 10,
        "cache_write_1h": 0,
        "output": 220,
        "total": 10338
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
