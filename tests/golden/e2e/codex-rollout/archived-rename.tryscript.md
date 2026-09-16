---
sandbox: ../../../../crates/urollup-core/tests/fixtures/codex-rollout/archived-rename
path:
  - $UROLLUP_BIN
env:
  CLAUDE_CONFIG_DIR: $GOLDEN_EMPTY_ROOT
  CODEX_HOME: .
  PI_CODING_AGENT_SESSION_DIR: $GOLDEN_EMPTY_ROOT/sessions
---
# E2E: codex-rollout/archived-rename

The fixture case
[`codex-rollout/archived-rename`](../../../../crates/urollup-core/tests/fixtures/codex-rollout/archived-rename/)
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
Uncached input 8,000
Cache read     10,000
Cache write    0
Output         870
Reasoning      240
Total tokens   18,870

COVERAGE
Status complete  Copies excluded 1  Limit observations 0
Unresolved 0  Possible 0  Requests without usage 0

REQUEST SIZES (inclusive input tokens)
Count 5  p50 3,000  p90 6,000  p99 6,000  max 6,000

ACCOUNT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
unknown | 5 | 18,000 | 870 | 18,870

EFFORT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
medium | 4 | 15,500 | 730 | 16,230
unknown | 1 | 2,500 | 140 | 2,640

MODEL BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
gpt-5.2-codex | 4 | 15,500 | 730 | 16,230
unknown | 1 | 2,500 | 140 | 2,640

PROJECT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
project | 5 | 18,000 | 870 | 18,870

DIAGNOSTICS
codex-rollout-duplicate-location x2: the same Codex thread and rollout were found at multiple locations
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
    "copies_excluded": 1,
    "limit_observations": 0,
    "requests_without_usage": 0,
    "complete": true
  },
  "diagnostics": [
    {
      "code": "codex-rollout-duplicate-location",
      "count": 2,
      "detail": "the same Codex thread and rollout were found at multiple locations"
    }
  ],
  "totals": {
    "requests": {
      "owned": 5,
      "ambiguous": 0,
      "unknown": 0
    },
    "tokens": {
      "uncached_input": 8000,
      "cache_read": 10000,
      "cache_write": 0,
      "cache_write_unspecified": 0,
      "output": 870,
      "reasoning": 240,
      "total": 18870
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
          "uncached_input": 8000,
          "cache_read": 10000,
          "cache_write": 0,
          "cache_write_unspecified": 0,
          "output": 870,
          "reasoning": 240,
          "total": 18870
        }
      }
    ],
    "effort": [
      {
        "group": "effort",
        "value": "medium",
        "requests": {
          "owned": 4,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 6700,
          "cache_read": 8800,
          "cache_write": 0,
          "cache_write_unspecified": 0,
          "output": 730,
          "reasoning": 180,
          "total": 16230
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
          "uncached_input": 1300,
          "cache_read": 1200,
          "cache_write": 0,
          "cache_write_unspecified": 0,
          "output": 140,
          "reasoning": 60,
          "total": 2640
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
          "uncached_input": 6700,
          "cache_read": 8800,
          "cache_write": 0,
          "cache_write_unspecified": 0,
          "output": 730,
          "reasoning": 180,
          "total": 16230
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
          "uncached_input": 1300,
          "cache_read": 1200,
          "cache_write": 0,
          "cache_write_unspecified": 0,
          "output": 140,
          "reasoning": 60,
          "total": 2640
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
          "uncached_input": 8000,
          "cache_read": 10000,
          "cache_write": 0,
          "cache_write_unspecified": 0,
          "output": 870,
          "reasoning": 240,
          "total": 18870
        }
      }
    ]
  },
  "sizes": {
    "count": 5,
    "p50": 3000,
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
2026-09-09 | 2 | 4,200 | 3,800 | 0 | 460 | 8,460
2026-09-10 | 3 | 3,800 | 6,200 | 0 | 410 | 10,410

DIAGNOSTICS
codex-rollout-duplicate-location x2: the same Codex thread and rollout were found at multiple locations
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
      "date": "2026-09-09",
      "requests": {
        "owned": 2,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 4200,
        "cache_read": 3800,
        "cache_write": 0,
        "cache_write_unspecified": 0,
        "output": 460,
        "reasoning": 140,
        "total": 8460
      }
    },
    {
      "date": "2026-09-10",
      "requests": {
        "owned": 3,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 3800,
        "cache_read": 6200,
        "cache_write": 0,
        "cache_write_unspecified": 0,
        "output": 410,
        "reasoning": 100,
        "total": 10410
      }
    }
  ],
  "diagnostics": [
    {
      "code": "codex-rollout-duplicate-location",
      "count": 2,
      "detail": "the same Codex thread and rollout were found at multiple locations"
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
thr-v1-45s60qczzwygzpwds58pf5v07d | codex | project | 1 | 2,500 | 140 | 2,640
thr-v1-4fca36wgr9htq457v896dj2vs7 | codex | project | 1 | 1,500 | 90 | 1,590
thr-v1-6r95ar31vc6jwkadfjqfztzsxx | codex | project | 3 | 14,000 | 640 | 14,640

DIAGNOSTICS
codex-rollout-duplicate-location x2: the same Codex thread and rollout were found at multiple locations
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
      "thread": "thr-v1-45s60qczzwygzpwds58pf5v07d",
      "agent": "codex",
      "project": "project",
      "requests": {
        "owned": 1,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 1300,
        "cache_read": 1200,
        "cache_write": 0,
        "cache_write_unspecified": 0,
        "output": 140,
        "reasoning": 60,
        "total": 2640
      }
    },
    {
      "thread": "thr-v1-4fca36wgr9htq457v896dj2vs7",
      "agent": "codex",
      "project": "project",
      "requests": {
        "owned": 1,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 1500,
        "cache_read": 0,
        "cache_write": 0,
        "cache_write_unspecified": 0,
        "output": 90,
        "reasoning": 0,
        "total": 1590
      }
    },
    {
      "thread": "thr-v1-6r95ar31vc6jwkadfjqfztzsxx",
      "agent": "codex",
      "project": "project",
      "requests": {
        "owned": 3,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 5200,
        "cache_read": 8800,
        "cache_write": 0,
        "cache_write_unspecified": 0,
        "output": 640,
        "reasoning": 180,
        "total": 14640
      }
    }
  ],
  "diagnostics": [
    {
      "code": "codex-rollout-duplicate-location",
      "count": 2,
      "detail": "the same Codex thread and rollout were found at multiple locations"
    }
  ]
}
? 0
```

<!-- This document follows common-doc-guidelines.md.
See github.com/jlevy/practical-prose and review guidelines before editing.
-->
