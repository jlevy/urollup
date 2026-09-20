---
sandbox: ../../../../crates/urollup-core/tests/fixtures/codex-rollout/counter-reset-epoch
path:
  - $UROLLUP_BIN
env:
  CLAUDE_CONFIG_DIR: $GOLDEN_EMPTY_ROOT
  CODEX_HOME: .
  PI_CODING_AGENT_SESSION_DIR: $GOLDEN_EMPTY_ROOT/sessions
---
# E2E: codex-rollout/counter-reset-epoch

The fixture case
[`codex-rollout/counter-reset-epoch`](../../../../crates/urollup-core/tests/fixtures/codex-rollout/counter-reset-epoch/)
read through `CODEX_HOME` from a sandbox copy, with every other discovery root empty and
HOME hermetic. `make e2e-results` checks its reconciled results against `expected.json`;
this session records the complete output of each view for review.

## Report

```console
$ urollup report --all --timezone UTC
urollup report
Selection all  Scope self  Timezone UTC

TOTALS
Requests  4
Owned     4
Ambiguous 0
Unknown   0
Uncached input 6,000
Cache read     5,500
Cache write    0
Output         1,200
Reasoning      250
Total tokens   12,700

COVERAGE
Status complete  Copies excluded 0  Limit observations 0
Unresolved 0  Possible 0  Requests without usage 0

REQUEST SIZES (inclusive input tokens)
Count 4  p50 2,500  p90 4,000  p99 4,000  max 4,000

ACCOUNT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
unknown | 4 | 11,500 | 1,200 | 12,700

EFFORT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
medium | 4 | 11,500 | 1,200 | 12,700

MODEL BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
gpt-5.2-codex | 4 | 11,500 | 1,200 | 12,700

PROJECT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
project | 4 | 11,500 | 1,200 | 12,700

DIAGNOSTICS
codex-counter-epoch-reset x1: Codex cumulative usage decreased and opened a new counter epoch
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
      "code": "codex-counter-epoch-reset",
      "count": 1,
      "detail": "Codex cumulative usage decreased and opened a new counter epoch"
    }
  ],
  "totals": {
    "requests": {
      "owned": 4,
      "ambiguous": 0,
      "unknown": 0
    },
    "tokens": {
      "uncached_input": 6000,
      "cache_read": 5500,
      "cache_write": 0,
      "cache_write_unspecified": 0,
      "output": 1200,
      "reasoning": 250,
      "total": 12700
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
          "owned": 4,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 6000,
          "cache_read": 5500,
          "cache_write": 0,
          "cache_write_unspecified": 0,
          "output": 1200,
          "reasoning": 250,
          "total": 12700
        }
      }
    ],
    "effort": [
      {
        "group": "effort",
        "value": "medium",
        "requests": {
          "owned": 4,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 6000,
          "cache_read": 5500,
          "cache_write": 0,
          "cache_write_unspecified": 0,
          "output": 1200,
          "reasoning": 250,
          "total": 12700
        }
      }
    ],
    "model": [
      {
        "group": "model",
        "value": "gpt-5.2-codex",
        "requests": {
          "owned": 4,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 6000,
          "cache_read": 5500,
          "cache_write": 0,
          "cache_write_unspecified": 0,
          "output": 1200,
          "reasoning": 250,
          "total": 12700
        }
      }
    ],
    "project": [
      {
        "group": "project",
        "value": "project",
        "requests": {
          "owned": 4,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 6000,
          "cache_read": 5500,
          "cache_write": 0,
          "cache_write_unspecified": 0,
          "output": 1200,
          "reasoning": 250,
          "total": 12700
        }
      }
    ]
  },
  "sizes": {
    "count": 4,
    "p50": 2500,
    "p90": 4000,
    "p99": 4000,
    "max": 4000
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
2026-09-03 | 4 | 6,000 | 5,500 | 0 | 1,200 | 12,700

DIAGNOSTICS
codex-counter-epoch-reset x1: Codex cumulative usage decreased and opened a new counter epoch
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
        "owned": 4,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 6000,
        "cache_read": 5500,
        "cache_write": 0,
        "cache_write_unspecified": 0,
        "output": 1200,
        "reasoning": 250,
        "total": 12700
      }
    }
  ],
  "diagnostics": [
    {
      "code": "codex-counter-epoch-reset",
      "count": 1,
      "detail": "Codex cumulative usage decreased and opened a new counter epoch"
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
thr-v1-4txa2tqmept8jhkgy1sg155cwj | codex | project | 4 | 11,500 | 1,200 | 12,700

DIAGNOSTICS
codex-counter-epoch-reset x1: Codex cumulative usage decreased and opened a new counter epoch
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
      "thread": "thr-v1-4txa2tqmept8jhkgy1sg155cwj",
      "session": "019f0000-0000-7000-8000-000300000001",
      "agent": "codex",
      "project": "project",
      "requests": {
        "owned": 4,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 6000,
        "cache_read": 5500,
        "cache_write": 0,
        "cache_write_unspecified": 0,
        "output": 1200,
        "reasoning": 250,
        "total": 12700
      }
    }
  ],
  "diagnostics": [
    {
      "code": "codex-counter-epoch-reset",
      "count": 1,
      "detail": "Codex cumulative usage decreased and opened a new counter epoch"
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
thr-v1-4txa2tqmept8jhkgy1sg155cwj | codex | project | 4 | 11,500 | 1,200 | 12,700

DIAGNOSTICS
codex-counter-epoch-reset x1: Codex cumulative usage decreased and opened a new counter epoch
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
      "thread": "thr-v1-4txa2tqmept8jhkgy1sg155cwj",
      "session": "019f0000-0000-7000-8000-000300000001",
      "agent": "codex",
      "project": "project",
      "requests": {
        "owned": 4,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 6000,
        "cache_read": 5500,
        "cache_write": 0,
        "cache_write_unspecified": 0,
        "output": 1200,
        "reasoning": 250,
        "total": 12700
      }
    }
  ],
  "diagnostics": [
    {
      "code": "codex-counter-epoch-reset",
      "count": 1,
      "detail": "Codex cumulative usage decreased and opened a new counter epoch"
    }
  ]
}
? 0
```

<!-- This document follows common-doc-guidelines.md.
See github.com/jlevy/practical-prose and review guidelines before editing.
-->
