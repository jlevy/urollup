---
sandbox: ../../../../crates/urollup-core/tests/fixtures/claude-project/brief-double-counting
path:
  - $UROLLUP_BIN
env:
  CLAUDE_CONFIG_DIR: .
  CODEX_HOME: $GOLDEN_EMPTY_ROOT
  PI_CODING_AGENT_SESSION_DIR: $GOLDEN_EMPTY_ROOT/sessions
---
# E2E: claude-project/brief-double-counting

The fixture case
[`claude-project/brief-double-counting`](../../../../crates/urollup-core/tests/fixtures/claude-project/brief-double-counting/)
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
Requests  1
Owned     1
Ambiguous 0
Unknown   0
Uncached input 3
Cache read     40,000
Cache write    0
Output         600
Reasoning      -
Total tokens   40,603

COVERAGE
Status complete  Copies excluded 1  Limit observations 0
Unresolved 0  Possible 0  Requests without usage 0

REQUEST SIZES (inclusive input tokens)
Count 1  p50 40,003  p90 40,003  p99 40,003  max 40,003

ACCOUNT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
unknown | 1 | 40,003 | 600 | 40,603

EFFORT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
unknown | 1 | 40,003 | 600 | 40,603

MODEL BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
claude-sonnet-4-5 | 1 | 40,003 | 600 | 40,603

PROJECT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
project | 1 | 40,003 | 600 | 40,603
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
      "owned": 1,
      "ambiguous": 0,
      "unknown": 0
    },
    "tokens": {
      "uncached_input": 3,
      "cache_read": 40000,
      "cache_write": 0,
      "cache_write_5m": 0,
      "cache_write_1h": 0,
      "output": 600,
      "total": 40603
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
          "owned": 1,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 3,
          "cache_read": 40000,
          "cache_write": 0,
          "cache_write_5m": 0,
          "cache_write_1h": 0,
          "output": 600,
          "total": 40603
        }
      }
    ],
    "effort": [
      {
        "group": "effort",
        "value": "unknown",
        "requests": {
          "owned": 1,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 3,
          "cache_read": 40000,
          "cache_write": 0,
          "cache_write_5m": 0,
          "cache_write_1h": 0,
          "output": 600,
          "total": 40603
        }
      }
    ],
    "model": [
      {
        "group": "model",
        "value": "claude-sonnet-4-5",
        "requests": {
          "owned": 1,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 3,
          "cache_read": 40000,
          "cache_write": 0,
          "cache_write_5m": 0,
          "cache_write_1h": 0,
          "output": 600,
          "total": 40603
        }
      }
    ],
    "project": [
      {
        "group": "project",
        "value": "project",
        "requests": {
          "owned": 1,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 3,
          "cache_read": 40000,
          "cache_write": 0,
          "cache_write_5m": 0,
          "cache_write_1h": 0,
          "output": 600,
          "total": 40603
        }
      }
    ]
  },
  "sizes": {
    "count": 1,
    "p50": 40003,
    "p90": 40003,
    "p99": 40003,
    "max": 40003
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
2026-09-01 | 1 | 3 | 40,000 | 0 | 600 | 40,603
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
        "owned": 1,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 3,
        "cache_read": 40000,
        "cache_write": 0,
        "cache_write_5m": 0,
        "cache_write_1h": 0,
        "output": 600,
        "total": 40603
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
thr-v1-3thcne9z8tsbvaysf67z308sxy | claude | project | 0 | - | - | -
thr-v1-651kv0qwp91s22cgbfhfyj1818 | claude | project | 1 | 40,003 | 600 | 40,603
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
      "thread": "thr-v1-3thcne9z8tsbvaysf67z308sxy",
      "agent": "claude",
      "project": "project",
      "requests": {
        "owned": 0,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {}
    },
    {
      "thread": "thr-v1-651kv0qwp91s22cgbfhfyj1818",
      "agent": "claude",
      "project": "project",
      "requests": {
        "owned": 1,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 3,
        "cache_read": 40000,
        "cache_write": 0,
        "cache_write_5m": 0,
        "cache_write_1h": 0,
        "output": 600,
        "total": 40603
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
thr-v1-3thcne9z8tsbvaysf67z308sxy | claude | project | 0 | - | - | -
thr-v1-651kv0qwp91s22cgbfhfyj1818 | claude | project | 1 | 40,003 | 600 | 40,603
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
      "thread": "thr-v1-3thcne9z8tsbvaysf67z308sxy",
      "agent": "claude",
      "project": "project",
      "requests": {
        "owned": 0,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {}
    },
    {
      "thread": "thr-v1-651kv0qwp91s22cgbfhfyj1818",
      "agent": "claude",
      "project": "project",
      "requests": {
        "owned": 1,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 3,
        "cache_read": 40000,
        "cache_write": 0,
        "cache_write_5m": 0,
        "cache_write_1h": 0,
        "output": 600,
        "total": 40603
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
