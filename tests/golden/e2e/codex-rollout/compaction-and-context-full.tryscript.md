---
sandbox: ../../../../crates/urollup-core/tests/fixtures/codex-rollout/compaction-and-context-full
path:
  - $UROLLUP_BIN
env:
  CLAUDE_CONFIG_DIR: $GOLDEN_EMPTY_ROOT
  CODEX_HOME: .
  PI_CODING_AGENT_SESSION_DIR: $GOLDEN_EMPTY_ROOT/sessions
---
# E2E: codex-rollout/compaction-and-context-full

The fixture case
[`codex-rollout/compaction-and-context-full`](../../../../crates/urollup-core/tests/fixtures/codex-rollout/compaction-and-context-full/)
read through `CODEX_HOME` from a sandbox copy, with every other discovery root empty and
HOME hermetic. `make e2e-results` checks its reconciled results against `expected.json`;
this session records the complete output of each view for review.

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
Uncached input 49,000
Cache read     141,000
Cache write    0
Output         4,400
Reasoning      1,700
Total tokens   194,400

COVERAGE
Status complete  Copies excluded 0  Limit observations 0
Unresolved 0  Possible 0  Requests without usage 0

REQUEST SIZES (inclusive input tokens)
Count 3  p50 40,000  p90 120,000  p99 120,000  max 120,000

ACCOUNT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
unknown | 3 | 190,000 | 4,400 | 194,400

EFFORT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
medium | 3 | 190,000 | 4,400 | 194,400

MODEL BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
gpt-5.2-codex | 3 | 190,000 | 4,400 | 194,400

PROJECT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
project | 3 | 190,000 | 4,400 | 194,400

DIAGNOSTICS
codex-counter-epoch-reset x1: Codex cumulative usage decreased and opened a new counter epoch
codex-estimate-compaction x1: Codex emitted an estimate with zero input and output tokens
codex-estimate-context-window-fill x1: Codex emitted an estimate with zero input and output tokens
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
    },
    {
      "code": "codex-estimate-compaction",
      "count": 1,
      "detail": "Codex emitted an estimate with zero input and output tokens"
    },
    {
      "code": "codex-estimate-context-window-fill",
      "count": 1,
      "detail": "Codex emitted an estimate with zero input and output tokens"
    }
  ],
  "totals": {
    "requests": {
      "owned": 3,
      "ambiguous": 0,
      "unknown": 0
    },
    "tokens": {
      "uncached_input": 49000,
      "cache_read": 141000,
      "cache_write": 0,
      "cache_write_unspecified": 0,
      "output": 4400,
      "reasoning": 1700,
      "total": 194400
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
          "uncached_input": 49000,
          "cache_read": 141000,
          "cache_write": 0,
          "cache_write_unspecified": 0,
          "output": 4400,
          "reasoning": 1700,
          "total": 194400
        }
      }
    ],
    "effort": [
      {
        "group": "effort",
        "value": "medium",
        "requests": {
          "owned": 3,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 49000,
          "cache_read": 141000,
          "cache_write": 0,
          "cache_write_unspecified": 0,
          "output": 4400,
          "reasoning": 1700,
          "total": 194400
        }
      }
    ],
    "model": [
      {
        "group": "model",
        "value": "gpt-5.2-codex",
        "requests": {
          "owned": 3,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 49000,
          "cache_read": 141000,
          "cache_write": 0,
          "cache_write_unspecified": 0,
          "output": 4400,
          "reasoning": 1700,
          "total": 194400
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
          "uncached_input": 49000,
          "cache_read": 141000,
          "cache_write": 0,
          "cache_write_unspecified": 0,
          "output": 4400,
          "reasoning": 1700,
          "total": 194400
        }
      }
    ]
  },
  "sizes": {
    "count": 3,
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
2026-09-03 | 3 | 49,000 | 141,000 | 0 | 4,400 | 194,400

DIAGNOSTICS
codex-counter-epoch-reset x1: Codex cumulative usage decreased and opened a new counter epoch
codex-estimate-compaction x1: Codex emitted an estimate with zero input and output tokens
codex-estimate-context-window-fill x1: Codex emitted an estimate with zero input and output tokens
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
        "uncached_input": 49000,
        "cache_read": 141000,
        "cache_write": 0,
        "cache_write_unspecified": 0,
        "output": 4400,
        "reasoning": 1700,
        "total": 194400
      }
    }
  ],
  "diagnostics": [
    {
      "code": "codex-counter-epoch-reset",
      "count": 1,
      "detail": "Codex cumulative usage decreased and opened a new counter epoch"
    },
    {
      "code": "codex-estimate-compaction",
      "count": 1,
      "detail": "Codex emitted an estimate with zero input and output tokens"
    },
    {
      "code": "codex-estimate-context-window-fill",
      "count": 1,
      "detail": "Codex emitted an estimate with zero input and output tokens"
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
thr-v1-3gb8qga70xwagrm0mdc8t6dya2 | codex | project | 3 | 190,000 | 4,400 | 194,400

DIAGNOSTICS
codex-counter-epoch-reset x1: Codex cumulative usage decreased and opened a new counter epoch
codex-estimate-compaction x1: Codex emitted an estimate with zero input and output tokens
codex-estimate-context-window-fill x1: Codex emitted an estimate with zero input and output tokens
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
      "thread": "thr-v1-3gb8qga70xwagrm0mdc8t6dya2",
      "agent": "codex",
      "project": "project",
      "requests": {
        "owned": 3,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 49000,
        "cache_read": 141000,
        "cache_write": 0,
        "cache_write_unspecified": 0,
        "output": 4400,
        "reasoning": 1700,
        "total": 194400
      }
    }
  ],
  "diagnostics": [
    {
      "code": "codex-counter-epoch-reset",
      "count": 1,
      "detail": "Codex cumulative usage decreased and opened a new counter epoch"
    },
    {
      "code": "codex-estimate-compaction",
      "count": 1,
      "detail": "Codex emitted an estimate with zero input and output tokens"
    },
    {
      "code": "codex-estimate-context-window-fill",
      "count": 1,
      "detail": "Codex emitted an estimate with zero input and output tokens"
    }
  ]
}
? 0
```

<!-- This document follows common-doc-guidelines.md.
See github.com/jlevy/practical-prose and review guidelines before editing.
-->
