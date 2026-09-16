---
sandbox: ../../../../crates/urollup-core/tests/fixtures/codex-rollout/revert-file
path:
  - $UROLLUP_BIN
env:
  CLAUDE_CONFIG_DIR: $GOLDEN_EMPTY_ROOT
  CODEX_HOME: .
  PI_CODING_AGENT_SESSION_DIR: $GOLDEN_EMPTY_ROOT/sessions
---
# E2E: codex-rollout/revert-file

The fixture case
[`codex-rollout/revert-file`](../../../../crates/urollup-core/tests/fixtures/codex-rollout/revert-file/)
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
Uncached input 7,700
Cache read     7,300
Cache write    0
Output         950
Reasoning      340
Total tokens   15,950

COVERAGE
Status complete  Copies excluded 0  Limit observations 0
Unresolved 0  Possible 0  Requests without usage 0

REQUEST SIZES (inclusive input tokens)
Count 3  p50 5,000  p90 6,000  p99 6,000  max 6,000

ACCOUNT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
unknown | 3 | 15,000 | 950 | 15,950

EFFORT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
medium | 3 | 15,000 | 950 | 15,950

MODEL BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
gpt-5.2-codex | 3 | 15,000 | 950 | 15,950

PROJECT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
project | 3 | 15,000 | 950 | 15,950
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
      "owned": 3,
      "ambiguous": 0,
      "unknown": 0
    },
    "tokens": {
      "uncached_input": 7700,
      "cache_read": 7300,
      "cache_write": 0,
      "cache_write_unspecified": 0,
      "output": 950,
      "reasoning": 340,
      "total": 15950
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
          "uncached_input": 7700,
          "cache_read": 7300,
          "cache_write": 0,
          "cache_write_unspecified": 0,
          "output": 950,
          "reasoning": 340,
          "total": 15950
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
          "uncached_input": 7700,
          "cache_read": 7300,
          "cache_write": 0,
          "cache_write_unspecified": 0,
          "output": 950,
          "reasoning": 340,
          "total": 15950
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
          "uncached_input": 7700,
          "cache_read": 7300,
          "cache_write": 0,
          "cache_write_unspecified": 0,
          "output": 950,
          "reasoning": 340,
          "total": 15950
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
          "uncached_input": 7700,
          "cache_read": 7300,
          "cache_write": 0,
          "cache_write_unspecified": 0,
          "output": 950,
          "reasoning": 340,
          "total": 15950
        }
      }
    ]
  },
  "sizes": {
    "count": 3,
    "p50": 5000,
    "p90": 6000,
    "p99": 6000,
    "max": 6000
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
2026-09-07 | 3 | 7,700 | 7,300 | 0 | 950 | 15,950
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
      "date": "2026-09-07",
      "requests": {
        "owned": 3,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 7700,
        "cache_read": 7300,
        "cache_write": 0,
        "cache_write_unspecified": 0,
        "output": 950,
        "reasoning": 340,
        "total": 15950
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
thr-v1-4fcwaawz9nzs37kyyhszq7t0ww | codex | project | 3 | 15,000 | 950 | 15,950
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
      "thread": "thr-v1-4fcwaawz9nzs37kyyhszq7t0ww",
      "agent": "codex",
      "project": "project",
      "requests": {
        "owned": 3,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 7700,
        "cache_read": 7300,
        "cache_write": 0,
        "cache_write_unspecified": 0,
        "output": 950,
        "reasoning": 340,
        "total": 15950
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
thr-v1-4fcwaawz9nzs37kyyhszq7t0ww | codex | project | 3 | 15,000 | 950 | 15,950
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
      "thread": "thr-v1-4fcwaawz9nzs37kyyhszq7t0ww",
      "agent": "codex",
      "project": "project",
      "requests": {
        "owned": 3,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 7700,
        "cache_read": 7300,
        "cache_write": 0,
        "cache_write_unspecified": 0,
        "output": 950,
        "reasoning": 340,
        "total": 15950
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
