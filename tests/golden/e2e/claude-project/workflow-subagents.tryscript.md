---
sandbox: ../../../../crates/urollup-core/tests/fixtures/claude-project/workflow-subagents
path:
  - $UROLLUP_BIN
env:
  CLAUDE_CONFIG_DIR: .
  CODEX_HOME: $GOLDEN_EMPTY_ROOT
  PI_CODING_AGENT_SESSION_DIR: $GOLDEN_EMPTY_ROOT/sessions
---
# E2E: claude-project/workflow-subagents

The fixture case
[`claude-project/workflow-subagents`](../../../../crates/urollup-core/tests/fixtures/claude-project/workflow-subagents/)
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
Requests  5
Owned     5
Ambiguous 0
Unknown   0
Uncached input 21
Cache read     2,150
Cache write    4,370
Output         181
Reasoning      -
Total tokens   6,722

COVERAGE
Status complete  Copies excluded 0  Limit observations 0
Unresolved 0  Possible 0  Requests without usage 0

REQUEST SIZES (inclusive input tokens)
Count 5  p50 1,506  p90 2,209  p99 2,209  max 2,209

ACCOUNT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
unknown | 5 | 6,541 | 181 | 6,722

EFFORT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
unknown | 5 | 6,541 | 181 | 6,722

MODEL BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
claude-haiku-4-5 | 1 | 2,209 | 64 | 2,273
claude-sonnet-4-5 | 4 | 4,332 | 117 | 4,449

PROJECT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
project | 5 | 6,541 | 181 | 6,722
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
      "uncached_input": 21,
      "cache_read": 2150,
      "cache_write": 4370,
      "cache_write_5m": 4370,
      "cache_write_1h": 0,
      "output": 181,
      "total": 6722
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
          "uncached_input": 21,
          "cache_read": 2150,
          "cache_write": 4370,
          "cache_write_5m": 4370,
          "cache_write_1h": 0,
          "output": 181,
          "total": 6722
        }
      }
    ],
    "effort": [
      {
        "group": "effort",
        "value": "unknown",
        "requests": {
          "owned": 5,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 21,
          "cache_read": 2150,
          "cache_write": 4370,
          "cache_write_5m": 4370,
          "cache_write_1h": 0,
          "output": 181,
          "total": 6722
        }
      }
    ],
    "model": [
      {
        "group": "model",
        "value": "claude-haiku-4-5",
        "requests": {
          "owned": 1,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 9,
          "cache_read": 0,
          "cache_write": 2200,
          "cache_write_5m": 2200,
          "cache_write_1h": 0,
          "output": 64,
          "total": 2273
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
          "uncached_input": 12,
          "cache_read": 2150,
          "cache_write": 2170,
          "cache_write_5m": 2170,
          "cache_write_1h": 0,
          "output": 117,
          "total": 4449
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
          "uncached_input": 21,
          "cache_read": 2150,
          "cache_write": 4370,
          "cache_write_5m": 4370,
          "cache_write_1h": 0,
          "output": 181,
          "total": 6722
        }
      }
    ]
  },
  "sizes": {
    "count": 5,
    "p50": 1506,
    "p90": 2209,
    "p99": 2209,
    "max": 2209
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
2026-09-01 | 5 | 21 | 2,150 | 4,370 | 181 | 6,722
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
      "date": "2026-09-01",
      "requests": {
        "owned": 5,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 21,
        "cache_read": 2150,
        "cache_write": 4370,
        "cache_write_5m": 4370,
        "cache_write_1h": 0,
        "output": 181,
        "total": 6722
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
thr-v1-1rc9p7g58kc9gj4qwggfc6cntk | claude | project | 2 | 3,047 | 47 | 3,094
thr-v1-67y32e2f5r91t17t5p4yy13897 | claude | project | 1 | 2,209 | 64 | 2,273
thr-v1-7vstdg2csvw6av4gh4hw1922aa | claude | project | 2 | 1,285 | 70 | 1,355
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
      "thread": "thr-v1-1rc9p7g58kc9gj4qwggfc6cntk",
      "agent": "claude",
      "project": "project",
      "requests": {
        "owned": 2,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 7,
        "cache_read": 1500,
        "cache_write": 1540,
        "cache_write_5m": 1540,
        "cache_write_1h": 0,
        "output": 47,
        "total": 3094
      }
    },
    {
      "thread": "thr-v1-67y32e2f5r91t17t5p4yy13897",
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
        "cache_write": 2200,
        "cache_write_5m": 2200,
        "cache_write_1h": 0,
        "output": 64,
        "total": 2273
      }
    },
    {
      "thread": "thr-v1-7vstdg2csvw6av4gh4hw1922aa",
      "agent": "claude",
      "project": "project",
      "requests": {
        "owned": 2,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 5,
        "cache_read": 650,
        "cache_write": 630,
        "cache_write_5m": 630,
        "cache_write_1h": 0,
        "output": 70,
        "total": 1355
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
thr-v1-1rc9p7g58kc9gj4qwggfc6cntk | claude | project | 2 | 3,047 | 47 | 3,094
thr-v1-67y32e2f5r91t17t5p4yy13897 | claude | project | 1 | 2,209 | 64 | 2,273
thr-v1-7vstdg2csvw6av4gh4hw1922aa | claude | project | 2 | 1,285 | 70 | 1,355
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
      "thread": "thr-v1-1rc9p7g58kc9gj4qwggfc6cntk",
      "agent": "claude",
      "project": "project",
      "requests": {
        "owned": 2,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 7,
        "cache_read": 1500,
        "cache_write": 1540,
        "cache_write_5m": 1540,
        "cache_write_1h": 0,
        "output": 47,
        "total": 3094
      }
    },
    {
      "thread": "thr-v1-67y32e2f5r91t17t5p4yy13897",
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
        "cache_write": 2200,
        "cache_write_5m": 2200,
        "cache_write_1h": 0,
        "output": 64,
        "total": 2273
      }
    },
    {
      "thread": "thr-v1-7vstdg2csvw6av4gh4hw1922aa",
      "agent": "claude",
      "project": "project",
      "requests": {
        "owned": 2,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 5,
        "cache_read": 650,
        "cache_write": 630,
        "cache_write_5m": 630,
        "cache_write_1h": 0,
        "output": 70,
        "total": 1355
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
