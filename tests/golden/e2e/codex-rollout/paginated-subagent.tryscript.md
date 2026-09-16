---
sandbox: ../../../../crates/urollup-core/tests/fixtures/codex-rollout/paginated-subagent
path:
  - $UROLLUP_BIN
env:
  CLAUDE_CONFIG_DIR: $GOLDEN_EMPTY_ROOT
  CODEX_HOME: .
  PI_CODING_AGENT_SESSION_DIR: $GOLDEN_EMPTY_ROOT/sessions
---
# E2E: codex-rollout/paginated-subagent

The fixture case
[`codex-rollout/paginated-subagent`](../../../../crates/urollup-core/tests/fixtures/codex-rollout/paginated-subagent/)
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
Uncached input 8,000
Cache read     6,000
Cache write    0
Output         800
Reasoning      250
Total tokens   14,800

COVERAGE
Status complete  Copies excluded 1  Limit observations 0
Unresolved 0  Possible 0  Requests without usage 0

REQUEST SIZES (inclusive input tokens)
Count 2  p50 5,000  p90 9,000  p99 9,000  max 9,000

ACCOUNT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
unknown | 2 | 14,000 | 800 | 14,800

EFFORT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
medium | 2 | 14,000 | 800 | 14,800

MODEL BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
gpt-5.2-codex | 2 | 14,000 | 800 | 14,800

PROJECT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
project | 2 | 14,000 | 800 | 14,800
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
  "diagnostics": [],
  "totals": {
    "requests": {
      "owned": 2,
      "ambiguous": 0,
      "unknown": 0
    },
    "tokens": {
      "uncached_input": 8000,
      "cache_read": 6000,
      "cache_write": 0,
      "cache_write_unspecified": 0,
      "output": 800,
      "reasoning": 250,
      "total": 14800
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
          "uncached_input": 8000,
          "cache_read": 6000,
          "cache_write": 0,
          "cache_write_unspecified": 0,
          "output": 800,
          "reasoning": 250,
          "total": 14800
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
          "uncached_input": 8000,
          "cache_read": 6000,
          "cache_write": 0,
          "cache_write_unspecified": 0,
          "output": 800,
          "reasoning": 250,
          "total": 14800
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
          "uncached_input": 8000,
          "cache_read": 6000,
          "cache_write": 0,
          "cache_write_unspecified": 0,
          "output": 800,
          "reasoning": 250,
          "total": 14800
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
          "uncached_input": 8000,
          "cache_read": 6000,
          "cache_write": 0,
          "cache_write_unspecified": 0,
          "output": 800,
          "reasoning": 250,
          "total": 14800
        }
      }
    ]
  },
  "sizes": {
    "count": 2,
    "p50": 5000,
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
2026-09-06 | 2 | 8,000 | 6,000 | 0 | 800 | 14,800
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
        "owned": 2,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 8000,
        "cache_read": 6000,
        "cache_write": 0,
        "cache_write_unspecified": 0,
        "output": 800,
        "reasoning": 250,
        "total": 14800
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
thr-v1-28yzdyhyesskjb8263x08vdn2d | codex | project | 1 | 5,000 | 300 | 5,300
thr-v1-3ng9hcwm4psrtr9esgat010hyv | codex | project | 1 | 9,000 | 500 | 9,500
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
      "thread": "thr-v1-28yzdyhyesskjb8263x08vdn2d",
      "agent": "codex",
      "project": "project",
      "requests": {
        "owned": 1,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 3000,
        "cache_read": 2000,
        "cache_write": 0,
        "cache_write_unspecified": 0,
        "output": 300,
        "reasoning": 100,
        "total": 5300
      }
    },
    {
      "thread": "thr-v1-3ng9hcwm4psrtr9esgat010hyv",
      "agent": "codex",
      "project": "project",
      "requests": {
        "owned": 1,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 5000,
        "cache_read": 4000,
        "cache_write": 0,
        "cache_write_unspecified": 0,
        "output": 500,
        "reasoning": 150,
        "total": 9500
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
thr-v1-28yzdyhyesskjb8263x08vdn2d | codex | project | 1 | 5,000 | 300 | 5,300
thr-v1-3ng9hcwm4psrtr9esgat010hyv | codex | project | 1 | 9,000 | 500 | 9,500
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
      "thread": "thr-v1-28yzdyhyesskjb8263x08vdn2d",
      "agent": "codex",
      "project": "project",
      "requests": {
        "owned": 1,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 3000,
        "cache_read": 2000,
        "cache_write": 0,
        "cache_write_unspecified": 0,
        "output": 300,
        "reasoning": 100,
        "total": 5300
      }
    },
    {
      "thread": "thr-v1-3ng9hcwm4psrtr9esgat010hyv",
      "agent": "codex",
      "project": "project",
      "requests": {
        "owned": 1,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 5000,
        "cache_read": 4000,
        "cache_write": 0,
        "cache_write_unspecified": 0,
        "output": 500,
        "reasoning": 150,
        "total": 9500
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
