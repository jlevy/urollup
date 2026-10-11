---
sandbox: ../../../../crates/urollup-core/tests/fixtures/claude-project/resumed-subagent-copy
path:
  - $UROLLUP_BIN
env:
  CLAUDE_CONFIG_DIR: .
  CODEX_HOME: $GOLDEN_EMPTY_ROOT
  PI_CODING_AGENT_SESSION_DIR: $GOLDEN_EMPTY_ROOT/sessions
---
# E2E: claude-project/resumed-subagent-copy

The fixture case
[`claude-project/resumed-subagent-copy`](../../../../crates/urollup-core/tests/fixtures/claude-project/resumed-subagent-copy/)
read through `CLAUDE_CONFIG_DIR` from a sandbox copy, with every other discovery root
empty and HOME hermetic.
`make e2e-results` checks its reconciled results against `expected.json`; this session
records the complete output of each view for review.

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
Uncached input 46
Cache read     7,100
Cache write    2,550
Output         298
Reasoning      -
Total tokens   9,994

COVERAGE
Status complete  Copies excluded 2  Limit observations 0
Unresolved 0  Possible 0  Requests without usage 0

REQUEST SIZES (inclusive input tokens)
Count 7  p50 1,510  p90 2,064  p99 2,064  max 2,064

ACCOUNT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
unknown | 7 | 9,696 | 298 | 9,994

EFFORT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
unknown | 7 | 9,696 | 298 | 9,994

MODEL BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
claude-haiku-4-5 | 3 | 2,424 | 98 | 2,522
claude-sonnet-4-5 | 4 | 7,272 | 200 | 7,472

PROJECT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
project | 7 | 9,696 | 298 | 9,994
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
    "copies_excluded": 2,
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
      "uncached_input": 46,
      "cache_read": 7100,
      "cache_write": 2550,
      "cache_write_5m": 2550,
      "cache_write_1h": 0,
      "output": 298,
      "total": 9994
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
          "uncached_input": 46,
          "cache_read": 7100,
          "cache_write": 2550,
          "cache_write_5m": 2550,
          "cache_write_1h": 0,
          "output": 298,
          "total": 9994
        }
      }
    ],
    "effort": [
      {
        "group": "effort",
        "value": "unknown",
        "requests": {
          "owned": 7,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 46,
          "cache_read": 7100,
          "cache_write": 2550,
          "cache_write_5m": 2550,
          "cache_write_1h": 0,
          "output": 298,
          "total": 9994
        }
      }
    ],
    "model": [
      {
        "group": "model",
        "value": "claude-haiku-4-5",
        "requests": {
          "owned": 3,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 24,
          "cache_read": 800,
          "cache_write": 1600,
          "cache_write_5m": 1600,
          "cache_write_1h": 0,
          "output": 98,
          "total": 2522
        }
      },
      {
        "group": "model",
        "value": "claude-sonnet-4-5",
        "requests": {
          "owned": 4,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 22,
          "cache_read": 6300,
          "cache_write": 950,
          "cache_write_5m": 950,
          "cache_write_1h": 0,
          "output": 200,
          "total": 7472
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
          "uncached_input": 46,
          "cache_read": 7100,
          "cache_write": 2550,
          "cache_write_5m": 2550,
          "cache_write_1h": 0,
          "output": 298,
          "total": 9994
        }
      }
    ]
  },
  "sizes": {
    "count": 7,
    "p50": 1510,
    "p90": 2064,
    "p99": 2064,
    "max": 2064
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
2026-09-06 | 7 | 46 | 7,100 | 2,550 | 298 | 9,994
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
        "owned": 7,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 46,
        "cache_read": 7100,
        "cache_write": 2550,
        "cache_write_5m": 2550,
        "cache_write_1h": 0,
        "output": 298,
        "total": 9994
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
thr-v1-14m8m1hawwrznm6yx254p3ccja | claude | project | 1 | 609 | 33 | 642
thr-v1-5j1sbk2y2typqdg54bfwx47x4z | claude | project | 2 | 3,202 | 110 | 3,312
thr-v1-6qzh9abkv691cp7yw3pqhnsqat | claude | project | 2 | 1,815 | 65 | 1,880
thr-v1-70p6m29demj516fc2ztwdg1cvv | claude | project | 2 | 4,070 | 90 | 4,160
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
      "thread": "thr-v1-14m8m1hawwrznm6yx254p3ccja",
      "session": "00000000-0000-4000-8000-001800000002/a0000000000180002",
      "agent": "claude",
      "project": "project",
      "requests": {
        "owned": 1,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 9,
        "cache_read": 0,
        "cache_write": 600,
        "cache_write_5m": 600,
        "cache_write_1h": 0,
        "output": 33,
        "total": 642
      },
      "last_date": "2026-09-06",
      "undated_requests": 0
    },
    {
      "thread": "thr-v1-5j1sbk2y2typqdg54bfwx47x4z",
      "session": "00000000-0000-4000-8000-001800000001",
      "agent": "claude",
      "project": "project",
      "requests": {
        "owned": 2,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 12,
        "cache_read": 2600,
        "cache_write": 590,
        "cache_write_5m": 590,
        "cache_write_1h": 0,
        "output": 110,
        "total": 3312
      },
      "last_date": "2026-09-06",
      "undated_requests": 0
    },
    {
      "thread": "thr-v1-6qzh9abkv691cp7yw3pqhnsqat",
      "session": "00000000-0000-4000-8000-001800000001/a0000000000180001",
      "agent": "claude",
      "project": "project",
      "requests": {
        "owned": 2,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 15,
        "cache_read": 800,
        "cache_write": 1000,
        "cache_write_5m": 1000,
        "cache_write_1h": 0,
        "output": 65,
        "total": 1880
      },
      "last_date": "2026-09-06",
      "undated_requests": 0
    },
    {
      "thread": "thr-v1-70p6m29demj516fc2ztwdg1cvv",
      "session": "00000000-0000-4000-8000-001800000002",
      "agent": "claude",
      "project": "project",
      "requests": {
        "owned": 2,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 10,
        "cache_read": 3700,
        "cache_write": 360,
        "cache_write_5m": 360,
        "cache_write_1h": 0,
        "output": 90,
        "total": 4160
      },
      "last_date": "2026-09-06",
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
thr-v1-14m8m1hawwrznm6yx254p3ccja | claude | project | 1 | 609 | 33 | 642
thr-v1-5j1sbk2y2typqdg54bfwx47x4z | claude | project | 2 | 3,202 | 110 | 3,312
thr-v1-6qzh9abkv691cp7yw3pqhnsqat | claude | project | 2 | 1,815 | 65 | 1,880
thr-v1-70p6m29demj516fc2ztwdg1cvv | claude | project | 2 | 4,070 | 90 | 4,160
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
      "thread": "thr-v1-14m8m1hawwrznm6yx254p3ccja",
      "session": "00000000-0000-4000-8000-001800000002/a0000000000180002",
      "agent": "claude",
      "project": "project",
      "requests": {
        "owned": 1,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 9,
        "cache_read": 0,
        "cache_write": 600,
        "cache_write_5m": 600,
        "cache_write_1h": 0,
        "output": 33,
        "total": 642
      },
      "last_date": "2026-09-06",
      "undated_requests": 0
    },
    {
      "thread": "thr-v1-5j1sbk2y2typqdg54bfwx47x4z",
      "session": "00000000-0000-4000-8000-001800000001",
      "agent": "claude",
      "project": "project",
      "requests": {
        "owned": 2,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 12,
        "cache_read": 2600,
        "cache_write": 590,
        "cache_write_5m": 590,
        "cache_write_1h": 0,
        "output": 110,
        "total": 3312
      },
      "last_date": "2026-09-06",
      "undated_requests": 0
    },
    {
      "thread": "thr-v1-6qzh9abkv691cp7yw3pqhnsqat",
      "session": "00000000-0000-4000-8000-001800000001/a0000000000180001",
      "agent": "claude",
      "project": "project",
      "requests": {
        "owned": 2,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 15,
        "cache_read": 800,
        "cache_write": 1000,
        "cache_write_5m": 1000,
        "cache_write_1h": 0,
        "output": 65,
        "total": 1880
      },
      "last_date": "2026-09-06",
      "undated_requests": 0
    },
    {
      "thread": "thr-v1-70p6m29demj516fc2ztwdg1cvv",
      "session": "00000000-0000-4000-8000-001800000002",
      "agent": "claude",
      "project": "project",
      "requests": {
        "owned": 2,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 10,
        "cache_read": 3700,
        "cache_write": 360,
        "cache_write_5m": 360,
        "cache_write_1h": 0,
        "output": 90,
        "total": 4160
      },
      "last_date": "2026-09-06",
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
