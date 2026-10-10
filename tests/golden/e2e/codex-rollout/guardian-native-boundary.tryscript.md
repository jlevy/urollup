---
sandbox: ../../../../crates/urollup-core/tests/fixtures/codex-rollout/guardian-native-boundary
path:
  - $UROLLUP_BIN
env:
  CLAUDE_CONFIG_DIR: $GOLDEN_EMPTY_ROOT
  CODEX_HOME: .
  PI_CODING_AGENT_SESSION_DIR: $GOLDEN_EMPTY_ROOT/sessions
---
# E2E: codex-rollout/guardian-native-boundary

The fixture case
[`codex-rollout/guardian-native-boundary`](../../../../crates/urollup-core/tests/fixtures/codex-rollout/guardian-native-boundary/)
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
Uncached input 7,600
Cache read     10,400
Cache write    0
Output         900
Reasoning      320
Total tokens   18,900

COVERAGE
Status complete  Copies excluded 3  Limit observations 0
Unresolved 0  Possible 0  Requests without usage 0

REQUEST SIZES (inclusive input tokens)
Count 3  p50 3,400  p90 12,000  p99 12,000  max 12,000

ACCOUNT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
unknown | 3 | 18,000 | 900 | 18,900

EFFORT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
low | 2 | 6,000 | 200 | 6,200
medium | 1 | 12,000 | 700 | 12,700

MODEL BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
codex-auto-review | 2 | 6,000 | 200 | 6,200
gpt-5.2-codex | 1 | 12,000 | 700 | 12,700

PROJECT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
project | 3 | 18,000 | 900 | 18,900
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
    "copies_excluded": 3,
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
      "uncached_input": 7600,
      "cache_read": 10400,
      "cache_write": 0,
      "cache_write_unspecified": 0,
      "output": 900,
      "reasoning": 320,
      "total": 18900
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
          "uncached_input": 7600,
          "cache_read": 10400,
          "cache_write": 0,
          "cache_write_unspecified": 0,
          "output": 900,
          "reasoning": 320,
          "total": 18900
        }
      }
    ],
    "effort": [
      {
        "group": "effort",
        "value": "low",
        "requests": {
          "owned": 2,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 3600,
          "cache_read": 2400,
          "cache_write": 0,
          "cache_write_unspecified": 0,
          "output": 200,
          "reasoning": 70,
          "total": 6200
        }
      },
      {
        "group": "effort",
        "value": "medium",
        "requests": {
          "owned": 1,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 4000,
          "cache_read": 8000,
          "cache_write": 0,
          "cache_write_unspecified": 0,
          "output": 700,
          "reasoning": 250,
          "total": 12700
        }
      }
    ],
    "model": [
      {
        "group": "model",
        "value": "codex-auto-review",
        "requests": {
          "owned": 2,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 3600,
          "cache_read": 2400,
          "cache_write": 0,
          "cache_write_unspecified": 0,
          "output": 200,
          "reasoning": 70,
          "total": 6200
        }
      },
      {
        "group": "model",
        "value": "gpt-5.2-codex",
        "requests": {
          "owned": 1,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 4000,
          "cache_read": 8000,
          "cache_write": 0,
          "cache_write_unspecified": 0,
          "output": 700,
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
          "owned": 3,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 7600,
          "cache_read": 10400,
          "cache_write": 0,
          "cache_write_unspecified": 0,
          "output": 900,
          "reasoning": 320,
          "total": 18900
        }
      }
    ]
  },
  "sizes": {
    "count": 3,
    "p50": 3400,
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
2026-10-02 | 3 | 7,600 | 10,400 | 0 | 900 | 18,900
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
      "date": "2026-10-02",
      "requests": {
        "owned": 3,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 7600,
        "cache_read": 10400,
        "cache_write": 0,
        "cache_write_unspecified": 0,
        "output": 900,
        "reasoning": 320,
        "total": 18900
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
thr-v1-2aevmt45kfw2jjhhf7sm4rmxzp | codex | project | 1 | 12,000 | 700 | 12,700
thr-v1-35dmt9ykd81yegzepffgnh36y9 | codex | project | 1 | 3,400 | 110 | 3,510
thr-v1-4w2s0cm2af091f5dj8tcqgtsp4 | codex | project | 1 | 2,600 | 90 | 2,690
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
      "thread": "thr-v1-2aevmt45kfw2jjhhf7sm4rmxzp",
      "session": "019f0000-0000-7000-8000-001900000001",
      "agent": "codex",
      "project": "project",
      "requests": {
        "owned": 1,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 4000,
        "cache_read": 8000,
        "cache_write": 0,
        "cache_write_unspecified": 0,
        "output": 700,
        "reasoning": 250,
        "total": 12700
      },
      "last_date": "2026-10-02",
      "undated_requests": 0
    },
    {
      "thread": "thr-v1-35dmt9ykd81yegzepffgnh36y9",
      "session": "019f0000-0000-7000-8000-001900000003",
      "agent": "codex",
      "project": "project",
      "requests": {
        "owned": 1,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 1000,
        "cache_read": 2400,
        "cache_write": 0,
        "cache_write_unspecified": 0,
        "output": 110,
        "reasoning": 40,
        "total": 3510
      },
      "last_date": "2026-10-02",
      "undated_requests": 0
    },
    {
      "thread": "thr-v1-4w2s0cm2af091f5dj8tcqgtsp4",
      "session": "019f0000-0000-7000-8000-001900000002",
      "agent": "codex",
      "project": "project",
      "requests": {
        "owned": 1,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 2600,
        "cache_read": 0,
        "cache_write": 0,
        "cache_write_unspecified": 0,
        "output": 90,
        "reasoning": 30,
        "total": 2690
      },
      "last_date": "2026-10-02",
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
thr-v1-2aevmt45kfw2jjhhf7sm4rmxzp | codex | project | 1 | 12,000 | 700 | 12,700
thr-v1-35dmt9ykd81yegzepffgnh36y9 | codex | project | 1 | 3,400 | 110 | 3,510
thr-v1-4w2s0cm2af091f5dj8tcqgtsp4 | codex | project | 1 | 2,600 | 90 | 2,690
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
      "thread": "thr-v1-2aevmt45kfw2jjhhf7sm4rmxzp",
      "session": "019f0000-0000-7000-8000-001900000001",
      "agent": "codex",
      "project": "project",
      "requests": {
        "owned": 1,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 4000,
        "cache_read": 8000,
        "cache_write": 0,
        "cache_write_unspecified": 0,
        "output": 700,
        "reasoning": 250,
        "total": 12700
      },
      "last_date": "2026-10-02",
      "undated_requests": 0
    },
    {
      "thread": "thr-v1-35dmt9ykd81yegzepffgnh36y9",
      "session": "019f0000-0000-7000-8000-001900000003",
      "agent": "codex",
      "project": "project",
      "requests": {
        "owned": 1,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 1000,
        "cache_read": 2400,
        "cache_write": 0,
        "cache_write_unspecified": 0,
        "output": 110,
        "reasoning": 40,
        "total": 3510
      },
      "last_date": "2026-10-02",
      "undated_requests": 0
    },
    {
      "thread": "thr-v1-4w2s0cm2af091f5dj8tcqgtsp4",
      "session": "019f0000-0000-7000-8000-001900000002",
      "agent": "codex",
      "project": "project",
      "requests": {
        "owned": 1,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 2600,
        "cache_read": 0,
        "cache_write": 0,
        "cache_write_unspecified": 0,
        "output": 90,
        "reasoning": 30,
        "total": 2690
      },
      "last_date": "2026-10-02",
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
