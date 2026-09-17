---
sandbox: ../../../../crates/urollup-core/tests/fixtures/codex-rollout/legacy-subagent-prefix
path:
  - $UROLLUP_BIN
env:
  CLAUDE_CONFIG_DIR: $GOLDEN_EMPTY_ROOT
  CODEX_HOME: .
  PI_CODING_AGENT_SESSION_DIR: $GOLDEN_EMPTY_ROOT/sessions
---
# E2E: codex-rollout/legacy-subagent-prefix

The fixture case
[`codex-rollout/legacy-subagent-prefix`](../../../../crates/urollup-core/tests/fixtures/codex-rollout/legacy-subagent-prefix/)
read through `CODEX_HOME` from a sandbox copy, with every other discovery root empty and
HOME hermetic. `make e2e-results` checks its reconciled results against `expected.json`;
this session records the complete output of each view for review.

## Report

```console
$ urollup report --all --timezone UTC
urollup report
Selection all  Scope self  Timezone UTC

TOTALS
Requests  8
Owned     8
Ambiguous 0
Unknown   0
Uncached input 12,000
Cache read     28,500
Cache write    0
Output         2,500
Reasoning      820
Total tokens   43,000

COVERAGE
Status complete  Copies excluded 5  Limit observations 0
Unresolved 0  Possible 0  Requests without usage 0

REQUEST SIZES (inclusive input tokens)
Count 8  p50 3,500  p90 9,000  p99 9,000  max 9,000

ACCOUNT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
unknown | 8 | 40,500 | 2,500 | 43,000

EFFORT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
medium | 8 | 40,500 | 2,500 | 43,000

MODEL BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
gpt-5.2-codex | 8 | 40,500 | 2,500 | 43,000

PROJECT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
project | 8 | 40,500 | 2,500 | 43,000

DIAGNOSTICS
codex-copied-history-inferred x17: Codex copied-history boundary was inferred from legacy rollout records
thread-orphan x2: Codex thread names a parent whose rollout was not discovered
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
    "copies_excluded": 5,
    "limit_observations": 0,
    "requests_without_usage": 0,
    "complete": true
  },
  "diagnostics": [
    {
      "code": "codex-copied-history-inferred",
      "count": 17,
      "detail": "Codex copied-history boundary was inferred from legacy rollout records"
    },
    {
      "code": "thread-orphan",
      "count": 2,
      "detail": "Codex thread names a parent whose rollout was not discovered"
    }
  ],
  "totals": {
    "requests": {
      "owned": 8,
      "ambiguous": 0,
      "unknown": 0
    },
    "tokens": {
      "uncached_input": 12000,
      "cache_read": 28500,
      "cache_write": 0,
      "cache_write_unspecified": 0,
      "output": 2500,
      "reasoning": 820,
      "total": 43000
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
          "owned": 8,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 12000,
          "cache_read": 28500,
          "cache_write": 0,
          "cache_write_unspecified": 0,
          "output": 2500,
          "reasoning": 820,
          "total": 43000
        }
      }
    ],
    "effort": [
      {
        "group": "effort",
        "value": "medium",
        "requests": {
          "owned": 8,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 12000,
          "cache_read": 28500,
          "cache_write": 0,
          "cache_write_unspecified": 0,
          "output": 2500,
          "reasoning": 820,
          "total": 43000
        }
      }
    ],
    "model": [
      {
        "group": "model",
        "value": "gpt-5.2-codex",
        "requests": {
          "owned": 8,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 12000,
          "cache_read": 28500,
          "cache_write": 0,
          "cache_write_unspecified": 0,
          "output": 2500,
          "reasoning": 820,
          "total": 43000
        }
      }
    ],
    "project": [
      {
        "group": "project",
        "value": "project",
        "requests": {
          "owned": 8,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 12000,
          "cache_read": 28500,
          "cache_write": 0,
          "cache_write_unspecified": 0,
          "output": 2500,
          "reasoning": 820,
          "total": 43000
        }
      }
    ]
  },
  "sizes": {
    "count": 8,
    "p50": 3500,
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
2026-09-05 | 8 | 12,000 | 28,500 | 0 | 2,500 | 43,000

DIAGNOSTICS
codex-copied-history-inferred x17: Codex copied-history boundary was inferred from legacy rollout records
thread-orphan x2: Codex thread names a parent whose rollout was not discovered
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
      "date": "2026-09-05",
      "requests": {
        "owned": 8,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 12000,
        "cache_read": 28500,
        "cache_write": 0,
        "cache_write_unspecified": 0,
        "output": 2500,
        "reasoning": 820,
        "total": 43000
      }
    }
  ],
  "diagnostics": [
    {
      "code": "codex-copied-history-inferred",
      "count": 17,
      "detail": "Codex copied-history boundary was inferred from legacy rollout records"
    },
    {
      "code": "thread-orphan",
      "count": 2,
      "detail": "Codex thread names a parent whose rollout was not discovered"
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
thr-v1-0demasgtqpc81av8pqp9qk44g2 | codex | project | 2 | 6,500 | 250 | 6,750
thr-v1-0k2vgw8f5a6ad575m7j27e8vnf | codex | project | 2 | 14,500 | 550 | 15,050
thr-v1-52wfqca3118hs6n8v43yabkqmp | codex | project | 2 | 4,500 | 200 | 4,700
thr-v1-7q3rw8a20ffnp0hn5738jdpmnm | codex | project | 2 | 15,000 | 1,500 | 16,500

DIAGNOSTICS
codex-copied-history-inferred x17: Codex copied-history boundary was inferred from legacy rollout records
thread-orphan x2: Codex thread names a parent whose rollout was not discovered
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
      "thread": "thr-v1-0demasgtqpc81av8pqp9qk44g2",
      "agent": "codex",
      "project": "project",
      "requests": {
        "owned": 2,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 1000,
        "cache_read": 5500,
        "cache_write": 0,
        "cache_write_unspecified": 0,
        "output": 250,
        "reasoning": 50,
        "total": 6750
      }
    },
    {
      "thread": "thr-v1-0k2vgw8f5a6ad575m7j27e8vnf",
      "agent": "codex",
      "project": "project",
      "requests": {
        "owned": 2,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 1500,
        "cache_read": 13000,
        "cache_write": 0,
        "cache_write_unspecified": 0,
        "output": 550,
        "reasoning": 200,
        "total": 15050
      }
    },
    {
      "thread": "thr-v1-52wfqca3118hs6n8v43yabkqmp",
      "agent": "codex",
      "project": "project",
      "requests": {
        "owned": 2,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 1000,
        "cache_read": 3500,
        "cache_write": 0,
        "cache_write_unspecified": 0,
        "output": 200,
        "reasoning": 70,
        "total": 4700
      }
    },
    {
      "thread": "thr-v1-7q3rw8a20ffnp0hn5738jdpmnm",
      "agent": "codex",
      "project": "project",
      "requests": {
        "owned": 2,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 8500,
        "cache_read": 6500,
        "cache_write": 0,
        "cache_write_unspecified": 0,
        "output": 1500,
        "reasoning": 500,
        "total": 16500
      }
    }
  ],
  "diagnostics": [
    {
      "code": "codex-copied-history-inferred",
      "count": 17,
      "detail": "Codex copied-history boundary was inferred from legacy rollout records"
    },
    {
      "code": "thread-orphan",
      "count": 2,
      "detail": "Codex thread names a parent whose rollout was not discovered"
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
thr-v1-0demasgtqpc81av8pqp9qk44g2 | codex | project | 2 | 6,500 | 250 | 6,750
thr-v1-0k2vgw8f5a6ad575m7j27e8vnf | codex | project | 2 | 14,500 | 550 | 15,050
thr-v1-52wfqca3118hs6n8v43yabkqmp | codex | project | 2 | 4,500 | 200 | 4,700
thr-v1-7q3rw8a20ffnp0hn5738jdpmnm | codex | project | 2 | 15,000 | 1,500 | 16,500

DIAGNOSTICS
codex-copied-history-inferred x17: Codex copied-history boundary was inferred from legacy rollout records
thread-orphan x2: Codex thread names a parent whose rollout was not discovered
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
      "thread": "thr-v1-0demasgtqpc81av8pqp9qk44g2",
      "agent": "codex",
      "project": "project",
      "requests": {
        "owned": 2,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 1000,
        "cache_read": 5500,
        "cache_write": 0,
        "cache_write_unspecified": 0,
        "output": 250,
        "reasoning": 50,
        "total": 6750
      }
    },
    {
      "thread": "thr-v1-0k2vgw8f5a6ad575m7j27e8vnf",
      "agent": "codex",
      "project": "project",
      "requests": {
        "owned": 2,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 1500,
        "cache_read": 13000,
        "cache_write": 0,
        "cache_write_unspecified": 0,
        "output": 550,
        "reasoning": 200,
        "total": 15050
      }
    },
    {
      "thread": "thr-v1-52wfqca3118hs6n8v43yabkqmp",
      "agent": "codex",
      "project": "project",
      "requests": {
        "owned": 2,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 1000,
        "cache_read": 3500,
        "cache_write": 0,
        "cache_write_unspecified": 0,
        "output": 200,
        "reasoning": 70,
        "total": 4700
      }
    },
    {
      "thread": "thr-v1-7q3rw8a20ffnp0hn5738jdpmnm",
      "agent": "codex",
      "project": "project",
      "requests": {
        "owned": 2,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 8500,
        "cache_read": 6500,
        "cache_write": 0,
        "cache_write_unspecified": 0,
        "output": 1500,
        "reasoning": 500,
        "total": 16500
      }
    }
  ],
  "diagnostics": [
    {
      "code": "codex-copied-history-inferred",
      "count": 17,
      "detail": "Codex copied-history boundary was inferred from legacy rollout records"
    },
    {
      "code": "thread-orphan",
      "count": 2,
      "detail": "Codex thread names a parent whose rollout was not discovered"
    }
  ]
}
? 0
```

<!-- This document follows common-doc-guidelines.md.
See github.com/jlevy/practical-prose and review guidelines before editing.
-->
