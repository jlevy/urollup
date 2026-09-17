---
sandbox: ../../../../crates/urollup-core/tests/fixtures/codex-rollout/info-null
path:
  - $UROLLUP_BIN
env:
  CLAUDE_CONFIG_DIR: $GOLDEN_EMPTY_ROOT
  CODEX_HOME: .
  PI_CODING_AGENT_SESSION_DIR: $GOLDEN_EMPTY_ROOT/sessions
---
# E2E: codex-rollout/info-null

The fixture case
[`codex-rollout/info-null`](../../../../crates/urollup-core/tests/fixtures/codex-rollout/info-null/)
read through `CODEX_HOME` from a sandbox copy, with every other discovery root empty and
HOME hermetic. `make e2e-results` checks its reconciled results against `expected.json`;
this session records the complete output of each view for review.

## Report

```console
$ urollup report --all --timezone UTC
urollup report
Selection all  Scope self  Timezone UTC

TOTALS
Requests  1
Owned     1
Ambiguous 0
Unknown   0
Uncached input 4,000
Cache read     1,000
Cache write    0
Output         400
Reasoning      100
Total tokens   5,400

COVERAGE
Status complete  Copies excluded 0  Limit observations 4
Unresolved 0  Possible 0  Requests without usage 0

REQUEST SIZES (inclusive input tokens)
Count 1  p50 5,000  p90 5,000  p99 5,000  max 5,000

ACCOUNT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
unknown | 1 | 5,000 | 400 | 5,400

EFFORT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
medium | 1 | 5,000 | 400 | 5,400

MODEL BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
gpt-5.2-codex | 1 | 5,000 | 400 | 5,400

PROJECT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
project | 1 | 5,000 | 400 | 5,400
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
    "limit_observations": 4,
    "requests_without_usage": 0,
    "complete": true
  },
  "diagnostics": [],
  "totals": {
    "requests": {
      "owned": 1,
      "ambiguous": 0,
      "unknown": 0
    },
    "tokens": {
      "uncached_input": 4000,
      "cache_read": 1000,
      "cache_write": 0,
      "cache_write_unspecified": 0,
      "output": 400,
      "reasoning": 100,
      "total": 5400
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
          "owned": 1,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 4000,
          "cache_read": 1000,
          "cache_write": 0,
          "cache_write_unspecified": 0,
          "output": 400,
          "reasoning": 100,
          "total": 5400
        }
      }
    ],
    "effort": [
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
          "cache_read": 1000,
          "cache_write": 0,
          "cache_write_unspecified": 0,
          "output": 400,
          "reasoning": 100,
          "total": 5400
        }
      }
    ],
    "model": [
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
          "cache_read": 1000,
          "cache_write": 0,
          "cache_write_unspecified": 0,
          "output": 400,
          "reasoning": 100,
          "total": 5400
        }
      }
    ],
    "project": [
      {
        "group": "project",
        "value": "project",
        "requests": {
          "owned": 1,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 4000,
          "cache_read": 1000,
          "cache_write": 0,
          "cache_write_unspecified": 0,
          "output": 400,
          "reasoning": 100,
          "total": 5400
        }
      }
    ]
  },
  "sizes": {
    "count": 1,
    "p50": 5000,
    "p90": 5000,
    "p99": 5000,
    "max": 5000
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
2026-09-02 | 1 | 4,000 | 1,000 | 0 | 400 | 5,400
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
      "date": "2026-09-02",
      "requests": {
        "owned": 1,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 4000,
        "cache_read": 1000,
        "cache_write": 0,
        "cache_write_unspecified": 0,
        "output": 400,
        "reasoning": 100,
        "total": 5400
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
thr-v1-52ecpy78vm2b9cs8w39j6beb7s | codex | project | 1 | 5,000 | 400 | 5,400
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
      "thread": "thr-v1-52ecpy78vm2b9cs8w39j6beb7s",
      "session": "019f0000-0000-7000-8000-000200000001",
      "agent": "codex",
      "project": "project",
      "requests": {
        "owned": 1,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 4000,
        "cache_read": 1000,
        "cache_write": 0,
        "cache_write_unspecified": 0,
        "output": 400,
        "reasoning": 100,
        "total": 5400
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
thr-v1-52ecpy78vm2b9cs8w39j6beb7s | codex | project | 1 | 5,000 | 400 | 5,400
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
      "thread": "thr-v1-52ecpy78vm2b9cs8w39j6beb7s",
      "session": "019f0000-0000-7000-8000-000200000001",
      "agent": "codex",
      "project": "project",
      "requests": {
        "owned": 1,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 4000,
        "cache_read": 1000,
        "cache_write": 0,
        "cache_write_unspecified": 0,
        "output": 400,
        "reasoning": 100,
        "total": 5400
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
