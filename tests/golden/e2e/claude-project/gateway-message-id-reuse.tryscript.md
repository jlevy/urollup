---
sandbox: ../../../../crates/urollup-core/tests/fixtures/claude-project/gateway-message-id-reuse
path:
  - $UROLLUP_BIN
env:
  CLAUDE_CONFIG_DIR: .
  CODEX_HOME: $GOLDEN_EMPTY_ROOT
  PI_CODING_AGENT_SESSION_DIR: $GOLDEN_EMPTY_ROOT/sessions
---
# E2E: claude-project/gateway-message-id-reuse

The fixture case
[`claude-project/gateway-message-id-reuse`](../../../../crates/urollup-core/tests/fixtures/claude-project/gateway-message-id-reuse/)
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
Uncached input 47
Cache read     0
Cache write    0
Output         145
Reasoning      -
Total tokens   192

COVERAGE
Status complete  Copies excluded 1  Limit observations 0
Unresolved 0  Possible 0  Requests without usage 0

REQUEST SIZES (inclusive input tokens)
Count 3  p50 12  p90 30  p99 30  max 30

ACCOUNT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
unknown | 3 | 47 | 145 | 192

EFFORT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
unknown | 3 | 47 | 145 | 192

MODEL BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
claude-haiku-4-5 | 1 | 30 | 90 | 120
claude-sonnet-4-5 | 2 | 17 | 55 | 72

PROJECT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
project | 3 | 47 | 145 | 192

DIAGNOSTICS
identity-key-conflict x2: Claude message ID msg_gw_000001 is reused by conflicting responses
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
  "diagnostics": [
    {
      "code": "identity-key-conflict",
      "count": 2,
      "detail": "Claude message ID msg_gw_000001 is reused by conflicting responses"
    }
  ],
  "totals": {
    "requests": {
      "owned": 3,
      "ambiguous": 0,
      "unknown": 0
    },
    "tokens": {
      "uncached_input": 47,
      "cache_read": 0,
      "cache_write": 0,
      "cache_write_5m": 0,
      "cache_write_1h": 0,
      "output": 145,
      "total": 192
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
          "uncached_input": 47,
          "cache_read": 0,
          "cache_write": 0,
          "cache_write_5m": 0,
          "cache_write_1h": 0,
          "output": 145,
          "total": 192
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
          "uncached_input": 47,
          "cache_read": 0,
          "cache_write": 0,
          "cache_write_5m": 0,
          "cache_write_1h": 0,
          "output": 145,
          "total": 192
        }
      }
    ],
    "model": [
      {
        "group": "model",
        "value": "claude-haiku-4-5",
        "requests": {
          "owned": 1,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 30,
          "cache_read": 0,
          "cache_write": 0,
          "cache_write_5m": 0,
          "cache_write_1h": 0,
          "output": 90,
          "total": 120
        }
      },
      {
        "group": "model",
        "value": "claude-sonnet-4-5",
        "requests": {
          "owned": 2,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 17,
          "cache_read": 0,
          "cache_write": 0,
          "cache_write_5m": 0,
          "cache_write_1h": 0,
          "output": 55,
          "total": 72
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
          "uncached_input": 47,
          "cache_read": 0,
          "cache_write": 0,
          "cache_write_5m": 0,
          "cache_write_1h": 0,
          "output": 145,
          "total": 192
        }
      }
    ]
  },
  "sizes": {
    "count": 3,
    "p50": 12,
    "p90": 30,
    "p99": 30,
    "max": 30
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
2026-09-03 | 3 | 47 | 0 | 0 | 145 | 192

DIAGNOSTICS
identity-key-conflict x2: Claude message ID msg_gw_000001 is reused by conflicting responses
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
      "date": "2026-09-03",
      "requests": {
        "owned": 3,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 47,
        "cache_read": 0,
        "cache_write": 0,
        "cache_write_5m": 0,
        "cache_write_1h": 0,
        "output": 145,
        "total": 192
      }
    }
  ],
  "diagnostics": [
    {
      "code": "identity-key-conflict",
      "count": 2,
      "detail": "Claude message ID msg_gw_000001 is reused by conflicting responses"
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
thr-v1-1rj51e38bxy9exhyr97n8vhnqf | claude | project | 1 | 5 | 15 | 20
thr-v1-4zveh62q6vepjzgy5hnth0xy26 | claude | project | 1 | 30 | 90 | 120
thr-v1-556ady3m2k0hvd6wen5ccm43vf | claude | project | 1 | 12 | 40 | 52

DIAGNOSTICS
identity-key-conflict x2: Claude message ID msg_gw_000001 is reused by conflicting responses
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
      "thread": "thr-v1-1rj51e38bxy9exhyr97n8vhnqf",
      "agent": "claude",
      "project": "project",
      "requests": {
        "owned": 1,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 5,
        "cache_read": 0,
        "cache_write": 0,
        "cache_write_5m": 0,
        "cache_write_1h": 0,
        "output": 15,
        "total": 20
      }
    },
    {
      "thread": "thr-v1-4zveh62q6vepjzgy5hnth0xy26",
      "agent": "claude",
      "project": "project",
      "requests": {
        "owned": 1,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 30,
        "cache_read": 0,
        "cache_write": 0,
        "cache_write_5m": 0,
        "cache_write_1h": 0,
        "output": 90,
        "total": 120
      }
    },
    {
      "thread": "thr-v1-556ady3m2k0hvd6wen5ccm43vf",
      "agent": "claude",
      "project": "project",
      "requests": {
        "owned": 1,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 12,
        "cache_read": 0,
        "cache_write": 0,
        "cache_write_5m": 0,
        "cache_write_1h": 0,
        "output": 40,
        "total": 52
      }
    }
  ],
  "diagnostics": [
    {
      "code": "identity-key-conflict",
      "count": 2,
      "detail": "Claude message ID msg_gw_000001 is reused by conflicting responses"
    }
  ]
}
? 0
```

<!-- This document follows common-doc-guidelines.md.
See github.com/jlevy/practical-prose and review guidelines before editing.
-->
