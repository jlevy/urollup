---
sandbox: ../../../../crates/urollup-core/tests/fixtures/codex-rollout/pending-tail
path:
  - $UROLLUP_BIN
env:
  CLAUDE_CONFIG_DIR: $GOLDEN_EMPTY_ROOT
  CODEX_HOME: .
  PI_CODING_AGENT_SESSION_DIR: $GOLDEN_EMPTY_ROOT/sessions
---
# E2E: codex-rollout/pending-tail

The fixture case
[`codex-rollout/pending-tail`](../../../../crates/urollup-core/tests/fixtures/codex-rollout/pending-tail/)
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
Uncached input 1,900
Cache read     5,100
Cache write    0
Output         270
Reasoning      100
Total tokens   7,270

COVERAGE
Status complete  Copies excluded 0  Limit observations 0
Unresolved 0  Possible 0  Requests without usage 0

REQUEST SIZES (inclusive input tokens)
Count 2  p50 3,000  p90 4,000  p99 4,000  max 4,000

ACCOUNT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
unknown | 2 | 7,000 | 270 | 7,270

EFFORT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
medium | 2 | 7,000 | 270 | 7,270

MODEL BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
gpt-5.2-codex | 2 | 7,000 | 270 | 7,270

PROJECT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
project | 2 | 7,000 | 270 | 7,270

DIAGNOSTICS
malformed-line x1: a complete Codex rollout line is malformed
pending-tail x1: the incomplete final Codex rollout line is pending
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
  "diagnostics": [
    {
      "code": "malformed-line",
      "count": 1,
      "detail": "a complete Codex rollout line is malformed"
    },
    {
      "code": "pending-tail",
      "count": 1,
      "detail": "the incomplete final Codex rollout line is pending"
    }
  ],
  "totals": {
    "requests": {
      "owned": 2,
      "ambiguous": 0,
      "unknown": 0
    },
    "tokens": {
      "uncached_input": 1900,
      "cache_read": 5100,
      "cache_write": 0,
      "cache_write_unspecified": 0,
      "output": 270,
      "reasoning": 100,
      "total": 7270
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
          "uncached_input": 1900,
          "cache_read": 5100,
          "cache_write": 0,
          "cache_write_unspecified": 0,
          "output": 270,
          "reasoning": 100,
          "total": 7270
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
          "uncached_input": 1900,
          "cache_read": 5100,
          "cache_write": 0,
          "cache_write_unspecified": 0,
          "output": 270,
          "reasoning": 100,
          "total": 7270
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
          "uncached_input": 1900,
          "cache_read": 5100,
          "cache_write": 0,
          "cache_write_unspecified": 0,
          "output": 270,
          "reasoning": 100,
          "total": 7270
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
          "uncached_input": 1900,
          "cache_read": 5100,
          "cache_write": 0,
          "cache_write_unspecified": 0,
          "output": 270,
          "reasoning": 100,
          "total": 7270
        }
      }
    ]
  },
  "sizes": {
    "count": 2,
    "p50": 3000,
    "p90": 4000,
    "p99": 4000,
    "max": 4000
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
2026-09-12 | 2 | 1,900 | 5,100 | 0 | 270 | 7,270

DIAGNOSTICS
malformed-line x1: a complete Codex rollout line is malformed
pending-tail x1: the incomplete final Codex rollout line is pending
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
      "date": "2026-09-12",
      "requests": {
        "owned": 2,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 1900,
        "cache_read": 5100,
        "cache_write": 0,
        "cache_write_unspecified": 0,
        "output": 270,
        "reasoning": 100,
        "total": 7270
      }
    }
  ],
  "diagnostics": [
    {
      "code": "malformed-line",
      "count": 1,
      "detail": "a complete Codex rollout line is malformed"
    },
    {
      "code": "pending-tail",
      "count": 1,
      "detail": "the incomplete final Codex rollout line is pending"
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
thr-v1-5c3jz5d8fvjsm2zv4tb8rq28aw | codex | project | 2 | 7,000 | 270 | 7,270

DIAGNOSTICS
malformed-line x1: a complete Codex rollout line is malformed
pending-tail x1: the incomplete final Codex rollout line is pending
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
      "thread": "thr-v1-5c3jz5d8fvjsm2zv4tb8rq28aw",
      "session": "019f0000-0000-7000-8000-001400000001",
      "agent": "codex",
      "project": "project",
      "requests": {
        "owned": 2,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 1900,
        "cache_read": 5100,
        "cache_write": 0,
        "cache_write_unspecified": 0,
        "output": 270,
        "reasoning": 100,
        "total": 7270
      }
    }
  ],
  "diagnostics": [
    {
      "code": "malformed-line",
      "count": 1,
      "detail": "a complete Codex rollout line is malformed"
    },
    {
      "code": "pending-tail",
      "count": 1,
      "detail": "the incomplete final Codex rollout line is pending"
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
thr-v1-5c3jz5d8fvjsm2zv4tb8rq28aw | codex | project | 2 | 7,000 | 270 | 7,270

DIAGNOSTICS
malformed-line x1: a complete Codex rollout line is malformed
pending-tail x1: the incomplete final Codex rollout line is pending
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
      "thread": "thr-v1-5c3jz5d8fvjsm2zv4tb8rq28aw",
      "session": "019f0000-0000-7000-8000-001400000001",
      "agent": "codex",
      "project": "project",
      "requests": {
        "owned": 2,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 1900,
        "cache_read": 5100,
        "cache_write": 0,
        "cache_write_unspecified": 0,
        "output": 270,
        "reasoning": 100,
        "total": 7270
      }
    }
  ],
  "diagnostics": [
    {
      "code": "malformed-line",
      "count": 1,
      "detail": "a complete Codex rollout line is malformed"
    },
    {
      "code": "pending-tail",
      "count": 1,
      "detail": "the incomplete final Codex rollout line is pending"
    }
  ]
}
? 0
```

<!-- This document follows common-doc-guidelines.md.
See github.com/jlevy/practical-prose and review guidelines before editing.
-->
