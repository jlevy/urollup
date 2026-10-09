---
sandbox: ../../../../crates/urollup-core/tests/fixtures/codex-rollout/compaction-lowered-total
path:
  - $UROLLUP_BIN
env:
  CLAUDE_CONFIG_DIR: $GOLDEN_EMPTY_ROOT
  CODEX_HOME: .
  PI_CODING_AGENT_SESSION_DIR: $GOLDEN_EMPTY_ROOT/sessions
---
# E2E: codex-rollout/compaction-lowered-total

The fixture case
[`codex-rollout/compaction-lowered-total`](../../../../crates/urollup-core/tests/fixtures/codex-rollout/compaction-lowered-total/)
read through `CODEX_HOME` from a sandbox copy, with every other discovery root empty and
HOME hermetic. `make e2e-results` checks its reconciled results against `expected.json`;
this session records the complete output of each view for review.

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
Uncached input 46,000
Cache read     213,000
Cache write    0
Output         6,000
Reasoning      2,300
Total tokens   265,000

COVERAGE
Status complete  Copies excluded 0  Limit observations 0
Unresolved 0  Possible 0  Requests without usage 0

REQUEST SIZES (inclusive input tokens)
Count 5  p50 40,000  p90 120,000  p99 120,000  max 120,000

ACCOUNT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
unknown | 5 | 259,000 | 6,000 | 265,000

EFFORT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
high | 3 | 169,000 | 3,500 | 172,500
medium | 2 | 90,000 | 2,500 | 92,500

MODEL BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
gpt-5.2-codex | 5 | 259,000 | 6,000 | 265,000

PROJECT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
project | 5 | 259,000 | 6,000 | 265,000

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
      "owned": 5,
      "ambiguous": 0,
      "unknown": 0
    },
    "tokens": {
      "uncached_input": 46000,
      "cache_read": 213000,
      "cache_write": 0,
      "cache_write_unspecified": 0,
      "output": 6000,
      "reasoning": 2300,
      "total": 265000
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
          "uncached_input": 46000,
          "cache_read": 213000,
          "cache_write": 0,
          "cache_write_unspecified": 0,
          "output": 6000,
          "reasoning": 2300,
          "total": 265000
        }
      }
    ],
    "effort": [
      {
        "group": "effort",
        "value": "high",
        "requests": {
          "owned": 3,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 31000,
          "cache_read": 138000,
          "cache_write": 0,
          "cache_write_unspecified": 0,
          "output": 3500,
          "reasoning": 1300,
          "total": 172500
        }
      },
      {
        "group": "effort",
        "value": "medium",
        "requests": {
          "owned": 2,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 15000,
          "cache_read": 75000,
          "cache_write": 0,
          "cache_write_unspecified": 0,
          "output": 2500,
          "reasoning": 1000,
          "total": 92500
        }
      }
    ],
    "model": [
      {
        "group": "model",
        "value": "gpt-5.2-codex",
        "requests": {
          "owned": 5,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 46000,
          "cache_read": 213000,
          "cache_write": 0,
          "cache_write_unspecified": 0,
          "output": 6000,
          "reasoning": 2300,
          "total": 265000
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
          "uncached_input": 46000,
          "cache_read": 213000,
          "cache_write": 0,
          "cache_write_unspecified": 0,
          "output": 6000,
          "reasoning": 2300,
          "total": 265000
        }
      }
    ]
  },
  "sizes": {
    "count": 5,
    "p50": 40000,
    "p90": 120000,
    "p99": 120000,
    "max": 120000
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
2026-09-13 | 5 | 46,000 | 213,000 | 0 | 6,000 | 265,000

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
      "date": "2026-09-13",
      "requests": {
        "owned": 5,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 46000,
        "cache_read": 213000,
        "cache_write": 0,
        "cache_write_unspecified": 0,
        "output": 6000,
        "reasoning": 2300,
        "total": 265000
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
thr-v1-1v07wwy6vy0e39tjw13hkhb8gc | codex | project | 5 | 259,000 | 6,000 | 265,000

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
      "thread": "thr-v1-1v07wwy6vy0e39tjw13hkhb8gc",
      "session": "019f0000-0000-7000-8000-001500000001",
      "agent": "codex",
      "project": "project",
      "requests": {
        "owned": 5,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 46000,
        "cache_read": 213000,
        "cache_write": 0,
        "cache_write_unspecified": 0,
        "output": 6000,
        "reasoning": 2300,
        "total": 265000
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
thr-v1-1v07wwy6vy0e39tjw13hkhb8gc | codex | project | 5 | 259,000 | 6,000 | 265,000

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
      "thread": "thr-v1-1v07wwy6vy0e39tjw13hkhb8gc",
      "session": "019f0000-0000-7000-8000-001500000001",
      "agent": "codex",
      "project": "project",
      "requests": {
        "owned": 5,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 46000,
        "cache_read": 213000,
        "cache_write": 0,
        "cache_write_unspecified": 0,
        "output": 6000,
        "reasoning": 2300,
        "total": 265000
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
