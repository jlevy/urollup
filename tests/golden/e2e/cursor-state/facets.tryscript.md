---
sandbox: ../../../../crates/urollup-core/tests/fixtures/cursor-state/facets
path:
  - $UROLLUP_BIN
env:
  CLAUDE_CONFIG_DIR: $GOLDEN_EMPTY_ROOT
  CODEX_HOME: $GOLDEN_EMPTY_ROOT
  PI_CODING_AGENT_SESSION_DIR: $GOLDEN_EMPTY_ROOT/sessions
  UROLLUP_CURSOR_DIRS: .
---
# E2E: cursor-state/facets

The fixture case
[`cursor-state/facets`](../../../../crates/urollup-core/tests/fixtures/cursor-state/facets/)
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
Requests  5
Owned     5
Ambiguous 0
Unknown   0
Uncached input 22
Cache read     -
Cache write    -
Output         6
Reasoning      -
Total tokens   28

COVERAGE
Status complete  Copies excluded 0  Limit observations 0
Unresolved 0  Possible 0  Requests without usage 0

REQUEST SIZES (inclusive input tokens)
Count 5  p50 4  p90 8  p99 8  max 8

ACCOUNT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
unknown | 5 | 22 | 6 | 28

EFFORT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
style-1 | 1 | 5 | 1 | 6
style-2 | 1 | 8 | 2 | 10
unknown | 3 | 9 | 3 | 12

MODEL BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
claude-4.6-opus-high-thinking | 1 | 8 | 2 | 10
composer-2.5-fast | 1 | 3 | 1 | 4
gemini-3-pro | 1 | 5 | 1 | 6
gpt-5.1-codex-high | 1 | 4 | 1 | 5
kimi-k2-instruct | 1 | 2 | 1 | 3

PROJECT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
unknown | 5 | 22 | 6 | 28
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
      "uncached_input": 22,
      "output": 6,
      "total": 28
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
          "uncached_input": 22,
          "output": 6,
          "total": 28
        }
      }
    ],
    "effort": [
      {
        "group": "effort",
        "value": "style-1",
        "requests": {
          "owned": 1,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 5,
          "output": 1,
          "total": 6
        }
      },
      {
        "group": "effort",
        "value": "style-2",
        "requests": {
          "owned": 1,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 8,
          "output": 2,
          "total": 10
        }
      },
      {
        "group": "effort",
        "value": "unknown",
        "requests": {
          "owned": 3,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 9,
          "output": 3,
          "total": 12
        }
      }
    ],
    "model": [
      {
        "group": "model",
        "value": "claude-4.6-opus-high-thinking",
        "requests": {
          "owned": 1,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 8,
          "output": 2,
          "total": 10
        }
      },
      {
        "group": "model",
        "value": "composer-2.5-fast",
        "requests": {
          "owned": 1,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 3,
          "output": 1,
          "total": 4
        }
      },
      {
        "group": "model",
        "value": "gemini-3-pro",
        "requests": {
          "owned": 1,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 5,
          "output": 1,
          "total": 6
        }
      },
      {
        "group": "model",
        "value": "gpt-5.1-codex-high",
        "requests": {
          "owned": 1,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 4,
          "output": 1,
          "total": 5
        }
      },
      {
        "group": "model",
        "value": "kimi-k2-instruct",
        "requests": {
          "owned": 1,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 2,
          "output": 1,
          "total": 3
        }
      }
    ],
    "project": [
      {
        "group": "project",
        "value": "unknown",
        "requests": {
          "owned": 5,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 22,
          "output": 6,
          "total": 28
        }
      }
    ]
  },
  "sizes": {
    "count": 5,
    "p50": 4,
    "p90": 8,
    "p99": 8,
    "max": 8
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
2024-09-18 | 1 | 8 | - | - | 2 | 10
2024-09-19 | 4 | 14 | - | - | 4 | 18
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
        "owned": 1,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 8,
        "output": 2,
        "total": 10
      }
    },
    {
      "date": "2024-09-19",
      "requests": {
        "owned": 4,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 14,
        "output": 4,
        "total": 18
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
thr-v1-12xs050v8ftc90fbrs07p3drgy | cursor | unknown | 1 | 3 | 1 | 4
thr-v1-172efbmpfmvk15kjws2nmc0h6j | cursor | unknown | 1 | 2 | 1 | 3
thr-v1-3tt2mxsat8ppq7tatwh9fk5jw7 | cursor | unknown | 1 | 4 | 1 | 5
thr-v1-41r9mkshyfn5c56jk2e6t0xrk1 | cursor | unknown | 1 | 5 | 1 | 6
thr-v1-447jdx7db7b6ne30w60pgfsffx | cursor | unknown | 1 | 8 | 2 | 10
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
      "thread": "thr-v1-12xs050v8ftc90fbrs07p3drgy",
      "session": "88888888-8888-4888-8888-888888888888",
      "agent": "cursor",
      "requests": {
        "owned": 1,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 3,
        "output": 1,
        "total": 4
      }
    },
    {
      "thread": "thr-v1-172efbmpfmvk15kjws2nmc0h6j",
      "session": "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa",
      "agent": "cursor",
      "requests": {
        "owned": 1,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 2,
        "output": 1,
        "total": 3
      }
    },
    {
      "thread": "thr-v1-3tt2mxsat8ppq7tatwh9fk5jw7",
      "session": "77777777-7777-4777-8777-777777777777",
      "agent": "cursor",
      "requests": {
        "owned": 1,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 4,
        "output": 1,
        "total": 5
      }
    },
    {
      "thread": "thr-v1-41r9mkshyfn5c56jk2e6t0xrk1",
      "session": "66666666-6666-4666-8666-666666666666",
      "agent": "cursor",
      "requests": {
        "owned": 1,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 5,
        "output": 1,
        "total": 6
      }
    },
    {
      "thread": "thr-v1-447jdx7db7b6ne30w60pgfsffx",
      "session": "55555555-5555-4555-8555-555555555555",
      "agent": "cursor",
      "requests": {
        "owned": 1,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 8,
        "output": 2,
        "total": 10
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
thr-v1-12xs050v8ftc90fbrs07p3drgy | cursor | unknown | 1 | 3 | 1 | 4
thr-v1-172efbmpfmvk15kjws2nmc0h6j | cursor | unknown | 1 | 2 | 1 | 3
thr-v1-3tt2mxsat8ppq7tatwh9fk5jw7 | cursor | unknown | 1 | 4 | 1 | 5
thr-v1-41r9mkshyfn5c56jk2e6t0xrk1 | cursor | unknown | 1 | 5 | 1 | 6
thr-v1-447jdx7db7b6ne30w60pgfsffx | cursor | unknown | 1 | 8 | 2 | 10
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
      "thread": "thr-v1-12xs050v8ftc90fbrs07p3drgy",
      "session": "88888888-8888-4888-8888-888888888888",
      "agent": "cursor",
      "requests": {
        "owned": 1,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 3,
        "output": 1,
        "total": 4
      }
    },
    {
      "thread": "thr-v1-172efbmpfmvk15kjws2nmc0h6j",
      "session": "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa",
      "agent": "cursor",
      "requests": {
        "owned": 1,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 2,
        "output": 1,
        "total": 3
      }
    },
    {
      "thread": "thr-v1-3tt2mxsat8ppq7tatwh9fk5jw7",
      "session": "77777777-7777-4777-8777-777777777777",
      "agent": "cursor",
      "requests": {
        "owned": 1,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 4,
        "output": 1,
        "total": 5
      }
    },
    {
      "thread": "thr-v1-41r9mkshyfn5c56jk2e6t0xrk1",
      "session": "66666666-6666-4666-8666-666666666666",
      "agent": "cursor",
      "requests": {
        "owned": 1,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 5,
        "output": 1,
        "total": 6
      }
    },
    {
      "thread": "thr-v1-447jdx7db7b6ne30w60pgfsffx",
      "session": "55555555-5555-4555-8555-555555555555",
      "agent": "cursor",
      "requests": {
        "owned": 1,
        "ambiguous": 0,
        "unknown": 0
      },
      "tokens": {
        "uncached_input": 8,
        "output": 2,
        "total": 10
      }
    }
  ],
  "diagnostics": []
}
? 0
```

## Report by provider, agent, model, effort and purpose

```console
$ urollup report --all --timezone UTC --group-by provider,agent,model,effort,purpose
urollup report
Selection all  Scope self  Timezone UTC

TOTALS
Requests  5
Owned     5
Ambiguous 0
Unknown   0
Uncached input 22
Cache read     -
Cache write    -
Output         6
Reasoning      -
Total tokens   28

COVERAGE
Status complete  Copies excluded 0  Limit observations 0
Unresolved 0  Possible 0  Requests without usage 0

REQUEST SIZES (inclusive input tokens)
Count 5  p50 4  p90 8  p99 8  max 8

AGENT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
cursor | 5 | 22 | 6 | 28

EFFORT BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
style-1 | 1 | 5 | 1 | 6
style-2 | 1 | 8 | 2 | 10
unknown | 3 | 9 | 3 | 12

MODEL BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
claude-4.6-opus-high-thinking | 1 | 8 | 2 | 10
composer-2.5-fast | 1 | 3 | 1 | 4
gemini-3-pro | 1 | 5 | 1 | 6
gpt-5.1-codex-high | 1 | 4 | 1 | 5
kimi-k2-instruct | 1 | 2 | 1 | 3

PROVIDER BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
anthropic | 1 | 8 | 2 | 10
cursor | 1 | 3 | 1 | 4
google | 1 | 5 | 1 | 6
moonshot | 1 | 2 | 1 | 3
openai | 1 | 4 | 1 | 5

PURPOSE BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
agent | 1 | 8 | 2 | 10
background | 1 | 2 | 1 | 3
chat | 1 | 5 | 1 | 6
multitask | 1 | 3 | 1 | 4
plan | 1 | 4 | 1 | 5
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
  "diagnostics": [],
  "totals": {
    "requests": {
      "owned": 5,
      "ambiguous": 0,
      "unknown": 0
    },
    "tokens": {
      "uncached_input": 22,
      "output": 6,
      "total": 28
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
          "owned": 5,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 22,
          "output": 6,
          "total": 28
        }
      }
    ],
    "effort": [
      {
        "group": "effort",
        "value": "style-1",
        "requests": {
          "owned": 1,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 5,
          "output": 1,
          "total": 6
        }
      },
      {
        "group": "effort",
        "value": "style-2",
        "requests": {
          "owned": 1,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 8,
          "output": 2,
          "total": 10
        }
      },
      {
        "group": "effort",
        "value": "unknown",
        "requests": {
          "owned": 3,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 9,
          "output": 3,
          "total": 12
        }
      }
    ],
    "model": [
      {
        "group": "model",
        "value": "claude-4.6-opus-high-thinking",
        "requests": {
          "owned": 1,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 8,
          "output": 2,
          "total": 10
        }
      },
      {
        "group": "model",
        "value": "composer-2.5-fast",
        "requests": {
          "owned": 1,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 3,
          "output": 1,
          "total": 4
        }
      },
      {
        "group": "model",
        "value": "gemini-3-pro",
        "requests": {
          "owned": 1,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 5,
          "output": 1,
          "total": 6
        }
      },
      {
        "group": "model",
        "value": "gpt-5.1-codex-high",
        "requests": {
          "owned": 1,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 4,
          "output": 1,
          "total": 5
        }
      },
      {
        "group": "model",
        "value": "kimi-k2-instruct",
        "requests": {
          "owned": 1,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 2,
          "output": 1,
          "total": 3
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
          "uncached_input": 8,
          "output": 2,
          "total": 10
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
          "uncached_input": 3,
          "output": 1,
          "total": 4
        }
      },
      {
        "group": "provider",
        "value": "google",
        "requests": {
          "owned": 1,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 5,
          "output": 1,
          "total": 6
        }
      },
      {
        "group": "provider",
        "value": "moonshot",
        "requests": {
          "owned": 1,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 2,
          "output": 1,
          "total": 3
        }
      },
      {
        "group": "provider",
        "value": "openai",
        "requests": {
          "owned": 1,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 4,
          "output": 1,
          "total": 5
        }
      }
    ],
    "purpose": [
      {
        "group": "purpose",
        "value": "agent",
        "requests": {
          "owned": 1,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 8,
          "output": 2,
          "total": 10
        }
      },
      {
        "group": "purpose",
        "value": "background",
        "requests": {
          "owned": 1,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 2,
          "output": 1,
          "total": 3
        }
      },
      {
        "group": "purpose",
        "value": "chat",
        "requests": {
          "owned": 1,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 5,
          "output": 1,
          "total": 6
        }
      },
      {
        "group": "purpose",
        "value": "multitask",
        "requests": {
          "owned": 1,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 3,
          "output": 1,
          "total": 4
        }
      },
      {
        "group": "purpose",
        "value": "plan",
        "requests": {
          "owned": 1,
          "ambiguous": 0,
          "unknown": 0
        },
        "tokens": {
          "uncached_input": 4,
          "output": 1,
          "total": 5
        }
      }
    ]
  },
  "sizes": {
    "count": 5,
    "p50": 4,
    "p90": 8,
    "p99": 8,
    "max": 8
  }
}
? 0
```

<!-- This document follows common-doc-guidelines.md.
See github.com/jlevy/practical-prose and review guidelines before editing.
-->
