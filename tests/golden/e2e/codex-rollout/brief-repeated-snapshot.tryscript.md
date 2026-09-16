---
sandbox: ../../../../crates/urollup-core/tests/fixtures/codex-rollout/brief-repeated-snapshot
path:
  - $UROLLUP_BIN
env:
  CLAUDE_CONFIG_DIR: $GOLDEN_EMPTY_ROOT
  CODEX_HOME: .
  PI_CODING_AGENT_SESSION_DIR: $GOLDEN_EMPTY_ROOT/sessions
---
# E2E: codex-rollout/brief-repeated-snapshot

The fixture case
[`codex-rollout/brief-repeated-snapshot`](../../../../crates/urollup-core/tests/fixtures/codex-rollout/brief-repeated-snapshot/)
read through `CODEX_HOME` from a sandbox copy, with every other discovery root empty and
HOME hermetic. `make e2e-results` checks its reconciled results against `expected.json`;
this session records the complete output of each view for review.

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
Uncached input 12,000
Cache read     8,000
Cache write    0
Output         5,000
Reasoning      1,500
Total tokens   25,000

COVERAGE
Status complete  Copies excluded 0  Limit observations 4
Unresolved 0  Possible 0  Requests without usage 0

REQUEST SIZES (inclusive input tokens)
Count 2  p50 8,000  p90 12,000  p99 12,000  max 12,000

ACCOUNT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
unknown | 2 | 20,000 | 5,000 | 25,000

EFFORT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
medium | 2 | 20,000 | 5,000 | 25,000

MODEL BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
gpt-5.2-codex | 2 | 20,000 | 5,000 | 25,000

PROJECT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
project | 2 | 20,000 | 5,000 | 25,000
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
    "limit_observations": 4,
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
      "uncached_input": 12000,
      "cache_read": 8000,
      "cache_write": 0,
      "cache_write_unspecified": 0,
      "output": 5000,
      "reasoning": 1500,
      "total": 25000
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
          "uncached_input": 12000,
          "cache_read": 8000,
          "cache_write": 0,
          "cache_write_unspecified": 0,
          "output": 5000,
          "reasoning": 1500,
          "total": 25000
        }
      }
    ],
    "effort": [
      {
        "group": "effort",
        "value": "medium",
        "requests": {
          "owned": 2,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 12000,
          "cache_read": 8000,
          "cache_write": 0,
          "cache_write_unspecified": 0,
          "output": 5000,
          "reasoning": 1500,
          "total": 25000
        }
      }
    ],
    "model": [
      {
        "group": "model",
        "value": "gpt-5.2-codex",
        "requests": {
          "owned": 2,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 12000,
          "cache_read": 8000,
          "cache_write": 0,
          "cache_write_unspecified": 0,
          "output": 5000,
          "reasoning": 1500,
          "total": 25000
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
          "uncached_input": 12000,
          "cache_read": 8000,
          "cache_write": 0,
          "cache_write_unspecified": 0,
          "output": 5000,
          "reasoning": 1500,
          "total": 25000
        }
      }
    ]
  },
  "sizes": {
    "count": 2,
    "p50": 8000,
    "p90": 12000,
    "p99": 12000,
    "max": 12000
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
2026-09-02 | 2 | 12,000 | 8,000 | 0 | 5,000 | 25,000
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
      "date": "2026-09-02",
      "requests": {
        "owned": 2,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 12000,
        "cache_read": 8000,
        "cache_write": 0,
        "cache_write_unspecified": 0,
        "output": 5000,
        "reasoning": 1500,
        "total": 25000
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
thr-v1-7zrcw4qg6ds2yawtfa2ajb4v09 | codex | project | 2 | 20,000 | 5,000 | 25,000
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
      "thread": "thr-v1-7zrcw4qg6ds2yawtfa2ajb4v09",
      "agent": "codex",
      "project": "project",
      "requests": {
        "owned": 2,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 12000,
        "cache_read": 8000,
        "cache_write": 0,
        "cache_write_unspecified": 0,
        "output": 5000,
        "reasoning": 1500,
        "total": 25000
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
thr-v1-7zrcw4qg6ds2yawtfa2ajb4v09 | codex | project | 2 | 20,000 | 5,000 | 25,000
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
      "thread": "thr-v1-7zrcw4qg6ds2yawtfa2ajb4v09",
      "agent": "codex",
      "project": "project",
      "requests": {
        "owned": 2,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 12000,
        "cache_read": 8000,
        "cache_write": 0,
        "cache_write_unspecified": 0,
        "output": 5000,
        "reasoning": 1500,
        "total": 25000
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
