---
sandbox: ../../../../crates/urollup-core/tests/fixtures/codex-rollout/auto-review-model
path:
  - $UROLLUP_BIN
env:
  CLAUDE_CONFIG_DIR: $GOLDEN_EMPTY_ROOT
  CODEX_HOME: .
  PI_CODING_AGENT_SESSION_DIR: $GOLDEN_EMPTY_ROOT/sessions
---
# E2E: codex-rollout/auto-review-model

The fixture case
[`codex-rollout/auto-review-model`](../../../../crates/urollup-core/tests/fixtures/codex-rollout/auto-review-model/)
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
Uncached input 4,200
Cache read     6,000
Cache write    0
Output         560
Reasoning      220
Total tokens   10,760

COVERAGE
Status complete  Copies excluded 0  Limit observations 0
Unresolved 0  Possible 0  Requests without usage 0

REQUEST SIZES (inclusive input tokens)
Count 2  p50 2,200  p90 8,000  p99 8,000  max 8,000

ACCOUNT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
unknown | 2 | 10,200 | 560 | 10,760

EFFORT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
high | 1 | 8,000 | 500 | 8,500
unknown | 1 | 2,200 | 60 | 2,260

MODEL BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
gpt-5.2-codex | 1 | 8,000 | 500 | 8,500
unknown | 1 | 2,200 | 60 | 2,260

PROJECT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
project | 2 | 10,200 | 560 | 10,760
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
      "uncached_input": 4200,
      "cache_read": 6000,
      "cache_write": 0,
      "cache_write_unspecified": 0,
      "output": 560,
      "reasoning": 220,
      "total": 10760
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
          "uncached_input": 4200,
          "cache_read": 6000,
          "cache_write": 0,
          "cache_write_unspecified": 0,
          "output": 560,
          "reasoning": 220,
          "total": 10760
        }
      }
    ],
    "effort": [
      {
        "group": "effort",
        "value": "high",
        "requests": {
          "owned": 1,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 2000,
          "cache_read": 6000,
          "cache_write": 0,
          "cache_write_unspecified": 0,
          "output": 500,
          "reasoning": 200,
          "total": 8500
        }
      },
      {
        "group": "effort",
        "value": "unknown",
        "requests": {
          "owned": 1,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 2200,
          "cache_read": 0,
          "cache_write": 0,
          "cache_write_unspecified": 0,
          "output": 60,
          "reasoning": 20,
          "total": 2260
        }
      }
    ],
    "model": [
      {
        "group": "model",
        "value": "gpt-5.2-codex",
        "requests": {
          "owned": 1,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 2000,
          "cache_read": 6000,
          "cache_write": 0,
          "cache_write_unspecified": 0,
          "output": 500,
          "reasoning": 200,
          "total": 8500
        }
      },
      {
        "group": "model",
        "value": "unknown",
        "requests": {
          "owned": 1,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 2200,
          "cache_read": 0,
          "cache_write": 0,
          "cache_write_unspecified": 0,
          "output": 60,
          "reasoning": 20,
          "total": 2260
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
          "uncached_input": 4200,
          "cache_read": 6000,
          "cache_write": 0,
          "cache_write_unspecified": 0,
          "output": 560,
          "reasoning": 220,
          "total": 10760
        }
      }
    ]
  },
  "sizes": {
    "count": 2,
    "p50": 2200,
    "p90": 8000,
    "p99": 8000,
    "max": 8000
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
2026-09-11 | 2 | 4,200 | 6,000 | 0 | 560 | 10,760
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
      "date": "2026-09-11",
      "requests": {
        "owned": 2,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 4200,
        "cache_read": 6000,
        "cache_write": 0,
        "cache_write_unspecified": 0,
        "output": 560,
        "reasoning": 220,
        "total": 10760
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
thr-v1-4rp0v4rxbna4q2qfkhe7yqxj8r | codex | project | 1 | 2,200 | 60 | 2,260
thr-v1-7b011gqs6atykhhpg55am3tbrt | codex | project | 1 | 8,000 | 500 | 8,500
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
      "thread": "thr-v1-4rp0v4rxbna4q2qfkhe7yqxj8r",
      "session": "019f0000-0000-7000-8000-001300000002",
      "agent": "codex",
      "project": "project",
      "requests": {
        "owned": 1,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 2200,
        "cache_read": 0,
        "cache_write": 0,
        "cache_write_unspecified": 0,
        "output": 60,
        "reasoning": 20,
        "total": 2260
      }
    },
    {
      "thread": "thr-v1-7b011gqs6atykhhpg55am3tbrt",
      "session": "019f0000-0000-7000-8000-001300000001",
      "agent": "codex",
      "project": "project",
      "requests": {
        "owned": 1,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 2000,
        "cache_read": 6000,
        "cache_write": 0,
        "cache_write_unspecified": 0,
        "output": 500,
        "reasoning": 200,
        "total": 8500
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
thr-v1-4rp0v4rxbna4q2qfkhe7yqxj8r | codex | project | 1 | 2,200 | 60 | 2,260
thr-v1-7b011gqs6atykhhpg55am3tbrt | codex | project | 1 | 8,000 | 500 | 8,500
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
      "thread": "thr-v1-4rp0v4rxbna4q2qfkhe7yqxj8r",
      "session": "019f0000-0000-7000-8000-001300000002",
      "agent": "codex",
      "project": "project",
      "requests": {
        "owned": 1,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 2200,
        "cache_read": 0,
        "cache_write": 0,
        "cache_write_unspecified": 0,
        "output": 60,
        "reasoning": 20,
        "total": 2260
      }
    },
    {
      "thread": "thr-v1-7b011gqs6atykhhpg55am3tbrt",
      "session": "019f0000-0000-7000-8000-001300000001",
      "agent": "codex",
      "project": "project",
      "requests": {
        "owned": 1,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 2000,
        "cache_read": 6000,
        "cache_write": 0,
        "cache_write_unspecified": 0,
        "output": 500,
        "reasoning": 200,
        "total": 8500
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
