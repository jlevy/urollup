---
sandbox: ../../../../crates/urollup-core/tests/fixtures/codex-rollout/cache-write-input
path:
  - $UROLLUP_BIN
env:
  CLAUDE_CONFIG_DIR: $GOLDEN_EMPTY_ROOT
  CODEX_HOME: .
  PI_CODING_AGENT_SESSION_DIR: $GOLDEN_EMPTY_ROOT/sessions
---
# E2E: codex-rollout/cache-write-input

The fixture case
[`codex-rollout/cache-write-input`](../../../../crates/urollup-core/tests/fixtures/codex-rollout/cache-write-input/)
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
Uncached input 1,000
Cache read     2,600
Cache write    1,000
Output         180
Reasoning      15
Total tokens   4,780

COVERAGE
Status complete  Copies excluded 0  Limit observations 0
Unresolved 0  Possible 0  Requests without usage 0

REQUEST SIZES (inclusive input tokens)
Count 5  p50 1,000  p90 1,200  p99 1,200  max 1,200

ACCOUNT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
unknown | 5 | 4,600 | 180 | 4,780

EFFORT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
high | 3 | 2,700 | 110 | 2,810
medium | 2 | 1,900 | 70 | 1,970

MODEL BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
gpt-5.2-codex | 5 | 4,600 | 180 | 4,780

PROJECT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
project | 5 | 4,600 | 180 | 4,780
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
      "owned": 5,
      "ambiguous": 0,
      "unknown": 0
    },
    "tokens": {
      "uncached_input": 1000,
      "cache_read": 2600,
      "cache_write": 1000,
      "cache_write_unspecified": 1000,
      "output": 180,
      "reasoning": 15,
      "total": 4780
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
          "uncached_input": 1000,
          "cache_read": 2600,
          "cache_write": 1000,
          "cache_write_unspecified": 1000,
          "output": 180,
          "reasoning": 15,
          "total": 4780
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
          "uncached_input": 700,
          "cache_read": 1500,
          "cache_write": 500,
          "cache_write_unspecified": 500,
          "output": 110,
          "reasoning": 10,
          "total": 2810
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
          "uncached_input": 300,
          "cache_read": 1100,
          "cache_write": 500,
          "cache_write_unspecified": 500,
          "output": 70,
          "reasoning": 5,
          "total": 1970
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
          "uncached_input": 1000,
          "cache_read": 2600,
          "cache_write": 1000,
          "cache_write_unspecified": 1000,
          "output": 180,
          "reasoning": 15,
          "total": 4780
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
          "uncached_input": 1000,
          "cache_read": 2600,
          "cache_write": 1000,
          "cache_write_unspecified": 1000,
          "output": 180,
          "reasoning": 15,
          "total": 4780
        }
      }
    ]
  },
  "sizes": {
    "count": 5,
    "p50": 1000,
    "p90": 1200,
    "p99": 1200,
    "max": 1200
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
2026-09-20 | 5 | 1,000 | 2,600 | 1,000 | 180 | 4,780
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
      "date": "2026-09-20",
      "requests": {
        "owned": 5,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 1000,
        "cache_read": 2600,
        "cache_write": 1000,
        "cache_write_unspecified": 1000,
        "output": 180,
        "reasoning": 15,
        "total": 4780
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
thr-v1-0thtmsbves4yk6dtek6gsybcak | codex | project | 3 | 2,700 | 110 | 2,810
thr-v1-2mfxscg26pf6mw2qwfhc94z59w | codex | project | 2 | 1,900 | 70 | 1,970
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
      "thread": "thr-v1-0thtmsbves4yk6dtek6gsybcak",
      "session": "019f0000-0000-7000-8000-002000000001",
      "agent": "codex",
      "project": "project",
      "requests": {
        "owned": 3,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 700,
        "cache_read": 1500,
        "cache_write": 500,
        "cache_write_unspecified": 500,
        "output": 110,
        "reasoning": 10,
        "total": 2810
      },
      "last_date": "2026-09-20",
      "undated_requests": 0
    },
    {
      "thread": "thr-v1-2mfxscg26pf6mw2qwfhc94z59w",
      "session": "019f0000-0000-7000-8000-002000000002",
      "agent": "codex",
      "project": "project",
      "requests": {
        "owned": 2,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 300,
        "cache_read": 1100,
        "cache_write": 500,
        "cache_write_unspecified": 500,
        "output": 70,
        "reasoning": 5,
        "total": 1970
      },
      "last_date": "2026-09-20",
      "undated_requests": 0
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
thr-v1-0thtmsbves4yk6dtek6gsybcak | codex | project | 3 | 2,700 | 110 | 2,810
thr-v1-2mfxscg26pf6mw2qwfhc94z59w | codex | project | 2 | 1,900 | 70 | 1,970
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
      "thread": "thr-v1-0thtmsbves4yk6dtek6gsybcak",
      "session": "019f0000-0000-7000-8000-002000000001",
      "agent": "codex",
      "project": "project",
      "requests": {
        "owned": 3,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 700,
        "cache_read": 1500,
        "cache_write": 500,
        "cache_write_unspecified": 500,
        "output": 110,
        "reasoning": 10,
        "total": 2810
      },
      "last_date": "2026-09-20",
      "undated_requests": 0
    },
    {
      "thread": "thr-v1-2mfxscg26pf6mw2qwfhc94z59w",
      "session": "019f0000-0000-7000-8000-002000000002",
      "agent": "codex",
      "project": "project",
      "requests": {
        "owned": 2,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 300,
        "cache_read": 1100,
        "cache_write": 500,
        "cache_write_unspecified": 500,
        "output": 70,
        "reasoning": 5,
        "total": 1970
      },
      "last_date": "2026-09-20",
      "undated_requests": 0
    }
  ],
  "diagnostics": []
}
? 0
```

<!-- This document follows common-doc-guidelines.md.
See github.com/jlevy/practical-prose and review guidelines before editing.
-->
