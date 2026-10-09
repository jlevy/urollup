---
sandbox: ../../../../crates/urollup-core/tests/fixtures/cursor-state/basic
path:
  - $UROLLUP_BIN
env:
  CLAUDE_CONFIG_DIR: $GOLDEN_EMPTY_ROOT
  CODEX_HOME: $GOLDEN_EMPTY_ROOT
  PI_CODING_AGENT_SESSION_DIR: $GOLDEN_EMPTY_ROOT/sessions
  UROLLUP_CURSOR_DIRS: .
  CURSOR_CONVERSATION_ID: 11111111-1111-4111-8111-111111111111
---
# E2E: cursor-state/basic

The fixture case
[`cursor-state/basic`](../../../../crates/urollup-core/tests/fixtures/cursor-state/basic/)
read through `UROLLUP_CURSOR_DIRS` from a sandbox copy, with every other discovery root
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
Uncached input 30
Cache read     -
Cache write    -
Output         8
Reasoning      -
Total tokens   38

COVERAGE
Status complete  Copies excluded 0  Limit observations 0
Unresolved 0  Possible 0  Requests without usage 0

REQUEST SIZES (inclusive input tokens)
Count 2  p50 10  p90 20  p99 20  max 20

ACCOUNT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
unknown | 2 | 30 | 8 | 38

EFFORT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
style-2 | 1 | 20 | 5 | 25
unknown | 1 | 10 | 3 | 13

MODEL BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
claude-4.5-opus-high-thinking | 1 | 20 | 5 | 25
cursor-grok-4.6-xhigh-fast | 1 | 10 | 3 | 13

PROJECT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
unknown | 2 | 30 | 8 | 38

DIAGNOSTICS
cursor-estimate-cost x1: Cursor reported a session cost estimate
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
      "code": "cursor-estimate-cost",
      "count": 1,
      "detail": "Cursor reported a session cost estimate"
    }
  ],
  "totals": {
    "requests": {
      "owned": 2,
      "ambiguous": 0,
      "unknown": 0
    },
    "tokens": {
      "uncached_input": 30,
      "output": 8,
      "total": 38
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
          "uncached_input": 30,
          "output": 8,
          "total": 38
        }
      }
    ],
    "effort": [
      {
        "group": "effort",
        "value": "style-2",
        "requests": {
          "owned": 1,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 20,
          "output": 5,
          "total": 25
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
          "uncached_input": 10,
          "output": 3,
          "total": 13
        }
      }
    ],
    "model": [
      {
        "group": "model",
        "value": "claude-4.5-opus-high-thinking",
        "requests": {
          "owned": 1,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 20,
          "output": 5,
          "total": 25
        }
      },
      {
        "group": "model",
        "value": "cursor-grok-4.6-xhigh-fast",
        "requests": {
          "owned": 1,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 10,
          "output": 3,
          "total": 13
        }
      }
    ],
    "project": [
      {
        "group": "project",
        "value": "unknown",
        "requests": {
          "owned": 2,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 30,
          "output": 8,
          "total": 38
        }
      }
    ]
  },
  "sizes": {
    "count": 2,
    "p50": 10,
    "p90": 20,
    "p99": 20,
    "max": 20
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
2024-09-18 | 2 | 30 | - | - | 8 | 38

DIAGNOSTICS
cursor-estimate-cost x1: Cursor reported a session cost estimate
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
      "date": "2024-09-18",
      "requests": {
        "owned": 2,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 30,
        "output": 8,
        "total": 38
      }
    }
  ],
  "diagnostics": [
    {
      "code": "cursor-estimate-cost",
      "count": 1,
      "detail": "Cursor reported a session cost estimate"
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
thr-v1-28bscd1h6qcyy03gnmj5vhx8tg | cursor | unknown | 1 | 20 | 5 | 25
thr-v1-2dfpbnpasb0xdch1kgkm0tswfw | cursor | unknown | 1 | 10 | 3 | 13
thr-v1-2r5gncw638mdv11ptxq05nq9vg | cursor | unknown | 0 | - | - | -

DIAGNOSTICS
cursor-estimate-cost x1: Cursor reported a session cost estimate
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
      "thread": "thr-v1-28bscd1h6qcyy03gnmj5vhx8tg",
      "session": "11111111-1111-4111-8111-111111111111",
      "agent": "cursor",
      "requests": {
        "owned": 1,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 20,
        "output": 5,
        "total": 25
      }
    },
    {
      "thread": "thr-v1-2dfpbnpasb0xdch1kgkm0tswfw",
      "session": "22222222-2222-4222-8222-222222222222",
      "agent": "cursor",
      "requests": {
        "owned": 1,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 10,
        "output": 3,
        "total": 13
      }
    },
    {
      "thread": "thr-v1-2r5gncw638mdv11ptxq05nq9vg",
      "session": "33333333-3333-4333-8333-333333333333",
      "agent": "cursor",
      "requests": {
        "owned": 0,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {}
    }
  ],
  "diagnostics": [
    {
      "code": "cursor-estimate-cost",
      "count": 1,
      "detail": "Cursor reported a session cost estimate"
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
thr-v1-28bscd1h6qcyy03gnmj5vhx8tg | cursor | unknown | 1 | 20 | 5 | 25
thr-v1-2dfpbnpasb0xdch1kgkm0tswfw | cursor | unknown | 1 | 10 | 3 | 13
thr-v1-2r5gncw638mdv11ptxq05nq9vg | cursor | unknown | 0 | - | - | -

DIAGNOSTICS
cursor-estimate-cost x1: Cursor reported a session cost estimate
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
      "thread": "thr-v1-28bscd1h6qcyy03gnmj5vhx8tg",
      "session": "11111111-1111-4111-8111-111111111111",
      "agent": "cursor",
      "requests": {
        "owned": 1,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 20,
        "output": 5,
        "total": 25
      }
    },
    {
      "thread": "thr-v1-2dfpbnpasb0xdch1kgkm0tswfw",
      "session": "22222222-2222-4222-8222-222222222222",
      "agent": "cursor",
      "requests": {
        "owned": 1,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 10,
        "output": 3,
        "total": 13
      }
    },
    {
      "thread": "thr-v1-2r5gncw638mdv11ptxq05nq9vg",
      "session": "33333333-3333-4333-8333-333333333333",
      "agent": "cursor",
      "requests": {
        "owned": 0,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {}
    }
  ],
  "diagnostics": [
    {
      "code": "cursor-estimate-cost",
      "count": 1,
      "detail": "Cursor reported a session cost estimate"
    }
  ]
}
? 0
```

## Report by provider, agent, model, effort and purpose

```console
$ urollup report --all --timezone UTC --group-by provider,agent,model,effort,purpose
urollup report
Selection all  Scope self  Timezone UTC

TOTALS
Requests  2
Owned     2
Ambiguous 0
Unknown   0
Uncached input 30
Cache read     -
Cache write    -
Output         8
Reasoning      -
Total tokens   38

COVERAGE
Status complete  Copies excluded 0  Limit observations 0
Unresolved 0  Possible 0  Requests without usage 0

REQUEST SIZES (inclusive input tokens)
Count 2  p50 10  p90 20  p99 20  max 20

AGENT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
cursor | 2 | 30 | 8 | 38

EFFORT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
style-2 | 1 | 20 | 5 | 25
unknown | 1 | 10 | 3 | 13

MODEL BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
claude-4.5-opus-high-thinking | 1 | 20 | 5 | 25
cursor-grok-4.6-xhigh-fast | 1 | 10 | 3 | 13

PROVIDER BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
anthropic | 1 | 20 | 5 | 25
cursor | 1 | 10 | 3 | 13

PURPOSE BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
agent | 2 | 30 | 8 | 38

DIAGNOSTICS
cursor-estimate-cost x1: Cursor reported a session cost estimate
? 0
```

## Report JSON by provider, agent, model, effort and purpose

```console
$ urollup report --all --format json --timezone UTC --group-by provider,agent,model,effort,purpose
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
      "code": "cursor-estimate-cost",
      "count": 1,
      "detail": "Cursor reported a session cost estimate"
    }
  ],
  "totals": {
    "requests": {
      "owned": 2,
      "ambiguous": 0,
      "unknown": 0
    },
    "tokens": {
      "uncached_input": 30,
      "output": 8,
      "total": 38
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
    "agent": [
      {
        "group": "agent",
        "value": "cursor",
        "requests": {
          "owned": 2,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 30,
          "output": 8,
          "total": 38
        }
      }
    ],
    "effort": [
      {
        "group": "effort",
        "value": "style-2",
        "requests": {
          "owned": 1,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 20,
          "output": 5,
          "total": 25
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
          "uncached_input": 10,
          "output": 3,
          "total": 13
        }
      }
    ],
    "model": [
      {
        "group": "model",
        "value": "claude-4.5-opus-high-thinking",
        "requests": {
          "owned": 1,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 20,
          "output": 5,
          "total": 25
        }
      },
      {
        "group": "model",
        "value": "cursor-grok-4.6-xhigh-fast",
        "requests": {
          "owned": 1,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 10,
          "output": 3,
          "total": 13
        }
      }
    ],
    "provider": [
      {
        "group": "provider",
        "value": "anthropic",
        "requests": {
          "owned": 1,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 20,
          "output": 5,
          "total": 25
        }
      },
      {
        "group": "provider",
        "value": "cursor",
        "requests": {
          "owned": 1,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 10,
          "output": 3,
          "total": 13
        }
      }
    ],
    "purpose": [
      {
        "group": "purpose",
        "value": "agent",
        "requests": {
          "owned": 2,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 30,
          "output": 8,
          "total": 38
        }
      }
    ]
  },
  "sizes": {
    "count": 2,
    "p50": 10,
    "p90": 20,
    "p99": 20,
    "max": 20
  }
}
? 0
```

## Current session

`CURSOR_CONVERSATION_ID` is the fixture composer with Opus tokens.
`--agent cursor` is the same disambiguation used when Claude and Cursor signals are both
present.

```console
$ urollup report --current --agent cursor --timezone UTC
urollup report
Selection current  Scope descendants  Timezone UTC

TOTALS
Requests  1
Owned     1
Ambiguous 0
Unknown   0
Uncached input 20
Cache read     -
Cache write    -
Output         5
Reasoning      -
Total tokens   25

COVERAGE
Status complete  Copies excluded 0  Limit observations 0
Unresolved 0  Possible 0  Requests without usage 0

REQUEST SIZES (inclusive input tokens)
Count 1  p50 20  p90 20  p99 20  max 20

ACCOUNT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
unknown | 1 | 20 | 5 | 25

EFFORT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
style-2 | 1 | 20 | 5 | 25

MODEL BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
claude-4.5-opus-high-thinking | 1 | 20 | 5 | 25

PROJECT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
unknown | 1 | 20 | 5 | 25
? 0
```

## Exact session

```console
$ urollup report --session 11111111-1111-4111-8111-111111111111 --timezone UTC
urollup report
Selection session  Scope descendants  Timezone UTC

TOTALS
Requests  1
Owned     1
Ambiguous 0
Unknown   0
Uncached input 20
Cache read     -
Cache write    -
Output         5
Reasoning      -
Total tokens   25

COVERAGE
Status complete  Copies excluded 0  Limit observations 0
Unresolved 0  Possible 0  Requests without usage 0

REQUEST SIZES (inclusive input tokens)
Count 1  p50 20  p90 20  p99 20  max 20

ACCOUNT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
unknown | 1 | 20 | 5 | 25

EFFORT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
style-2 | 1 | 20 | 5 | 25

MODEL BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
claude-4.5-opus-high-thinking | 1 | 20 | 5 | 25

PROJECT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
unknown | 1 | 20 | 5 | 25
? 0
```

A Cursor JSONL transcript path is a selector for that `composerId`, not a usage source.
The shared store file is not a session.

```console
$ urollup report --session agent-transcripts/11111111-1111-4111-8111-111111111111/11111111-1111-4111-8111-111111111111.jsonl --timezone UTC
urollup report
Selection session  Scope descendants  Timezone UTC

TOTALS
Requests  1
Owned     1
Ambiguous 0
Unknown   0
Uncached input 20
Cache read     -
Cache write    -
Output         5
Reasoning      -
Total tokens   25

COVERAGE
Status complete  Copies excluded 0  Limit observations 0
Unresolved 0  Possible 0  Requests without usage 0

REQUEST SIZES (inclusive input tokens)
Count 1  p50 20  p90 20  p99 20  max 20

ACCOUNT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
unknown | 1 | 20 | 5 | 25

EFFORT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
style-2 | 1 | 20 | 5 | 25

MODEL BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
claude-4.5-opus-high-thinking | 1 | 20 | 5 | 25

PROJECT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
unknown | 1 | 20 | 5 | 25
? 0
```

```console
$ urollup report --session cursor-state.json --timezone UTC
! error: session selector "cursor-state.json" did not match any discovered session
? 1
```

## JSONL is not a usage source

`--source` on the JSONL transcript is rejected.
The snapshot remains the usage owner.
A snapshot plus that transcript is still rejected, so the two stores cannot be counted
twice.

```console
$ urollup report --source agent-transcripts/11111111-1111-4111-8111-111111111111/11111111-1111-4111-8111-111111111111.jsonl --no-default-sources --timezone UTC
! error: source agent-transcripts/11111111-1111-4111-8111-111111111111/11111111-1111-4111-8111-111111111111.jsonl is a Cursor JSONL transcript, which is not a usage owner; pass state.vscdb or a cursor-state fixture
? 1
```

```console
$ urollup report --source cursor-state.json --source agent-transcripts/11111111-1111-4111-8111-111111111111/11111111-1111-4111-8111-111111111111.jsonl --no-default-sources --timezone UTC
! error: source agent-transcripts/11111111-1111-4111-8111-111111111111/11111111-1111-4111-8111-111111111111.jsonl is a Cursor JSONL transcript, which is not a usage owner; pass state.vscdb or a cursor-state fixture
? 1
```

<!-- This document follows common-doc-guidelines.md.
See github.com/jlevy/practical-prose and review guidelines before editing.
-->
