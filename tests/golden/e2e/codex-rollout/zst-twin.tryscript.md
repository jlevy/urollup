---
sandbox: ../../../../crates/urollup-core/tests/fixtures/codex-rollout/zst-twin
path:
  - $UROLLUP_BIN
env:
  CLAUDE_CONFIG_DIR: $GOLDEN_EMPTY_ROOT
  CODEX_HOME: .
  PI_CODING_AGENT_SESSION_DIR: $GOLDEN_EMPTY_ROOT/sessions
---
# E2E: codex-rollout/zst-twin

The fixture case
[`codex-rollout/zst-twin`](../../../../crates/urollup-core/tests/fixtures/codex-rollout/zst-twin/)
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
Uncached input 4,500
Cache read     19,500
Cache write    0
Output         1,070
Reasoning      310
Total tokens   25,070

COVERAGE
Status complete  Copies excluded 0  Limit observations 0
Unresolved 0  Possible 0  Requests without usage 0

REQUEST SIZES (inclusive input tokens)
Count 3  p50 8,000  p90 9,000  p99 9,000  max 9,000

ACCOUNT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
unknown | 3 | 24,000 | 1,070 | 25,070

EFFORT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
medium | 3 | 24,000 | 1,070 | 25,070

MODEL BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
gpt-5.2-codex | 3 | 24,000 | 1,070 | 25,070

PROJECT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
project | 3 | 24,000 | 1,070 | 25,070
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
      "uncached_input": 4500,
      "cache_read": 19500,
      "cache_write": 0,
      "cache_write_unspecified": 0,
      "output": 1070,
      "reasoning": 310,
      "total": 25070
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
          "uncached_input": 4500,
          "cache_read": 19500,
          "cache_write": 0,
          "cache_write_unspecified": 0,
          "output": 1070,
          "reasoning": 310,
          "total": 25070
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
          "uncached_input": 4500,
          "cache_read": 19500,
          "cache_write": 0,
          "cache_write_unspecified": 0,
          "output": 1070,
          "reasoning": 310,
          "total": 25070
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
          "uncached_input": 4500,
          "cache_read": 19500,
          "cache_write": 0,
          "cache_write_unspecified": 0,
          "output": 1070,
          "reasoning": 310,
          "total": 25070
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
          "uncached_input": 4500,
          "cache_read": 19500,
          "cache_write": 0,
          "cache_write_unspecified": 0,
          "output": 1070,
          "reasoning": 310,
          "total": 25070
        }
      }
    ]
  },
  "sizes": {
    "count": 3,
    "p50": 8000,
    "p90": 9000,
    "p99": 9000,
    "max": 9000
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
2026-09-08 | 2 | 3,500 | 11,500 | 0 | 770 | 15,770
2026-09-15 | 1 | 1,000 | 8,000 | 0 | 300 | 9,300
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
      "date": "2026-09-08",
      "requests": {
        "owned": 2,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 3500,
        "cache_read": 11500,
        "cache_write": 0,
        "cache_write_unspecified": 0,
        "output": 770,
        "reasoning": 260,
        "total": 15770
      }
    },
    {
      "date": "2026-09-15",
      "requests": {
        "owned": 1,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 1000,
        "cache_read": 8000,
        "cache_write": 0,
        "cache_write_unspecified": 0,
        "output": 300,
        "reasoning": 50,
        "total": 9300
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
thr-v1-2vkw36et5c63jwxsgmbb6jmrp5 | codex | project | 3 | 24,000 | 1,070 | 25,070
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
      "thread": "thr-v1-2vkw36et5c63jwxsgmbb6jmrp5",
      "agent": "codex",
      "project": "project",
      "requests": {
        "owned": 3,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 4500,
        "cache_read": 19500,
        "cache_write": 0,
        "cache_write_unspecified": 0,
        "output": 1070,
        "reasoning": 310,
        "total": 25070
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
