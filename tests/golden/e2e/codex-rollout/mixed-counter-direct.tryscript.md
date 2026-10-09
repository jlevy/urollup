---
sandbox: ../../../../crates/urollup-core/tests/fixtures/codex-rollout/mixed-counter-direct
path:
  - $UROLLUP_BIN
env:
  CLAUDE_CONFIG_DIR: $GOLDEN_EMPTY_ROOT
  CODEX_HOME: .
  PI_CODING_AGENT_SESSION_DIR: $GOLDEN_EMPTY_ROOT/sessions
---
# E2E: codex-rollout/mixed-counter-direct

The fixture case
[`codex-rollout/mixed-counter-direct`](../../../../crates/urollup-core/tests/fixtures/codex-rollout/mixed-counter-direct/)
read through `CODEX_HOME` from a sandbox copy, with every other discovery root empty and
HOME hermetic. `make e2e-results` checks its reconciled results against `expected.json`;
this session records the complete output of each view for review.

## Report

```console
$ urollup report --all --timezone UTC
urollup report
Selection all  Scope self  Timezone UTC

TOTALS
Requests  7
Owned     7
Ambiguous 0
Unknown   0
Uncached input 36,000
Cache read     98,000
Cache write    0
Output         5,700
Reasoning      1,750
Total tokens   139,700

COVERAGE
Status complete  Copies excluded 0  Limit observations 0
Unresolved 0  Possible 0  Requests without usage 0

REQUEST SIZES (inclusive input tokens)
Count 7  p50 15,000  p90 34,000  p99 34,000  max 34,000

ACCOUNT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
unknown | 7 | 134,000 | 5,700 | 139,700

EFFORT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
high | 4 | 87,000 | 3,000 | 90,000
medium | 3 | 47,000 | 2,700 | 49,700

MODEL BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
gpt-5.1-codex | 3 | 47,000 | 2,700 | 49,700
gpt-5.2-codex | 4 | 87,000 | 3,000 | 90,000

PROJECT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
project | 7 | 134,000 | 5,700 | 139,700
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
      "owned": 7,
      "ambiguous": 0,
      "unknown": 0
    },
    "tokens": {
      "uncached_input": 36000,
      "cache_read": 98000,
      "cache_write": 0,
      "cache_write_unspecified": 0,
      "output": 5700,
      "reasoning": 1750,
      "total": 139700
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
          "owned": 7,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 36000,
          "cache_read": 98000,
          "cache_write": 0,
          "cache_write_unspecified": 0,
          "output": 5700,
          "reasoning": 1750,
          "total": 139700
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
          "uncached_input": 25000,
          "cache_read": 62000,
          "cache_write": 0,
          "cache_write_unspecified": 0,
          "output": 3000,
          "reasoning": 850,
          "total": 90000
        }
      },
      {
        "group": "effort",
        "value": "medium",
        "requests": {
          "owned": 3,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 11000,
          "cache_read": 36000,
          "cache_write": 0,
          "cache_write_unspecified": 0,
          "output": 2700,
          "reasoning": 900,
          "total": 49700
        }
      }
    ],
    "model": [
      {
        "group": "model",
        "value": "gpt-5.1-codex",
        "requests": {
          "owned": 3,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 11000,
          "cache_read": 36000,
          "cache_write": 0,
          "cache_write_unspecified": 0,
          "output": 2700,
          "reasoning": 900,
          "total": 49700
        }
      },
      {
        "group": "model",
        "value": "gpt-5.2-codex",
        "requests": {
          "owned": 4,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 25000,
          "cache_read": 62000,
          "cache_write": 0,
          "cache_write_unspecified": 0,
          "output": 3000,
          "reasoning": 850,
          "total": 90000
        }
      }
    ],
    "project": [
      {
        "group": "project",
        "value": "project",
        "requests": {
          "owned": 7,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 36000,
          "cache_read": 98000,
          "cache_write": 0,
          "cache_write_unspecified": 0,
          "output": 5700,
          "reasoning": 1750,
          "total": 139700
        }
      }
    ]
  },
  "sizes": {
    "count": 7,
    "p50": 15000,
    "p90": 34000,
    "p99": 34000,
    "max": 34000
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
2026-09-18 | 7 | 36,000 | 98,000 | 0 | 5,700 | 139,700
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
      "date": "2026-09-18",
      "requests": {
        "owned": 7,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 36000,
        "cache_read": 98000,
        "cache_write": 0,
        "cache_write_unspecified": 0,
        "output": 5700,
        "reasoning": 1750,
        "total": 139700
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
thr-v1-262jyyetjxsmg2wc05d3txhh68 | codex | project | 5 | 111,000 | 4,500 | 115,500
thr-v1-4bs081r5c7z8t920qtmrps06f0 | codex | project | 2 | 23,000 | 1,200 | 24,200
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
      "thread": "thr-v1-262jyyetjxsmg2wc05d3txhh68",
      "session": "019f0000-0000-7000-8000-001700000001",
      "agent": "codex",
      "project": "project",
      "requests": {
        "owned": 5,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 21000,
        "cache_read": 90000,
        "cache_write": 0,
        "cache_write_unspecified": 0,
        "output": 4500,
        "reasoning": 1450,
        "total": 115500
      }
    },
    {
      "thread": "thr-v1-4bs081r5c7z8t920qtmrps06f0",
      "session": "019f0000-0000-7000-8000-001700000002",
      "agent": "codex",
      "project": "project",
      "requests": {
        "owned": 2,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 15000,
        "cache_read": 8000,
        "cache_write": 0,
        "cache_write_unspecified": 0,
        "output": 1200,
        "reasoning": 300,
        "total": 24200
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
thr-v1-262jyyetjxsmg2wc05d3txhh68 | codex | project | 5 | 111,000 | 4,500 | 115,500
thr-v1-4bs081r5c7z8t920qtmrps06f0 | codex | project | 2 | 23,000 | 1,200 | 24,200
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
      "thread": "thr-v1-262jyyetjxsmg2wc05d3txhh68",
      "session": "019f0000-0000-7000-8000-001700000001",
      "agent": "codex",
      "project": "project",
      "requests": {
        "owned": 5,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 21000,
        "cache_read": 90000,
        "cache_write": 0,
        "cache_write_unspecified": 0,
        "output": 4500,
        "reasoning": 1450,
        "total": 115500
      }
    },
    {
      "thread": "thr-v1-4bs081r5c7z8t920qtmrps06f0",
      "session": "019f0000-0000-7000-8000-001700000002",
      "agent": "codex",
      "project": "project",
      "requests": {
        "owned": 2,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 15000,
        "cache_read": 8000,
        "cache_write": 0,
        "cache_write_unspecified": 0,
        "output": 1200,
        "reasoning": 300,
        "total": 24200
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
