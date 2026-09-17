---
sandbox: ../../../../crates/urollup-core/tests/fixtures/claude-project/advisor-iterations
path:
  - $UROLLUP_BIN
env:
  CLAUDE_CONFIG_DIR: .
  CODEX_HOME: $GOLDEN_EMPTY_ROOT
  PI_CODING_AGENT_SESSION_DIR: $GOLDEN_EMPTY_ROOT/sessions
---
# E2E: claude-project/advisor-iterations

The fixture case
[`claude-project/advisor-iterations`](../../../../crates/urollup-core/tests/fixtures/claude-project/advisor-iterations/)
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
Requests  2
Owned     2
Ambiguous 0
Unknown   0
Uncached input 150,005
Cache read     248,100
Cache write    8,100
Output         7,770
Reasoning      -
Total tokens   413,975

COVERAGE
Status complete  Copies excluded 0  Limit observations 0
Unresolved 0  Possible 0  Requests without usage 0

REQUEST SIZES (inclusive input tokens)
Count 2  p50 8,103  p90 398,102  p99 398,102  max 398,102

ACCOUNT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
unknown | 2 | 406,205 | 7,770 | 413,975

EFFORT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
unknown | 2 | 406,205 | 7,770 | 413,975

MODEL BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
claude-opus-4-5 | 1 | 150,000 | 7,200 | 157,200
claude-sonnet-4-5 | 2 | 256,205 | 570 | 256,775

PROJECT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
project | 2 | 406,205 | 7,770 | 413,975
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
      "owned": 2,
      "ambiguous": 0,
      "unknown": 0
    },
    "tokens": {
      "uncached_input": 150005,
      "cache_read": 248100,
      "cache_write": 8100,
      "cache_write_5m": 8100,
      "cache_write_1h": 0,
      "cache_write_unspecified": 0,
      "output": 7770,
      "total": 413975
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
          "uncached_input": 150005,
          "cache_read": 248100,
          "cache_write": 8100,
          "cache_write_5m": 8100,
          "cache_write_1h": 0,
          "cache_write_unspecified": 0,
          "output": 7770,
          "total": 413975
        }
      }
    ],
    "effort": [
      {
        "group": "effort",
        "value": "unknown",
        "requests": {
          "owned": 2,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 150005,
          "cache_read": 248100,
          "cache_write": 8100,
          "cache_write_5m": 8100,
          "cache_write_1h": 0,
          "cache_write_unspecified": 0,
          "output": 7770,
          "total": 413975
        }
      }
    ],
    "model": [
      {
        "group": "model",
        "value": "claude-opus-4-5",
        "requests": {
          "owned": 1,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 150000,
          "cache_read": 0,
          "cache_write": 0,
          "cache_write_unspecified": 0,
          "output": 7200,
          "total": 157200
        }
      },
      {
        "group": "model",
        "value": "claude-sonnet-4-5",
        "requests": {
          "owned": 2,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 5,
          "cache_read": 248100,
          "cache_write": 8100,
          "cache_write_5m": 8100,
          "cache_write_1h": 0,
          "output": 570,
          "total": 256775
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
          "uncached_input": 150005,
          "cache_read": 248100,
          "cache_write": 8100,
          "cache_write_5m": 8100,
          "cache_write_1h": 0,
          "cache_write_unspecified": 0,
          "output": 7770,
          "total": 413975
        }
      }
    ]
  },
  "sizes": {
    "count": 2,
    "p50": 8103,
    "p90": 398102,
    "p99": 398102,
    "max": 398102
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
2026-09-01 | 2 | 150,005 | 248,100 | 8,100 | 7,770 | 413,975
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
        "owned": 2,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 150005,
        "cache_read": 248100,
        "cache_write": 8100,
        "cache_write_5m": 8100,
        "cache_write_1h": 0,
        "cache_write_unspecified": 0,
        "output": 7770,
        "total": 413975
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
thr-v1-7cgdxjpjx6dsmaqqxsvzhqwnab | claude | project | 2 | 406,205 | 7,770 | 413,975
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
      "thread": "thr-v1-7cgdxjpjx6dsmaqqxsvzhqwnab",
      "session": "00000000-0000-4000-8000-000600000001",
      "agent": "claude",
      "project": "project",
      "requests": {
        "owned": 2,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 150005,
        "cache_read": 248100,
        "cache_write": 8100,
        "cache_write_5m": 8100,
        "cache_write_1h": 0,
        "cache_write_unspecified": 0,
        "output": 7770,
        "total": 413975
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
thr-v1-7cgdxjpjx6dsmaqqxsvzhqwnab | claude | project | 2 | 406,205 | 7,770 | 413,975
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
      "thread": "thr-v1-7cgdxjpjx6dsmaqqxsvzhqwnab",
      "session": "00000000-0000-4000-8000-000600000001",
      "agent": "claude",
      "project": "project",
      "requests": {
        "owned": 2,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 150005,
        "cache_read": 248100,
        "cache_write": 8100,
        "cache_write_5m": 8100,
        "cache_write_1h": 0,
        "cache_write_unspecified": 0,
        "output": 7770,
        "total": 413975
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
