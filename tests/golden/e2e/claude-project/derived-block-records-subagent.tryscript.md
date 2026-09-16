---
sandbox: ../../../../crates/urollup-core/tests/fixtures/claude-project/derived-block-records-subagent
path:
  - $UROLLUP_BIN
env:
  CLAUDE_CONFIG_DIR: .
  CODEX_HOME: $GOLDEN_EMPTY_ROOT
  PI_CODING_AGENT_SESSION_DIR: $GOLDEN_EMPTY_ROOT/sessions
---
# E2E: claude-project/derived-block-records-subagent

The fixture case
[`claude-project/derived-block-records-subagent`](../../../../crates/urollup-core/tests/fixtures/claude-project/derived-block-records-subagent/)
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
Uncached input 4
Cache read     936,920
Cache write    13,522
Output         4,401
Reasoning      1,371
Total tokens   954,847

COVERAGE
Status complete  Copies excluded 0  Limit observations 0
Unresolved 0  Possible 0  Requests without usage 0

REQUEST SIZES (inclusive input tokens)
Count 2  p50 49,672  p90 900,774  p99 900,774  max 900,774

ACCOUNT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
unknown | 2 | 950,446 | 4,401 | 954,847

EFFORT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
xhigh | 2 | 950,446 | 4,401 | 954,847

MODEL BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
claude-opus-5 | 2 | 950,446 | 4,401 | 954,847

PROJECT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
project | 2 | 950,446 | 4,401 | 954,847
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
      "uncached_input": 4,
      "cache_read": 936920,
      "cache_write": 13522,
      "cache_write_5m": 13471,
      "cache_write_1h": 51,
      "output": 4401,
      "reasoning": 1371,
      "total": 954847
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
          "uncached_input": 4,
          "cache_read": 936920,
          "cache_write": 13522,
          "cache_write_5m": 13471,
          "cache_write_1h": 51,
          "output": 4401,
          "reasoning": 1371,
          "total": 954847
        }
      }
    ],
    "effort": [
      {
        "group": "effort",
        "value": "xhigh",
        "requests": {
          "owned": 2,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 4,
          "cache_read": 936920,
          "cache_write": 13522,
          "cache_write_5m": 13471,
          "cache_write_1h": 51,
          "output": 4401,
          "reasoning": 1371,
          "total": 954847
        }
      }
    ],
    "model": [
      {
        "group": "model",
        "value": "claude-opus-5",
        "requests": {
          "owned": 2,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 4,
          "cache_read": 936920,
          "cache_write": 13522,
          "cache_write_5m": 13471,
          "cache_write_1h": 51,
          "output": 4401,
          "reasoning": 1371,
          "total": 954847
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
          "uncached_input": 4,
          "cache_read": 936920,
          "cache_write": 13522,
          "cache_write_5m": 13471,
          "cache_write_1h": 51,
          "output": 4401,
          "reasoning": 1371,
          "total": 954847
        }
      }
    ]
  },
  "sizes": {
    "count": 2,
    "p50": 49672,
    "p90": 900774,
    "p99": 900774,
    "max": 900774
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
2026-09-01 | 2 | 4 | 936,920 | 13,522 | 4,401 | 954,847
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
        "uncached_input": 4,
        "cache_read": 936920,
        "cache_write": 13522,
        "cache_write_5m": 13471,
        "cache_write_1h": 51,
        "output": 4401,
        "reasoning": 1371,
        "total": 954847
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
thr-v1-0e4s65x8t7fxxeertq8bxabfm0 | claude | project | 1 | 900,774 | 4,218 | 904,992
thr-v1-6h6b9j6v2ytqbj5kyswsdhrkzd | claude | project | 1 | 49,672 | 183 | 49,855
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
      "thread": "thr-v1-0e4s65x8t7fxxeertq8bxabfm0",
      "agent": "claude",
      "project": "project",
      "requests": {
        "owned": 1,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 2,
        "cache_read": 900721,
        "cache_write": 51,
        "cache_write_5m": 0,
        "cache_write_1h": 51,
        "output": 4218,
        "reasoning": 1353,
        "total": 904992
      }
    },
    {
      "thread": "thr-v1-6h6b9j6v2ytqbj5kyswsdhrkzd",
      "agent": "claude",
      "project": "project",
      "requests": {
        "owned": 1,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 2,
        "cache_read": 36199,
        "cache_write": 13471,
        "cache_write_5m": 13471,
        "cache_write_1h": 0,
        "output": 183,
        "reasoning": 18,
        "total": 49855
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
