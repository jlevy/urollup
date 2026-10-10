---
sandbox: ../../../../crates/urollup-core/tests/fixtures/claude-project/ambiguous-owner
path:
  - $UROLLUP_BIN
env:
  CLAUDE_CONFIG_DIR: .
  CODEX_HOME: $GOLDEN_EMPTY_ROOT
  PI_CODING_AGENT_SESSION_DIR: $GOLDEN_EMPTY_ROOT/sessions
---
# E2E: claude-project/ambiguous-owner

The fixture case
[`claude-project/ambiguous-owner`](../../../../crates/urollup-core/tests/fixtures/claude-project/ambiguous-owner/)
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
Requests  3
Owned     2
Ambiguous 1
Unknown   0
Uncached input 65
Cache read     3,500
Cache write    300
Output         170
Reasoning      -
Total tokens   4,035

COVERAGE
Status complete  Copies excluded 0  Limit observations 0
Unresolved 0  Possible 0  Requests without usage 0

REQUEST SIZES (inclusive input tokens)
Count 3  p50 1,120  p90 2,230  p99 2,230  max 2,230

ACCOUNT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
unknown | 3 | 3,865 | 170 | 4,035

EFFORT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
unknown | 3 | 3,865 | 170 | 4,035

MODEL BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
claude-sonnet-4-5 | 3 | 3,865 | 170 | 4,035

PROJECT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
project | 3 | 3,865 | 170 | 4,035

DIAGNOSTICS
conflicting-owners x2: proven owners thr-v1-1ctftnp1tgm3qwb85vbdshjahb, thr-v1-552thqtbre5b2fbhx3xrwcvz1e
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
      "code": "conflicting-owners",
      "count": 2,
      "detail": "proven owners thr-v1-1ctftnp1tgm3qwb85vbdshjahb, thr-v1-552thqtbre5b2fbhx3xrwcvz1e"
    }
  ],
  "totals": {
    "requests": {
      "owned": 2,
      "ambiguous": 1,
      "unknown": 0
    },
    "tokens": {
      "uncached_input": 65,
      "cache_read": 3500,
      "cache_write": 300,
      "cache_write_5m": 300,
      "cache_write_1h": 0,
      "output": 170,
      "total": 4035
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
          "ambiguous": 1,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 65,
          "cache_read": 3500,
          "cache_write": 300,
          "cache_write_5m": 300,
          "cache_write_1h": 0,
          "output": 170,
          "total": 4035
        }
      }
    ],
    "effort": [
      {
        "group": "effort",
        "value": "unknown",
        "requests": {
          "owned": 2,
          "ambiguous": 1,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 65,
          "cache_read": 3500,
          "cache_write": 300,
          "cache_write_5m": 300,
          "cache_write_1h": 0,
          "output": 170,
          "total": 4035
        }
      }
    ],
    "model": [
      {
        "group": "model",
        "value": "claude-sonnet-4-5",
        "requests": {
          "owned": 2,
          "ambiguous": 1,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 65,
          "cache_read": 3500,
          "cache_write": 300,
          "cache_write_5m": 300,
          "cache_write_1h": 0,
          "output": 170,
          "total": 4035
        }
      }
    ],
    "project": [
      {
        "group": "project",
        "value": "project",
        "requests": {
          "owned": 2,
          "ambiguous": 1,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 65,
          "cache_read": 3500,
          "cache_write": 300,
          "cache_write_5m": 300,
          "cache_write_1h": 0,
          "output": 170,
          "total": 4035
        }
      }
    ]
  },
  "sizes": {
    "count": 3,
    "p50": 1120,
    "p90": 2230,
    "p99": 2230,
    "max": 2230
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
2026-09-05 | 3 | 65 | 3,500 | 300 | 170 | 4,035

DIAGNOSTICS
conflicting-owners x2: proven owners thr-v1-1ctftnp1tgm3qwb85vbdshjahb, thr-v1-552thqtbre5b2fbhx3xrwcvz1e
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
        "owned": 2,
        "ambiguous": 1,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 65,
        "cache_read": 3500,
        "cache_write": 300,
        "cache_write_5m": 300,
        "cache_write_1h": 0,
        "output": 170,
        "total": 4035
      }
    }
  ],
  "diagnostics": [
    {
      "code": "conflicting-owners",
      "count": 2,
      "detail": "proven owners thr-v1-1ctftnp1tgm3qwb85vbdshjahb, thr-v1-552thqtbre5b2fbhx3xrwcvz1e"
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
unowned | claude | - | 1 | 2,230 | 80 | 2,310
thr-v1-1ctftnp1tgm3qwb85vbdshjahb | claude | project | 1 | 515 | 40 | 555
thr-v1-552thqtbre5b2fbhx3xrwcvz1e | claude | project | 1 | 1,120 | 50 | 1,170

DIAGNOSTICS
conflicting-owners x2: proven owners thr-v1-1ctftnp1tgm3qwb85vbdshjahb, thr-v1-552thqtbre5b2fbhx3xrwcvz1e
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
      "thread": null,
      "session": null,
      "agent": "claude",
      "requests": {
        "owned": 0,
        "ambiguous": 1,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 30,
        "cache_read": 2000,
        "cache_write": 200,
        "cache_write_5m": 200,
        "cache_write_1h": 0,
        "output": 80,
        "total": 2310
      },
      "last_date": "2026-09-05",
      "undated_requests": 0
    },
    {
      "thread": "thr-v1-1ctftnp1tgm3qwb85vbdshjahb",
      "session": "00000000-0000-4000-8000-001700000002",
      "agent": "claude",
      "project": "project",
      "requests": {
        "owned": 1,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 15,
        "cache_read": 500,
        "cache_write": 0,
        "cache_write_5m": 0,
        "cache_write_1h": 0,
        "output": 40,
        "total": 555
      },
      "last_date": "2026-09-05",
      "undated_requests": 0
    },
    {
      "thread": "thr-v1-552thqtbre5b2fbhx3xrwcvz1e",
      "session": "00000000-0000-4000-8000-001700000001",
      "agent": "claude",
      "project": "project",
      "requests": {
        "owned": 1,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 20,
        "cache_read": 1000,
        "cache_write": 100,
        "cache_write_5m": 100,
        "cache_write_1h": 0,
        "output": 50,
        "total": 1170
      },
      "last_date": "2026-09-05",
      "undated_requests": 0
    }
  ],
  "diagnostics": [
    {
      "code": "conflicting-owners",
      "count": 2,
      "detail": "proven owners thr-v1-1ctftnp1tgm3qwb85vbdshjahb, thr-v1-552thqtbre5b2fbhx3xrwcvz1e"
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
unowned | claude | - | 1 | 2,230 | 80 | 2,310
thr-v1-1ctftnp1tgm3qwb85vbdshjahb | claude | project | 1 | 515 | 40 | 555
thr-v1-552thqtbre5b2fbhx3xrwcvz1e | claude | project | 1 | 1,120 | 50 | 1,170

DIAGNOSTICS
conflicting-owners x2: proven owners thr-v1-1ctftnp1tgm3qwb85vbdshjahb, thr-v1-552thqtbre5b2fbhx3xrwcvz1e
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
      "thread": null,
      "session": null,
      "agent": "claude",
      "requests": {
        "owned": 0,
        "ambiguous": 1,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 30,
        "cache_read": 2000,
        "cache_write": 200,
        "cache_write_5m": 200,
        "cache_write_1h": 0,
        "output": 80,
        "total": 2310
      },
      "last_date": "2026-09-05",
      "undated_requests": 0
    },
    {
      "thread": "thr-v1-1ctftnp1tgm3qwb85vbdshjahb",
      "session": "00000000-0000-4000-8000-001700000002",
      "agent": "claude",
      "project": "project",
      "requests": {
        "owned": 1,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 15,
        "cache_read": 500,
        "cache_write": 0,
        "cache_write_5m": 0,
        "cache_write_1h": 0,
        "output": 40,
        "total": 555
      },
      "last_date": "2026-09-05",
      "undated_requests": 0
    },
    {
      "thread": "thr-v1-552thqtbre5b2fbhx3xrwcvz1e",
      "session": "00000000-0000-4000-8000-001700000001",
      "agent": "claude",
      "project": "project",
      "requests": {
        "owned": 1,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 20,
        "cache_read": 1000,
        "cache_write": 100,
        "cache_write_5m": 100,
        "cache_write_1h": 0,
        "output": 50,
        "total": 1170
      },
      "last_date": "2026-09-05",
      "undated_requests": 0
    }
  ],
  "diagnostics": [
    {
      "code": "conflicting-owners",
      "count": 2,
      "detail": "proven owners thr-v1-1ctftnp1tgm3qwb85vbdshjahb, thr-v1-552thqtbre5b2fbhx3xrwcvz1e"
    }
  ]
}
? 0
```

<!-- This document follows common-doc-guidelines.md.
See github.com/jlevy/practical-prose and review guidelines before editing.
-->
