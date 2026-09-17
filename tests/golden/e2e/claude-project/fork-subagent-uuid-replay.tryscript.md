---
sandbox: ../../../../crates/urollup-core/tests/fixtures/claude-project/fork-subagent-uuid-replay
path:
  - $UROLLUP_BIN
env:
  CLAUDE_CONFIG_DIR: .
  CODEX_HOME: $GOLDEN_EMPTY_ROOT
  PI_CODING_AGENT_SESSION_DIR: $GOLDEN_EMPTY_ROOT/sessions
---
# E2E: claude-project/fork-subagent-uuid-replay

The fixture case
[`claude-project/fork-subagent-uuid-replay`](../../../../crates/urollup-core/tests/fixtures/claude-project/fork-subagent-uuid-replay/)
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
Uncached input 14
Cache read     6,150
Cache write    4,050
Output         255
Reasoning      -
Total tokens   10,469

COVERAGE
Status complete  Copies excluded 2  Limit observations 0
Unresolved 0  Possible 0  Requests without usage 0

REQUEST SIZES (inclusive input tokens)
Count 3  p50 3,152  p90 4,054  p99 4,054  max 4,054

ACCOUNT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
unknown | 3 | 10,214 | 255 | 10,469

EFFORT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
unknown | 3 | 10,214 | 255 | 10,469

MODEL BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
claude-sonnet-4-5 | 3 | 10,214 | 255 | 10,469

PROJECT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
project | 3 | 10,214 | 255 | 10,469
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
    "copies_excluded": 2,
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
      "uncached_input": 14,
      "cache_read": 6150,
      "cache_write": 4050,
      "cache_write_5m": 4050,
      "cache_write_1h": 0,
      "output": 255,
      "total": 10469
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
          "uncached_input": 14,
          "cache_read": 6150,
          "cache_write": 4050,
          "cache_write_5m": 4050,
          "cache_write_1h": 0,
          "output": 255,
          "total": 10469
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
          "uncached_input": 14,
          "cache_read": 6150,
          "cache_write": 4050,
          "cache_write_5m": 4050,
          "cache_write_1h": 0,
          "output": 255,
          "total": 10469
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
          "uncached_input": 14,
          "cache_read": 6150,
          "cache_write": 4050,
          "cache_write_5m": 4050,
          "cache_write_1h": 0,
          "output": 255,
          "total": 10469
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
          "uncached_input": 14,
          "cache_read": 6150,
          "cache_write": 4050,
          "cache_write_5m": 4050,
          "cache_write_1h": 0,
          "output": 255,
          "total": 10469
        }
      }
    ]
  },
  "sizes": {
    "count": 3,
    "p50": 3152,
    "p90": 4054,
    "p99": 4054,
    "max": 4054
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
2026-09-01 | 3 | 14 | 6,150 | 4,050 | 255 | 10,469
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
        "uncached_input": 14,
        "cache_read": 6150,
        "cache_write": 4050,
        "cache_write_5m": 4050,
        "cache_write_1h": 0,
        "output": 255,
        "total": 10469
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
thr-v1-712f3q4xpathegey6wnjavhsyr | claude | project | 2 | 6,160 | 135 | 6,295
thr-v1-7e473sr00xpgwdb4bq3swcwjn3 | claude | project | 1 | 4,054 | 120 | 4,174
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
      "thread": "thr-v1-712f3q4xpathegey6wnjavhsyr",
      "session": "00000000-0000-4000-8000-000500000001",
      "agent": "claude",
      "project": "project",
      "requests": {
        "owned": 2,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 10,
        "cache_read": 3000,
        "cache_write": 3150,
        "cache_write_5m": 3150,
        "cache_write_1h": 0,
        "output": 135,
        "total": 6295
      }
    },
    {
      "thread": "thr-v1-7e473sr00xpgwdb4bq3swcwjn3",
      "session": "00000000-0000-4000-8000-000500000001/a0000000000050001",
      "agent": "claude",
      "project": "project",
      "requests": {
        "owned": 1,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 4,
        "cache_read": 3150,
        "cache_write": 900,
        "cache_write_5m": 900,
        "cache_write_1h": 0,
        "output": 120,
        "total": 4174
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
thr-v1-712f3q4xpathegey6wnjavhsyr | claude | project | 2 | 6,160 | 135 | 6,295
thr-v1-7e473sr00xpgwdb4bq3swcwjn3 | claude | project | 1 | 4,054 | 120 | 4,174
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
      "thread": "thr-v1-712f3q4xpathegey6wnjavhsyr",
      "session": "00000000-0000-4000-8000-000500000001",
      "agent": "claude",
      "project": "project",
      "requests": {
        "owned": 2,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 10,
        "cache_read": 3000,
        "cache_write": 3150,
        "cache_write_5m": 3150,
        "cache_write_1h": 0,
        "output": 135,
        "total": 6295
      }
    },
    {
      "thread": "thr-v1-7e473sr00xpgwdb4bq3swcwjn3",
      "session": "00000000-0000-4000-8000-000500000001/a0000000000050001",
      "agent": "claude",
      "project": "project",
      "requests": {
        "owned": 1,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 4,
        "cache_read": 3150,
        "cache_write": 900,
        "cache_write_5m": 900,
        "cache_write_1h": 0,
        "output": 120,
        "total": 4174
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
