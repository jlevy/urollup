---
sandbox: ../../../../crates/urollup-core/tests/fixtures/claude-project/inline-sidechains
path:
  - $UROLLUP_BIN
env:
  CLAUDE_CONFIG_DIR: .
  CODEX_HOME: $GOLDEN_EMPTY_ROOT
  PI_CODING_AGENT_SESSION_DIR: $GOLDEN_EMPTY_ROOT/sessions
---
# E2E: claude-project/inline-sidechains

The fixture case
[`claude-project/inline-sidechains`](../../../../crates/urollup-core/tests/fixtures/claude-project/inline-sidechains/)
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
Requests  6
Owned     6
Ambiguous 0
Unknown   0
Uncached input 13
Cache read     1,350
Cache write    850
Output         140
Reasoning      -
Total tokens   2,353

COVERAGE
Status complete  Copies excluded 0  Limit observations 0
Unresolved 0  Possible 0  Requests without usage 0

REQUEST SIZES (inclusive input tokens)
Count 6  p50 303  p90 551  p99 551  max 551

ACCOUNT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
unknown | 6 | 2,213 | 140 | 2,353

EFFORT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
unknown | 6 | 2,213 | 140 | 2,353

MODEL BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
claude-haiku-4-5 | 3 | 1,108 | 85 | 1,193
claude-sonnet-4-5 | 3 | 1,105 | 55 | 1,160

PROJECT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
project | 6 | 2,213 | 140 | 2,353
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
      "owned": 6,
      "ambiguous": 0,
      "unknown": 0
    },
    "tokens": {
      "uncached_input": 13,
      "cache_read": 1350,
      "cache_write": 850,
      "cache_write_5m": 850,
      "cache_write_1h": 0,
      "output": 140,
      "total": 2353
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
          "owned": 6,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 13,
          "cache_read": 1350,
          "cache_write": 850,
          "cache_write_5m": 850,
          "cache_write_1h": 0,
          "output": 140,
          "total": 2353
        }
      }
    ],
    "effort": [
      {
        "group": "effort",
        "value": "unknown",
        "requests": {
          "owned": 6,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 13,
          "cache_read": 1350,
          "cache_write": 850,
          "cache_write_5m": 850,
          "cache_write_1h": 0,
          "output": 140,
          "total": 2353
        }
      }
    ],
    "model": [
      {
        "group": "model",
        "value": "claude-haiku-4-5",
        "requests": {
          "owned": 3,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 8,
          "cache_read": 400,
          "cache_write": 700,
          "cache_write_5m": 700,
          "cache_write_1h": 0,
          "output": 85,
          "total": 1193
        }
      },
      {
        "group": "model",
        "value": "claude-sonnet-4-5",
        "requests": {
          "owned": 3,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 5,
          "cache_read": 950,
          "cache_write": 150,
          "cache_write_5m": 150,
          "cache_write_1h": 0,
          "output": 55,
          "total": 1160
        }
      }
    ],
    "project": [
      {
        "group": "project",
        "value": "project",
        "requests": {
          "owned": 6,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 13,
          "cache_read": 1350,
          "cache_write": 850,
          "cache_write_5m": 850,
          "cache_write_1h": 0,
          "output": 140,
          "total": 2353
        }
      }
    ]
  },
  "sizes": {
    "count": 6,
    "p50": 303,
    "p90": 551,
    "p99": 551,
    "max": 551
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
2026-09-01 | 6 | 13 | 1,350 | 850 | 140 | 2,353
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
        "owned": 6,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 13,
        "cache_read": 1350,
        "cache_write": 850,
        "cache_write_5m": 850,
        "cache_write_1h": 0,
        "output": 140,
        "total": 2353
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
thr-v1-1jrcj9ma1ddd9v8447dhhbqx8h | claude | project | 3 | 1,105 | 55 | 1,160
thr-v1-3zw06f2nedcgvm2cdmfzvmd8ap | claude | project | 1 | 504 | 40 | 544
thr-v1-4xxb1qx9sn4bjzm8vmgtzak2tx | claude | project | 2 | 604 | 45 | 649
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
      "thread": "thr-v1-1jrcj9ma1ddd9v8447dhhbqx8h",
      "agent": "claude",
      "project": "project",
      "requests": {
        "owned": 3,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 5,
        "cache_read": 950,
        "cache_write": 150,
        "cache_write_5m": 150,
        "cache_write_1h": 0,
        "output": 55,
        "total": 1160
      }
    },
    {
      "thread": "thr-v1-3zw06f2nedcgvm2cdmfzvmd8ap",
      "agent": "claude",
      "project": "project",
      "requests": {
        "owned": 1,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 4,
        "cache_read": 0,
        "cache_write": 500,
        "cache_write_5m": 500,
        "cache_write_1h": 0,
        "output": 40,
        "total": 544
      }
    },
    {
      "thread": "thr-v1-4xxb1qx9sn4bjzm8vmgtzak2tx",
      "agent": "claude",
      "project": "project",
      "requests": {
        "owned": 2,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 4,
        "cache_read": 400,
        "cache_write": 200,
        "cache_write_5m": 200,
        "cache_write_1h": 0,
        "output": 45,
        "total": 649
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
thr-v1-1jrcj9ma1ddd9v8447dhhbqx8h | claude | project | 3 | 1,105 | 55 | 1,160
thr-v1-3zw06f2nedcgvm2cdmfzvmd8ap | claude | project | 1 | 504 | 40 | 544
thr-v1-4xxb1qx9sn4bjzm8vmgtzak2tx | claude | project | 2 | 604 | 45 | 649
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
      "thread": "thr-v1-1jrcj9ma1ddd9v8447dhhbqx8h",
      "agent": "claude",
      "project": "project",
      "requests": {
        "owned": 3,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 5,
        "cache_read": 950,
        "cache_write": 150,
        "cache_write_5m": 150,
        "cache_write_1h": 0,
        "output": 55,
        "total": 1160
      }
    },
    {
      "thread": "thr-v1-3zw06f2nedcgvm2cdmfzvmd8ap",
      "agent": "claude",
      "project": "project",
      "requests": {
        "owned": 1,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 4,
        "cache_read": 0,
        "cache_write": 500,
        "cache_write_5m": 500,
        "cache_write_1h": 0,
        "output": 40,
        "total": 544
      }
    },
    {
      "thread": "thr-v1-4xxb1qx9sn4bjzm8vmgtzak2tx",
      "agent": "claude",
      "project": "project",
      "requests": {
        "owned": 2,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 4,
        "cache_read": 400,
        "cache_write": 200,
        "cache_write_5m": 200,
        "cache_write_1h": 0,
        "output": 45,
        "total": 649
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
