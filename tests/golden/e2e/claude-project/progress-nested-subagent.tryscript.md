---
sandbox: ../../../../crates/urollup-core/tests/fixtures/claude-project/progress-nested-subagent
path:
  - $UROLLUP_BIN
env:
  CLAUDE_CONFIG_DIR: .
  CODEX_HOME: $GOLDEN_EMPTY_ROOT
  PI_CODING_AGENT_SESSION_DIR: $GOLDEN_EMPTY_ROOT/sessions
---
# E2E: claude-project/progress-nested-subagent

The fixture case
[`claude-project/progress-nested-subagent`](../../../../crates/urollup-core/tests/fixtures/claude-project/progress-nested-subagent/)
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
Uncached input 31
Cache read     4,920
Cache write    1,710
Output         235
Reasoning      -
Total tokens   6,896

COVERAGE
Status partial  Copies excluded 2  Limit observations 0
Unresolved 0  Possible 0  Requests without usage 1

REQUEST SIZES (inclusive input tokens)
Count 5  p50 1,510  p90 1,712  p99 1,712  max 1,712

ACCOUNT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
unknown | 5 | 6,661 | 235 | 6,896

EFFORT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
unknown | 5 | 6,661 | 235 | 6,896

MODEL BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
claude-haiku-4-5 | 2 | 1,815 | 65 | 1,880
claude-sonnet-4-5 | 3 | 4,846 | 170 | 5,016

PROJECT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
project | 5 | 6,661 | 235 | 6,896

DIAGNOSTICS
claude-nested-copy-without-original x1: request observed only as copies; its usage is not counted
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
    "requests_without_usage": 1,
    "complete": false
  },
  "diagnostics": [
    {
      "code": "claude-nested-copy-without-original",
      "count": 1,
      "detail": "request observed only as copies; its usage is not counted"
    }
  ],
  "totals": {
    "requests": {
      "owned": 5,
      "ambiguous": 0,
      "unknown": 0
    },
    "tokens": {
      "uncached_input": 31,
      "cache_read": 4920,
      "cache_write": 1710,
      "cache_write_5m": 1710,
      "cache_write_1h": 0,
      "output": 235,
      "total": 6896
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
          "uncached_input": 31,
          "cache_read": 4920,
          "cache_write": 1710,
          "cache_write_5m": 1710,
          "cache_write_1h": 0,
          "output": 235,
          "total": 6896
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
          "uncached_input": 31,
          "cache_read": 4920,
          "cache_write": 1710,
          "cache_write_5m": 1710,
          "cache_write_1h": 0,
          "output": 235,
          "total": 6896
        }
      }
    ],
    "model": [
      {
        "group": "model",
        "value": "claude-haiku-4-5",
        "requests": {
          "owned": 2,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 15,
          "cache_read": 800,
          "cache_write": 1000,
          "cache_write_5m": 1000,
          "cache_write_1h": 0,
          "output": 65,
          "total": 1880
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
          "uncached_input": 16,
          "cache_read": 4120,
          "cache_write": 710,
          "cache_write_5m": 710,
          "cache_write_1h": 0,
          "output": 170,
          "total": 5016
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
          "uncached_input": 31,
          "cache_read": 4920,
          "cache_write": 1710,
          "cache_write_5m": 1710,
          "cache_write_1h": 0,
          "output": 235,
          "total": 6896
        }
      }
    ]
  },
  "sizes": {
    "count": 5,
    "p50": 1510,
    "p90": 1712,
    "p99": 1712,
    "max": 1712
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
2026-09-01 | 5 | 31 | 4,920 | 1,710 | 235 | 6,896

DIAGNOSTICS
claude-nested-copy-without-original x1: request observed only as copies; its usage is not counted
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
        "owned": 5,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 31,
        "cache_read": 4920,
        "cache_write": 1710,
        "cache_write_5m": 1710,
        "cache_write_1h": 0,
        "output": 235,
        "total": 6896
      }
    }
  ],
  "diagnostics": [
    {
      "code": "claude-nested-copy-without-original",
      "count": 1,
      "detail": "request observed only as copies; its usage is not counted"
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
thr-v1-73p5bgk3r2n007f4h1ex51wek7 | claude | project | 3 | 4,846 | 170 | 5,016
thr-v1-7emcgxspd74ebbze9ahx536sjx | claude | project | 2 | 1,815 | 65 | 1,880

DIAGNOSTICS
claude-nested-copy-without-original x1: request observed only as copies; its usage is not counted
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
      "thread": "thr-v1-73p5bgk3r2n007f4h1ex51wek7",
      "agent": "claude",
      "project": "project",
      "requests": {
        "owned": 3,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 16,
        "cache_read": 4120,
        "cache_write": 710,
        "cache_write_5m": 710,
        "cache_write_1h": 0,
        "output": 170,
        "total": 5016
      }
    },
    {
      "thread": "thr-v1-7emcgxspd74ebbze9ahx536sjx",
      "agent": "claude",
      "project": "project",
      "requests": {
        "owned": 2,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 15,
        "cache_read": 800,
        "cache_write": 1000,
        "cache_write_5m": 1000,
        "cache_write_1h": 0,
        "output": 65,
        "total": 1880
      }
    }
  ],
  "diagnostics": [
    {
      "code": "claude-nested-copy-without-original",
      "count": 1,
      "detail": "request observed only as copies; its usage is not counted"
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
thr-v1-73p5bgk3r2n007f4h1ex51wek7 | claude | project | 3 | 4,846 | 170 | 5,016
thr-v1-7emcgxspd74ebbze9ahx536sjx | claude | project | 2 | 1,815 | 65 | 1,880

DIAGNOSTICS
claude-nested-copy-without-original x1: request observed only as copies; its usage is not counted
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
      "thread": "thr-v1-73p5bgk3r2n007f4h1ex51wek7",
      "agent": "claude",
      "project": "project",
      "requests": {
        "owned": 3,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 16,
        "cache_read": 4120,
        "cache_write": 710,
        "cache_write_5m": 710,
        "cache_write_1h": 0,
        "output": 170,
        "total": 5016
      }
    },
    {
      "thread": "thr-v1-7emcgxspd74ebbze9ahx536sjx",
      "agent": "claude",
      "project": "project",
      "requests": {
        "owned": 2,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 15,
        "cache_read": 800,
        "cache_write": 1000,
        "cache_write_5m": 1000,
        "cache_write_1h": 0,
        "output": 65,
        "total": 1880
      }
    }
  ],
  "diagnostics": [
    {
      "code": "claude-nested-copy-without-original",
      "count": 1,
      "detail": "request observed only as copies; its usage is not counted"
    }
  ]
}
? 0
```

<!-- This document follows common-doc-guidelines.md.
See github.com/jlevy/practical-prose and review guidelines before editing.
-->
