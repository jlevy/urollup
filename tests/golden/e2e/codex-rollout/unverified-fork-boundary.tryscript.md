---
sandbox: ../../../../crates/urollup-core/tests/fixtures/codex-rollout/unverified-fork-boundary
path:
  - $UROLLUP_BIN
env:
  CLAUDE_CONFIG_DIR: $GOLDEN_EMPTY_ROOT
  CODEX_HOME: .
  PI_CODING_AGENT_SESSION_DIR: $GOLDEN_EMPTY_ROOT/sessions
---
# E2E: codex-rollout/unverified-fork-boundary

The fixture case
[`codex-rollout/unverified-fork-boundary`](../../../../crates/urollup-core/tests/fixtures/codex-rollout/unverified-fork-boundary/)
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
Uncached input 5,000
Cache read     2,500
Cache write    0
Output         500
Reasoning      120
Total tokens   8,000

COVERAGE
Status partial  Copies excluded 4  Limit observations 0
Unresolved 0  Possible 0  Requests without usage 0

REQUEST SIZES (inclusive input tokens)
Count 2  p50 1,500  p90 6,000  p99 6,000  max 6,000

ACCOUNT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
unknown | 2 | 7,500 | 500 | 8,000

EFFORT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
medium | 2 | 7,500 | 500 | 8,000

MODEL BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
gpt-5.2-codex | 2 | 7,500 | 500 | 8,000

PROJECT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
project | 2 | 7,500 | 500 | 8,000

DIAGNOSTICS
codex-history-boundary-unverified x3: Codex fork-boundary evidence could not prove which usage is this thread's own, so that usage is excluded as a coverage gap; inspect the rollout's subagent_history_start_ordinal, its record ordinals and its first token_count after the boundary
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
    "copies_excluded": 4,
    "limit_observations": 0,
    "requests_without_usage": 0,
    "complete": false
  },
  "diagnostics": [
    {
      "code": "codex-history-boundary-unverified",
      "count": 3,
      "detail": "Codex fork-boundary evidence could not prove which usage is this thread's own, so that usage is excluded as a coverage gap; inspect the rollout's subagent_history_start_ordinal, its record ordinals and its first token_count after the boundary"
    }
  ],
  "totals": {
    "requests": {
      "owned": 2,
      "ambiguous": 0,
      "unknown": 0
    },
    "tokens": {
      "uncached_input": 5000,
      "cache_read": 2500,
      "cache_write": 0,
      "cache_write_unspecified": 0,
      "output": 500,
      "reasoning": 120,
      "total": 8000
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
          "uncached_input": 5000,
          "cache_read": 2500,
          "cache_write": 0,
          "cache_write_unspecified": 0,
          "output": 500,
          "reasoning": 120,
          "total": 8000
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
          "uncached_input": 5000,
          "cache_read": 2500,
          "cache_write": 0,
          "cache_write_unspecified": 0,
          "output": 500,
          "reasoning": 120,
          "total": 8000
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
          "uncached_input": 5000,
          "cache_read": 2500,
          "cache_write": 0,
          "cache_write_unspecified": 0,
          "output": 500,
          "reasoning": 120,
          "total": 8000
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
          "uncached_input": 5000,
          "cache_read": 2500,
          "cache_write": 0,
          "cache_write_unspecified": 0,
          "output": 500,
          "reasoning": 120,
          "total": 8000
        }
      }
    ]
  },
  "sizes": {
    "count": 2,
    "p50": 1500,
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
2026-09-18 | 2 | 5,000 | 2,500 | 0 | 500 | 8,000

DIAGNOSTICS
codex-history-boundary-unverified x3: Codex fork-boundary evidence could not prove which usage is this thread's own, so that usage is excluded as a coverage gap; inspect the rollout's subagent_history_start_ordinal, its record ordinals and its first token_count after the boundary
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
        "owned": 2,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 5000,
        "cache_read": 2500,
        "cache_write": 0,
        "cache_write_unspecified": 0,
        "output": 500,
        "reasoning": 120,
        "total": 8000
      }
    }
  ],
  "diagnostics": [
    {
      "code": "codex-history-boundary-unverified",
      "count": 3,
      "detail": "Codex fork-boundary evidence could not prove which usage is this thread's own, so that usage is excluded as a coverage gap; inspect the rollout's subagent_history_start_ordinal, its record ordinals and its first token_count after the boundary"
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
thr-v1-3nnt8ce52v76mwgg06hxyx2ckk | codex | project | 1 | 1,500 | 100 | 1,600
thr-v1-4v2gk2vh2xghps98t1xx79s9hr | codex | project | 0 | - | - | -
thr-v1-5v7d5swdnhm3953anp902kf4fc | codex | project | 1 | 6,000 | 400 | 6,400

DIAGNOSTICS
codex-history-boundary-unverified x3: Codex fork-boundary evidence could not prove which usage is this thread's own, so that usage is excluded as a coverage gap; inspect the rollout's subagent_history_start_ordinal, its record ordinals and its first token_count after the boundary
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
      "thread": "thr-v1-3nnt8ce52v76mwgg06hxyx2ckk",
      "session": "019f0000-0000-7000-8000-001600000002",
      "agent": "codex",
      "project": "project",
      "requests": {
        "owned": 1,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 1000,
        "cache_read": 500,
        "cache_write": 0,
        "cache_write_unspecified": 0,
        "output": 100,
        "reasoning": 20,
        "total": 1600
      },
      "last_date": "2026-09-18",
      "undated_requests": 0
    },
    {
      "thread": "thr-v1-4v2gk2vh2xghps98t1xx79s9hr",
      "session": "019f0000-0000-7000-8000-001600000003",
      "agent": "codex",
      "project": "project",
      "requests": {
        "owned": 0,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {},
      "undated_requests": 0
    },
    {
      "thread": "thr-v1-5v7d5swdnhm3953anp902kf4fc",
      "session": "019f0000-0000-7000-8000-001600000001",
      "agent": "codex",
      "project": "project",
      "requests": {
        "owned": 1,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 4000,
        "cache_read": 2000,
        "cache_write": 0,
        "cache_write_unspecified": 0,
        "output": 400,
        "reasoning": 100,
        "total": 6400
      },
      "last_date": "2026-09-18",
      "undated_requests": 0
    }
  ],
  "diagnostics": [
    {
      "code": "codex-history-boundary-unverified",
      "count": 3,
      "detail": "Codex fork-boundary evidence could not prove which usage is this thread's own, so that usage is excluded as a coverage gap; inspect the rollout's subagent_history_start_ordinal, its record ordinals and its first token_count after the boundary"
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
thr-v1-3nnt8ce52v76mwgg06hxyx2ckk | codex | project | 1 | 1,500 | 100 | 1,600
thr-v1-4v2gk2vh2xghps98t1xx79s9hr | codex | project | 0 | - | - | -
thr-v1-5v7d5swdnhm3953anp902kf4fc | codex | project | 1 | 6,000 | 400 | 6,400

DIAGNOSTICS
codex-history-boundary-unverified x3: Codex fork-boundary evidence could not prove which usage is this thread's own, so that usage is excluded as a coverage gap; inspect the rollout's subagent_history_start_ordinal, its record ordinals and its first token_count after the boundary
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
      "thread": "thr-v1-3nnt8ce52v76mwgg06hxyx2ckk",
      "session": "019f0000-0000-7000-8000-001600000002",
      "agent": "codex",
      "project": "project",
      "requests": {
        "owned": 1,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 1000,
        "cache_read": 500,
        "cache_write": 0,
        "cache_write_unspecified": 0,
        "output": 100,
        "reasoning": 20,
        "total": 1600
      },
      "last_date": "2026-09-18",
      "undated_requests": 0
    },
    {
      "thread": "thr-v1-4v2gk2vh2xghps98t1xx79s9hr",
      "session": "019f0000-0000-7000-8000-001600000003",
      "agent": "codex",
      "project": "project",
      "requests": {
        "owned": 0,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {},
      "undated_requests": 0
    },
    {
      "thread": "thr-v1-5v7d5swdnhm3953anp902kf4fc",
      "session": "019f0000-0000-7000-8000-001600000001",
      "agent": "codex",
      "project": "project",
      "requests": {
        "owned": 1,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 4000,
        "cache_read": 2000,
        "cache_write": 0,
        "cache_write_unspecified": 0,
        "output": 400,
        "reasoning": 100,
        "total": 6400
      },
      "last_date": "2026-09-18",
      "undated_requests": 0
    }
  ],
  "diagnostics": [
    {
      "code": "codex-history-boundary-unverified",
      "count": 3,
      "detail": "Codex fork-boundary evidence could not prove which usage is this thread's own, so that usage is excluded as a coverage gap; inspect the rollout's subagent_history_start_ordinal, its record ordinals and its first token_count after the boundary"
    }
  ]
}
? 0
```

<!-- This document follows common-doc-guidelines.md.
See github.com/jlevy/practical-prose and review guidelines before editing.
-->
