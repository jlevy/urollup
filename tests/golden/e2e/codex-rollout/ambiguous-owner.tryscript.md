---
sandbox: ../../../../crates/urollup-core/tests/fixtures/codex-rollout/ambiguous-owner
path:
  - $UROLLUP_BIN
env:
  CLAUDE_CONFIG_DIR: $GOLDEN_EMPTY_ROOT
  CODEX_HOME: .
  PI_CODING_AGENT_SESSION_DIR: $GOLDEN_EMPTY_ROOT/sessions
---
# E2E: codex-rollout/ambiguous-owner

The fixture case
[`codex-rollout/ambiguous-owner`](../../../../crates/urollup-core/tests/fixtures/codex-rollout/ambiguous-owner/)
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
Owned     2
Ambiguous 1
Unknown   0
Uncached input 6,000
Cache read     19,000
Cache write    0
Output         1,300
Reasoning      300
Total tokens   26,300

COVERAGE
Status complete  Copies excluded 0  Limit observations 0
Unresolved 0  Possible 0  Requests without usage 0

REQUEST SIZES (inclusive input tokens)
Count 3  p50 8,000  p90 12,000  p99 12,000  max 12,000

ACCOUNT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
unknown | 3 | 25,000 | 1,300 | 26,300

EFFORT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
high | 3 | 25,000 | 1,300 | 26,300

MODEL BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
gpt-5.2-codex | 3 | 25,000 | 1,300 | 26,300

PROJECT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
project | 3 | 25,000 | 1,300 | 26,300

DIAGNOSTICS
conflicting-owners x2: proven owners thr-v1-262jyyetjxsmg2wc05d3txhh68, thr-v1-4bs081r5c7z8t920qtmrps06f0
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
      "detail": "proven owners thr-v1-262jyyetjxsmg2wc05d3txhh68, thr-v1-4bs081r5c7z8t920qtmrps06f0"
    }
  ],
  "totals": {
    "requests": {
      "owned": 2,
      "ambiguous": 1,
      "unknown": 0
    },
    "tokens": {
      "uncached_input": 6000,
      "cache_read": 19000,
      "cache_write": 0,
      "cache_write_unspecified": 0,
      "output": 1300,
      "reasoning": 300,
      "total": 26300
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
          "uncached_input": 6000,
          "cache_read": 19000,
          "cache_write": 0,
          "cache_write_unspecified": 0,
          "output": 1300,
          "reasoning": 300,
          "total": 26300
        }
      }
    ],
    "effort": [
      {
        "group": "effort",
        "value": "high",
        "requests": {
          "owned": 2,
          "ambiguous": 1,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 6000,
          "cache_read": 19000,
          "cache_write": 0,
          "cache_write_unspecified": 0,
          "output": 1300,
          "reasoning": 300,
          "total": 26300
        }
      }
    ],
    "model": [
      {
        "group": "model",
        "value": "gpt-5.2-codex",
        "requests": {
          "owned": 2,
          "ambiguous": 1,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 6000,
          "cache_read": 19000,
          "cache_write": 0,
          "cache_write_unspecified": 0,
          "output": 1300,
          "reasoning": 300,
          "total": 26300
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
          "uncached_input": 6000,
          "cache_read": 19000,
          "cache_write": 0,
          "cache_write_unspecified": 0,
          "output": 1300,
          "reasoning": 300,
          "total": 26300
        }
      }
    ]
  },
  "sizes": {
    "count": 3,
    "p50": 8000,
    "p90": 12000,
    "p99": 12000,
    "max": 12000
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
2026-09-05 | 3 | 6,000 | 19,000 | 0 | 1,300 | 26,300

DIAGNOSTICS
conflicting-owners x2: proven owners thr-v1-262jyyetjxsmg2wc05d3txhh68, thr-v1-4bs081r5c7z8t920qtmrps06f0
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
        "uncached_input": 6000,
        "cache_read": 19000,
        "cache_write": 0,
        "cache_write_unspecified": 0,
        "output": 1300,
        "reasoning": 300,
        "total": 26300
      }
    }
  ],
  "diagnostics": [
    {
      "code": "conflicting-owners",
      "count": 2,
      "detail": "proven owners thr-v1-262jyyetjxsmg2wc05d3txhh68, thr-v1-4bs081r5c7z8t920qtmrps06f0"
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
unowned | codex | unknown | 1 | 12,000 | 600 | 12,600
thr-v1-262jyyetjxsmg2wc05d3txhh68 | codex | project | 1 | 8,000 | 400 | 8,400
thr-v1-4bs081r5c7z8t920qtmrps06f0 | codex | project | 1 | 5,000 | 300 | 5,300

DIAGNOSTICS
conflicting-owners x2: proven owners thr-v1-262jyyetjxsmg2wc05d3txhh68, thr-v1-4bs081r5c7z8t920qtmrps06f0
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
      "agent": "codex",
      "requests": {
        "owned": 0,
        "ambiguous": 1,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 3000,
        "cache_read": 9000,
        "cache_write": 0,
        "cache_write_unspecified": 0,
        "output": 600,
        "reasoning": 200,
        "total": 12600
      },
      "last_date": "2026-09-05",
      "undated_requests": 0
    },
    {
      "thread": "thr-v1-262jyyetjxsmg2wc05d3txhh68",
      "session": "019f0000-0000-7000-8000-001700000001",
      "agent": "codex",
      "project": "project",
      "requests": {
        "owned": 1,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 2000,
        "cache_read": 6000,
        "cache_write": 0,
        "cache_write_unspecified": 0,
        "output": 400,
        "reasoning": 100,
        "total": 8400
      },
      "last_date": "2026-09-05",
      "undated_requests": 0
    },
    {
      "thread": "thr-v1-4bs081r5c7z8t920qtmrps06f0",
      "session": "019f0000-0000-7000-8000-001700000002",
      "agent": "codex",
      "project": "project",
      "requests": {
        "owned": 1,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 1000,
        "cache_read": 4000,
        "cache_write": 0,
        "cache_write_unspecified": 0,
        "output": 300,
        "reasoning": 0,
        "total": 5300
      },
      "last_date": "2026-09-05",
      "undated_requests": 0
    }
  ],
  "diagnostics": [
    {
      "code": "conflicting-owners",
      "count": 2,
      "detail": "proven owners thr-v1-262jyyetjxsmg2wc05d3txhh68, thr-v1-4bs081r5c7z8t920qtmrps06f0"
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
unowned | codex | unknown | 1 | 12,000 | 600 | 12,600
thr-v1-262jyyetjxsmg2wc05d3txhh68 | codex | project | 1 | 8,000 | 400 | 8,400
thr-v1-4bs081r5c7z8t920qtmrps06f0 | codex | project | 1 | 5,000 | 300 | 5,300

DIAGNOSTICS
conflicting-owners x2: proven owners thr-v1-262jyyetjxsmg2wc05d3txhh68, thr-v1-4bs081r5c7z8t920qtmrps06f0
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
      "agent": "codex",
      "requests": {
        "owned": 0,
        "ambiguous": 1,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 3000,
        "cache_read": 9000,
        "cache_write": 0,
        "cache_write_unspecified": 0,
        "output": 600,
        "reasoning": 200,
        "total": 12600
      },
      "last_date": "2026-09-05",
      "undated_requests": 0
    },
    {
      "thread": "thr-v1-262jyyetjxsmg2wc05d3txhh68",
      "session": "019f0000-0000-7000-8000-001700000001",
      "agent": "codex",
      "project": "project",
      "requests": {
        "owned": 1,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 2000,
        "cache_read": 6000,
        "cache_write": 0,
        "cache_write_unspecified": 0,
        "output": 400,
        "reasoning": 100,
        "total": 8400
      },
      "last_date": "2026-09-05",
      "undated_requests": 0
    },
    {
      "thread": "thr-v1-4bs081r5c7z8t920qtmrps06f0",
      "session": "019f0000-0000-7000-8000-001700000002",
      "agent": "codex",
      "project": "project",
      "requests": {
        "owned": 1,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 1000,
        "cache_read": 4000,
        "cache_write": 0,
        "cache_write_unspecified": 0,
        "output": 300,
        "reasoning": 0,
        "total": 5300
      },
      "last_date": "2026-09-05",
      "undated_requests": 0
    }
  ],
  "diagnostics": [
    {
      "code": "conflicting-owners",
      "count": 2,
      "detail": "proven owners thr-v1-262jyyetjxsmg2wc05d3txhh68, thr-v1-4bs081r5c7z8t920qtmrps06f0"
    }
  ]
}
? 0
```

<!-- This document follows common-doc-guidelines.md.
See github.com/jlevy/practical-prose and review guidelines before editing.
-->
