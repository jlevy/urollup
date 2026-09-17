---
sandbox: ../../../../crates/urollup-core/tests/fixtures/codex-rollout/legacy-user-fork-counters
path:
  - $UROLLUP_BIN
env:
  CLAUDE_CONFIG_DIR: $GOLDEN_EMPTY_ROOT
  CODEX_HOME: .
  PI_CODING_AGENT_SESSION_DIR: $GOLDEN_EMPTY_ROOT/sessions
---
# E2E: codex-rollout/legacy-user-fork-counters

The fixture case
[`codex-rollout/legacy-user-fork-counters`](../../../../crates/urollup-core/tests/fixtures/codex-rollout/legacy-user-fork-counters/)
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
Uncached input 3,500
Cache read     32,500
Cache write    0
Output         1,350
Reasoning      420
Total tokens   37,350

COVERAGE
Status complete  Copies excluded 3  Limit observations 0
Unresolved 0  Possible 0  Requests without usage 0

REQUEST SIZES (inclusive input tokens)
Count 3  p50 12,000  p90 13,000  p99 13,000  max 13,000

ACCOUNT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
unknown | 3 | 36,000 | 1,350 | 37,350

EFFORT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
medium | 3 | 36,000 | 1,350 | 37,350

MODEL BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
gpt-5.2-codex | 3 | 36,000 | 1,350 | 37,350

PROJECT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
project | 3 | 36,000 | 1,350 | 37,350

DIAGNOSTICS
codex-copied-history-inferred x7: Codex copied-history boundary was inferred from legacy rollout records
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
  "diagnostics": [
    {
      "code": "codex-copied-history-inferred",
      "count": 7,
      "detail": "Codex copied-history boundary was inferred from legacy rollout records"
    }
  ],
  "totals": {
    "requests": {
      "owned": 3,
      "ambiguous": 0,
      "unknown": 0
    },
    "tokens": {
      "uncached_input": 3500,
      "cache_read": 32500,
      "cache_write": 0,
      "cache_write_unspecified": 0,
      "output": 1350,
      "reasoning": 420,
      "total": 37350
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
          "uncached_input": 3500,
          "cache_read": 32500,
          "cache_write": 0,
          "cache_write_unspecified": 0,
          "output": 1350,
          "reasoning": 420,
          "total": 37350
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
          "uncached_input": 3500,
          "cache_read": 32500,
          "cache_write": 0,
          "cache_write_unspecified": 0,
          "output": 1350,
          "reasoning": 420,
          "total": 37350
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
          "uncached_input": 3500,
          "cache_read": 32500,
          "cache_write": 0,
          "cache_write_unspecified": 0,
          "output": 1350,
          "reasoning": 420,
          "total": 37350
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
          "uncached_input": 3500,
          "cache_read": 32500,
          "cache_write": 0,
          "cache_write_unspecified": 0,
          "output": 1350,
          "reasoning": 420,
          "total": 37350
        }
      }
    ]
  },
  "sizes": {
    "count": 3,
    "p50": 12000,
    "p90": 13000,
    "p99": 13000,
    "max": 13000
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
2026-09-06 | 3 | 3,500 | 32,500 | 0 | 1,350 | 37,350

DIAGNOSTICS
codex-copied-history-inferred x7: Codex copied-history boundary was inferred from legacy rollout records
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
      "date": "2026-09-06",
      "requests": {
        "owned": 3,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 3500,
        "cache_read": 32500,
        "cache_write": 0,
        "cache_write_unspecified": 0,
        "output": 1350,
        "reasoning": 420,
        "total": 37350
      }
    }
  ],
  "diagnostics": [
    {
      "code": "codex-copied-history-inferred",
      "count": 7,
      "detail": "Codex copied-history boundary was inferred from legacy rollout records"
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
thr-v1-0j42fgmszghqpbfsx55gmnb7de | codex | project | 2 | 23,000 | 750 | 23,750
thr-v1-21nfv67arysdaq0z6vk47cgskm | codex | project | 1 | 13,000 | 600 | 13,600

DIAGNOSTICS
codex-copied-history-inferred x7: Codex copied-history boundary was inferred from legacy rollout records
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
      "thread": "thr-v1-0j42fgmszghqpbfsx55gmnb7de",
      "session": "019f0000-0000-7000-8000-000800000001",
      "agent": "codex",
      "project": "project",
      "requests": {
        "owned": 2,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 3000,
        "cache_read": 20000,
        "cache_write": 0,
        "cache_write_unspecified": 0,
        "output": 750,
        "reasoning": 220,
        "total": 23750
      }
    },
    {
      "thread": "thr-v1-21nfv67arysdaq0z6vk47cgskm",
      "session": "019f0000-0000-7000-8000-000800000002",
      "agent": "codex",
      "project": "project",
      "requests": {
        "owned": 1,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 500,
        "cache_read": 12500,
        "cache_write": 0,
        "cache_write_unspecified": 0,
        "output": 600,
        "reasoning": 200,
        "total": 13600
      }
    }
  ],
  "diagnostics": [
    {
      "code": "codex-copied-history-inferred",
      "count": 7,
      "detail": "Codex copied-history boundary was inferred from legacy rollout records"
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
thr-v1-0j42fgmszghqpbfsx55gmnb7de | codex | project | 2 | 23,000 | 750 | 23,750
thr-v1-21nfv67arysdaq0z6vk47cgskm | codex | project | 1 | 13,000 | 600 | 13,600

DIAGNOSTICS
codex-copied-history-inferred x7: Codex copied-history boundary was inferred from legacy rollout records
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
      "thread": "thr-v1-0j42fgmszghqpbfsx55gmnb7de",
      "session": "019f0000-0000-7000-8000-000800000001",
      "agent": "codex",
      "project": "project",
      "requests": {
        "owned": 2,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 3000,
        "cache_read": 20000,
        "cache_write": 0,
        "cache_write_unspecified": 0,
        "output": 750,
        "reasoning": 220,
        "total": 23750
      }
    },
    {
      "thread": "thr-v1-21nfv67arysdaq0z6vk47cgskm",
      "session": "019f0000-0000-7000-8000-000800000002",
      "agent": "codex",
      "project": "project",
      "requests": {
        "owned": 1,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 500,
        "cache_read": 12500,
        "cache_write": 0,
        "cache_write_unspecified": 0,
        "output": 600,
        "reasoning": 200,
        "total": 13600
      }
    }
  ],
  "diagnostics": [
    {
      "code": "codex-copied-history-inferred",
      "count": 7,
      "detail": "Codex copied-history boundary was inferred from legacy rollout records"
    }
  ]
}
? 0
```

<!-- This document follows common-doc-guidelines.md.
See github.com/jlevy/practical-prose and review guidelines before editing.
-->
