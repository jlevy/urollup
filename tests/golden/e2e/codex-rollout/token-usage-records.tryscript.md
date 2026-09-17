---
sandbox: ../../../../crates/urollup-core/tests/fixtures/codex-rollout/token-usage-records
path:
  - $UROLLUP_BIN
env:
  CLAUDE_CONFIG_DIR: $GOLDEN_EMPTY_ROOT
  CODEX_HOME: .
  PI_CODING_AGENT_SESSION_DIR: $GOLDEN_EMPTY_ROOT/sessions
---
# E2E: codex-rollout/token-usage-records

The fixture case
[`codex-rollout/token-usage-records`](../../../../crates/urollup-core/tests/fixtures/codex-rollout/token-usage-records/)
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
Uncached input 11,000
Cache read     59,000
Cache write    0
Output         3,200
Reasoning      950
Total tokens   73,200

COVERAGE
Status complete  Copies excluded 5  Limit observations 0
Unresolved 0  Possible 0  Requests without usage 0

REQUEST SIZES (inclusive input tokens)
Count 4  p50 14,000  p90 26,000  p99 26,000  max 26,000

ACCOUNT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
unknown | 4 | 70,000 | 3,200 | 73,200

EFFORT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
high | 4 | 70,000 | 3,200 | 73,200

MODEL BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
gpt-5.2-codex | 4 | 70,000 | 3,200 | 73,200

PROJECT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
project | 4 | 70,000 | 3,200 | 73,200
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
    "copies_excluded": 5,
    "limit_observations": 0,
    "requests_without_usage": 0,
    "complete": true
  },
  "diagnostics": [],
  "totals": {
    "requests": {
      "owned": 4,
      "ambiguous": 0,
      "unknown": 0
    },
    "tokens": {
      "uncached_input": 11000,
      "cache_read": 59000,
      "cache_write": 0,
      "cache_write_unspecified": 0,
      "output": 3200,
      "reasoning": 950,
      "total": 73200
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
          "uncached_input": 11000,
          "cache_read": 59000,
          "cache_write": 0,
          "cache_write_unspecified": 0,
          "output": 3200,
          "reasoning": 950,
          "total": 73200
        }
      }
    ],
    "effort": [
      {
        "group": "effort",
        "value": "high",
        "requests": {
          "owned": 4,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 11000,
          "cache_read": 59000,
          "cache_write": 0,
          "cache_write_unspecified": 0,
          "output": 3200,
          "reasoning": 950,
          "total": 73200
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
          "uncached_input": 11000,
          "cache_read": 59000,
          "cache_write": 0,
          "cache_write_unspecified": 0,
          "output": 3200,
          "reasoning": 950,
          "total": 73200
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
          "uncached_input": 11000,
          "cache_read": 59000,
          "cache_write": 0,
          "cache_write_unspecified": 0,
          "output": 3200,
          "reasoning": 950,
          "total": 73200
        }
      }
    ]
  },
  "sizes": {
    "count": 4,
    "p50": 14000,
    "p90": 26000,
    "p99": 26000,
    "max": 26000
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
2026-09-04 | 4 | 11,000 | 59,000 | 0 | 3,200 | 73,200
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
      "date": "2026-09-04",
      "requests": {
        "owned": 4,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 11000,
        "cache_read": 59000,
        "cache_write": 0,
        "cache_write_unspecified": 0,
        "output": 3200,
        "reasoning": 950,
        "total": 73200
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
thr-v1-0q6nd6zx5rsevepqef4zvtfzq4 | codex | project | 3 | 44,000 | 2,400 | 46,400
thr-v1-54qrcx86p7v7y23pp4bzf8c9wv | codex | project | 1 | 26,000 | 800 | 26,800
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
      "thread": "thr-v1-0q6nd6zx5rsevepqef4zvtfzq4",
      "session": "019f0000-0000-7000-8000-000500000001",
      "agent": "codex",
      "project": "project",
      "requests": {
        "owned": 3,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 8000,
        "cache_read": 36000,
        "cache_write": 0,
        "cache_write_unspecified": 0,
        "output": 2400,
        "reasoning": 700,
        "total": 46400
      }
    },
    {
      "thread": "thr-v1-54qrcx86p7v7y23pp4bzf8c9wv",
      "session": "019f0000-0000-7000-8000-000500000002",
      "agent": "codex",
      "project": "project",
      "requests": {
        "owned": 1,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 3000,
        "cache_read": 23000,
        "cache_write": 0,
        "cache_write_unspecified": 0,
        "output": 800,
        "reasoning": 250,
        "total": 26800
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
thr-v1-0q6nd6zx5rsevepqef4zvtfzq4 | codex | project | 3 | 44,000 | 2,400 | 46,400
thr-v1-54qrcx86p7v7y23pp4bzf8c9wv | codex | project | 1 | 26,000 | 800 | 26,800
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
      "thread": "thr-v1-0q6nd6zx5rsevepqef4zvtfzq4",
      "session": "019f0000-0000-7000-8000-000500000001",
      "agent": "codex",
      "project": "project",
      "requests": {
        "owned": 3,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 8000,
        "cache_read": 36000,
        "cache_write": 0,
        "cache_write_unspecified": 0,
        "output": 2400,
        "reasoning": 700,
        "total": 46400
      }
    },
    {
      "thread": "thr-v1-54qrcx86p7v7y23pp4bzf8c9wv",
      "session": "019f0000-0000-7000-8000-000500000002",
      "agent": "codex",
      "project": "project",
      "requests": {
        "owned": 1,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 3000,
        "cache_read": 23000,
        "cache_write": 0,
        "cache_write_unspecified": 0,
        "output": 800,
        "reasoning": 250,
        "total": 26800
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
