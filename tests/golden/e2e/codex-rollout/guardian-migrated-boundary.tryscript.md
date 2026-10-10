---
sandbox: ../../../../crates/urollup-core/tests/fixtures/codex-rollout/guardian-migrated-boundary
path:
  - $UROLLUP_BIN
env:
  CLAUDE_CONFIG_DIR: $GOLDEN_EMPTY_ROOT
  CODEX_HOME: .
  PI_CODING_AGENT_SESSION_DIR: $GOLDEN_EMPTY_ROOT/sessions
---
# E2E: codex-rollout/guardian-migrated-boundary

The fixture case
[`codex-rollout/guardian-migrated-boundary`](../../../../crates/urollup-core/tests/fixtures/codex-rollout/guardian-migrated-boundary/)
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
Uncached input 10,200
Cache read     10,000
Cache write    0
Output         990
Reasoning      340
Total tokens   21,190

COVERAGE
Status partial  Copies excluded 0  Limit observations 0
Unresolved 0  Possible 0  Requests without usage 0

REQUEST SIZES (inclusive input tokens)
Count 5  p50 3,000  p90 9,000  p99 9,000  max 9,000

ACCOUNT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
unknown | 5 | 20,200 | 990 | 21,190

EFFORT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
low | 3 | 7,200 | 240 | 7,440
medium | 1 | 9,000 | 600 | 9,600
unknown | 1 | 4,000 | 150 | 4,150

MODEL BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
codex-auto-review | 3 | 7,200 | 240 | 7,440
gpt-5.2-codex | 1 | 9,000 | 600 | 9,600
unknown | 1 | 4,000 | 150 | 4,150

PROJECT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
project | 5 | 20,200 | 990 | 21,190

DIAGNOSTICS
codex-history-boundary-unverified x1: Codex fork-boundary evidence could not prove which usage is this thread's own, so that usage is excluded as a coverage gap; inspect the rollout's subagent_history_start_ordinal, its record ordinals, its turn IDs against its parent's and its token_count totals
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
    "complete": false
  },
  "diagnostics": [
    {
      "code": "codex-history-boundary-unverified",
      "count": 1,
      "detail": "Codex fork-boundary evidence could not prove which usage is this thread's own, so that usage is excluded as a coverage gap; inspect the rollout's subagent_history_start_ordinal, its record ordinals, its turn IDs against its parent's and its token_count totals"
    }
  ],
  "totals": {
    "requests": {
      "owned": 5,
      "ambiguous": 0,
      "unknown": 0
    },
    "tokens": {
      "uncached_input": 10200,
      "cache_read": 10000,
      "cache_write": 0,
      "cache_write_unspecified": 0,
      "output": 990,
      "reasoning": 340,
      "total": 21190
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
          "uncached_input": 10200,
          "cache_read": 10000,
          "cache_write": 0,
          "cache_write_unspecified": 0,
          "output": 990,
          "reasoning": 340,
          "total": 21190
        }
      }
    ],
    "effort": [
      {
        "group": "effort",
        "value": "low",
        "requests": {
          "owned": 3,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 4200,
          "cache_read": 3000,
          "cache_write": 0,
          "cache_write_unspecified": 0,
          "output": 240,
          "reasoning": 90,
          "total": 7440
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
          "uncached_input": 3000,
          "cache_read": 6000,
          "cache_write": 0,
          "cache_write_unspecified": 0,
          "output": 600,
          "reasoning": 200,
          "total": 9600
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
          "uncached_input": 3000,
          "cache_read": 1000,
          "cache_write": 0,
          "cache_write_unspecified": 0,
          "output": 150,
          "reasoning": 50,
          "total": 4150
        }
      }
    ],
    "model": [
      {
        "group": "model",
        "value": "codex-auto-review",
        "requests": {
          "owned": 3,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 4200,
          "cache_read": 3000,
          "cache_write": 0,
          "cache_write_unspecified": 0,
          "output": 240,
          "reasoning": 90,
          "total": 7440
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
          "uncached_input": 3000,
          "cache_read": 6000,
          "cache_write": 0,
          "cache_write_unspecified": 0,
          "output": 600,
          "reasoning": 200,
          "total": 9600
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
          "uncached_input": 3000,
          "cache_read": 1000,
          "cache_write": 0,
          "cache_write_unspecified": 0,
          "output": 150,
          "reasoning": 50,
          "total": 4150
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
          "uncached_input": 10200,
          "cache_read": 10000,
          "cache_write": 0,
          "cache_write_unspecified": 0,
          "output": 990,
          "reasoning": 340,
          "total": 21190
        }
      }
    ]
  },
  "sizes": {
    "count": 5,
    "p50": 3000,
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
2026-09-19 | 5 | 10,200 | 10,000 | 0 | 990 | 21,190

DIAGNOSTICS
codex-history-boundary-unverified x1: Codex fork-boundary evidence could not prove which usage is this thread's own, so that usage is excluded as a coverage gap; inspect the rollout's subagent_history_start_ordinal, its record ordinals, its turn IDs against its parent's and its token_count totals
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
      "date": "2026-09-19",
      "requests": {
        "owned": 5,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 10200,
        "cache_read": 10000,
        "cache_write": 0,
        "cache_write_unspecified": 0,
        "output": 990,
        "reasoning": 340,
        "total": 21190
      }
    }
  ],
  "diagnostics": [
    {
      "code": "codex-history-boundary-unverified",
      "count": 1,
      "detail": "Codex fork-boundary evidence could not prove which usage is this thread's own, so that usage is excluded as a coverage gap; inspect the rollout's subagent_history_start_ordinal, its record ordinals, its turn IDs against its parent's and its token_count totals"
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
thr-v1-28kcjsrhm8jqxx1g09tvfcr2nh | codex | project | 1 | 9,000 | 600 | 9,600
thr-v1-6677t9nz0gvmcckg2155pb16rb | codex | project | 2 | 5,400 | 180 | 5,580
thr-v1-7pewbzqqw85gcpmgzy1ng3gvf9 | codex | project | 2 | 5,800 | 210 | 6,010

DIAGNOSTICS
codex-history-boundary-unverified x1: Codex fork-boundary evidence could not prove which usage is this thread's own, so that usage is excluded as a coverage gap; inspect the rollout's subagent_history_start_ordinal, its record ordinals, its turn IDs against its parent's and its token_count totals
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
      "thread": "thr-v1-28kcjsrhm8jqxx1g09tvfcr2nh",
      "session": "019f0000-0000-7000-8000-001800000001",
      "agent": "codex",
      "project": "project",
      "requests": {
        "owned": 1,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 3000,
        "cache_read": 6000,
        "cache_write": 0,
        "cache_write_unspecified": 0,
        "output": 600,
        "reasoning": 200,
        "total": 9600
      },
      "last_date": "2026-09-19",
      "undated_requests": 0
    },
    {
      "thread": "thr-v1-6677t9nz0gvmcckg2155pb16rb",
      "session": "019f0000-0000-7000-8000-001800000002",
      "agent": "codex",
      "project": "project",
      "requests": {
        "owned": 2,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 3400,
        "cache_read": 2000,
        "cache_write": 0,
        "cache_write_unspecified": 0,
        "output": 180,
        "reasoning": 70,
        "total": 5580
      },
      "last_date": "2026-09-19",
      "undated_requests": 0
    },
    {
      "thread": "thr-v1-7pewbzqqw85gcpmgzy1ng3gvf9",
      "session": "019f0000-0000-7000-8000-001800000003",
      "agent": "codex",
      "project": "project",
      "requests": {
        "owned": 2,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 3800,
        "cache_read": 2000,
        "cache_write": 0,
        "cache_write_unspecified": 0,
        "output": 210,
        "reasoning": 70,
        "total": 6010
      },
      "last_date": "2026-09-19",
      "undated_requests": 0
    }
  ],
  "diagnostics": [
    {
      "code": "codex-history-boundary-unverified",
      "count": 1,
      "detail": "Codex fork-boundary evidence could not prove which usage is this thread's own, so that usage is excluded as a coverage gap; inspect the rollout's subagent_history_start_ordinal, its record ordinals, its turn IDs against its parent's and its token_count totals"
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
thr-v1-28kcjsrhm8jqxx1g09tvfcr2nh | codex | project | 1 | 9,000 | 600 | 9,600
thr-v1-6677t9nz0gvmcckg2155pb16rb | codex | project | 2 | 5,400 | 180 | 5,580
thr-v1-7pewbzqqw85gcpmgzy1ng3gvf9 | codex | project | 2 | 5,800 | 210 | 6,010

DIAGNOSTICS
codex-history-boundary-unverified x1: Codex fork-boundary evidence could not prove which usage is this thread's own, so that usage is excluded as a coverage gap; inspect the rollout's subagent_history_start_ordinal, its record ordinals, its turn IDs against its parent's and its token_count totals
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
      "thread": "thr-v1-28kcjsrhm8jqxx1g09tvfcr2nh",
      "session": "019f0000-0000-7000-8000-001800000001",
      "agent": "codex",
      "project": "project",
      "requests": {
        "owned": 1,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 3000,
        "cache_read": 6000,
        "cache_write": 0,
        "cache_write_unspecified": 0,
        "output": 600,
        "reasoning": 200,
        "total": 9600
      },
      "last_date": "2026-09-19",
      "undated_requests": 0
    },
    {
      "thread": "thr-v1-6677t9nz0gvmcckg2155pb16rb",
      "session": "019f0000-0000-7000-8000-001800000002",
      "agent": "codex",
      "project": "project",
      "requests": {
        "owned": 2,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 3400,
        "cache_read": 2000,
        "cache_write": 0,
        "cache_write_unspecified": 0,
        "output": 180,
        "reasoning": 70,
        "total": 5580
      },
      "last_date": "2026-09-19",
      "undated_requests": 0
    },
    {
      "thread": "thr-v1-7pewbzqqw85gcpmgzy1ng3gvf9",
      "session": "019f0000-0000-7000-8000-001800000003",
      "agent": "codex",
      "project": "project",
      "requests": {
        "owned": 2,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 3800,
        "cache_read": 2000,
        "cache_write": 0,
        "cache_write_unspecified": 0,
        "output": 210,
        "reasoning": 70,
        "total": 6010
      },
      "last_date": "2026-09-19",
      "undated_requests": 0
    }
  ],
  "diagnostics": [
    {
      "code": "codex-history-boundary-unverified",
      "count": 1,
      "detail": "Codex fork-boundary evidence could not prove which usage is this thread's own, so that usage is excluded as a coverage gap; inspect the rollout's subagent_history_start_ordinal, its record ordinals, its turn IDs against its parent's and its token_count totals"
    }
  ]
}
? 0
```

<!-- This document follows common-doc-guidelines.md.
See github.com/jlevy/practical-prose and review guidelines before editing.
-->
