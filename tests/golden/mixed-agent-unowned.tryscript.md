---
sandbox: true
path:
  - $UROLLUP_BIN
env:
  CLAUDE_CONFIG_DIR: $GOLDEN_FIXTURES/claude-project/ambiguous-owner
  CODEX_HOME: $GOLDEN_FIXTURES/codex-rollout/ambiguous-owner
  PI_CODING_AGENT_SESSION_DIR: $GOLDEN_EMPTY_ROOT/sessions
---
# Unowned Usage Keeps Its Agent

Both agents’ `ambiguous-owner` fixture cases, read together.
In each, two sessions of one agent both claim one response, so the response has no
single owner, yet its agent is known.
`sessions` keeps one unowned row per agent rather than merging them under an unknown
agent, never guesses a session or project for them, and its rows still add up to the
report’s totals: 6 requests and 30,335 tokens.

## Report Totals

```console
$ urollup report --all --group-by model --timezone UTC
urollup report
Selection all  Scope self  Timezone UTC

TOTALS
Requests  6
Owned     4
Ambiguous 2
Unknown   0
Uncached input 6,065
Cache read     22,500
Cache write    300
Output         1,470
Reasoning      300
Total tokens   30,335

COVERAGE
Status complete  Copies excluded 0  Limit observations 0
Unresolved 0  Possible 0  Requests without usage 0

REQUEST SIZES (inclusive input tokens)
Count 6  p50 2,230  p90 12,000  p99 12,000  max 12,000

MODEL BREAKDOWN
VALUE | REQUESTS | INPUT | OUTPUT | TOTAL
claude-sonnet-4-5 | 3 | 3,865 | 170 | 4,035
gpt-5.2-codex | 3 | 25,000 | 1,300 | 26,300

DIAGNOSTICS
conflicting-owners x4: proven owners thr-v1-262jyyetjxsmg2wc05d3txhh68, thr-v1-4bs081r5c7z8t920qtmrps06f0
? 0
```

## One Unowned Row per Agent

```console
$ urollup sessions --all --timezone UTC
urollup sessions
Selection all  Scope self  Timezone UTC

THREAD | AGENT | PROJECT | REQUESTS | INPUT | OUTPUT | TOTAL
unowned | claude | - | 1 | 2,230 | 80 | 2,310
unowned | codex | - | 1 | 12,000 | 600 | 12,600
thr-v1-1ctftnp1tgm3qwb85vbdshjahb | claude | project | 1 | 515 | 40 | 555
thr-v1-262jyyetjxsmg2wc05d3txhh68 | codex | project | 1 | 8,000 | 400 | 8,400
thr-v1-4bs081r5c7z8t920qtmrps06f0 | codex | project | 1 | 5,000 | 300 | 5,300
thr-v1-552thqtbre5b2fbhx3xrwcvz1e | claude | project | 1 | 1,120 | 50 | 1,170

DIAGNOSTICS
conflicting-owners x4: proven owners thr-v1-262jyyetjxsmg2wc05d3txhh68, thr-v1-4bs081r5c7z8t920qtmrps06f0
? 0
```

In JSON an unowned row has a null `thread` and `session`, no `project`, and its source
agent.

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
      "count": 4,
      "detail": "proven owners thr-v1-262jyyetjxsmg2wc05d3txhh68, thr-v1-4bs081r5c7z8t920qtmrps06f0"
    }
  ]
}
? 0
```

<!-- This document follows common-doc-guidelines.md.
See github.com/jlevy/practical-prose and review guidelines before editing.
-->
