# CLI Goldens and End-to-End Result Checks

urollup’s command-line behavior is tested in two complementary layers, following
`tbd guidelines golden-testing-guidelines` and the reference of the pinned
[tryscript](https://github.com/jlevy/tryscript) 0.2.1, which the package prints with
`node_modules/.bin/tryscript docs`:

| Layer | What it holds | What it catches |
| --- | --- | --- |
| **Transcript goldens** (`*.tryscript.md`) | Complete stdout, stderr and exit status of each command, as a reviewable session | Any change in what a user sees, including changes nobody wrote a test for |
| **Result checks** (`scripts/check-e2e-results.mjs`) | Reconciled truth per fixture case, from that case’s `expected.json` | Wrong totals, ownership, excluded copies, limit observations or diagnostics, with a field diff and the naive-sum overcount |

The transcript layer is transparent-box: it shows whole outputs, never a `grep` or `jq`
slice of one value. The result layer is the domain assertion beside it, so a diff and a
wrong number fail differently.
Both run on a build of this repository, never against an installed `urollup`.

```bash
make golden          # build, lint the corpus, compare every session
make e2e-results     # reconciled results for every fixture case
make golden-update   # rewrite stale expectations, then compare (read the diff)
make test            # both, plus the Rust tests
```

## Isolation

`make check` normally runs inside a coding agent, whose environment names that agent’s
live session (`CLAUDE_CODE_SESSION_ID`, `CODEX_THREAD_ID`, `PI_SESSION_FILE`) and whose
`HOME` holds the maintainer’s real logs.
Nothing in a golden run may read those, so
[`scripts/golden-env.mjs`](../../scripts/golden-env.mjs) builds the environment from
nothing:

- **Allowlist:** only `PATH` and the temporary-directory and shell variables each
  platform needs survive; every `CLAUDE_*`, `CODEX_*`, `PI_*`, `UROLLUP_*`, `GIT_*` and
  credential variable is dropped.
- **Hermetic HOME:** `HOME`, the Windows profile variables and the XDG base directories
  point inside a fresh `urollup-golden-*` temporary root, and `UROLLUP_CAPTURE_DIR`
  points beside it, so a capture (milestone 0.3) cannot reach a platform data directory.
- **Canaries:** that HOME holds a synthetic log in every default discovery root
  (`~/.claude/projects`, `$XDG_CONFIG_HOME/claude/projects`, `~/.codex/sessions`,
  `~/.codex/archived_sessions`, `~/.pi/agent/sessions`) and a default source manifest,
  each naming `urollup-home-canary` in its path, model and working directory.
  A run that reads default roots therefore prints the token instead of quietly reading
  whatever HOME holds, and the corpus lint refuses the token in any committed file.
- **Write guard:** the runners snapshot that HOME and fail on anything a run created,
  changed or removed there, and the results checker also fails on a write into the
  working directory.
- **Per case:** each session or case names its own roots through the native variables
  (`CLAUDE_CONFIG_DIR`, `CODEX_HOME`, `PI_CODING_AGENT_SESSION_DIR`), pointing every
  agent it does not use at an empty root, and fixes `--timezone`. The
  [ccusage reconciliation harness](../../docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md#ccusage-reconciliation-harness)
  isolates its runs the same way, over the same fixture roots.

Windows resolves its platform config and data directories through the shell API rather
than `APPDATA` and `LOCALAPPDATA`, so isolation there depends on urollup honoring its
own override variables; goldens never rely on a platform default holding nothing.

## Writing a Session

Front matter is fixed by
[`scripts/check-golden-invocations.mjs`](../../scripts/check-golden-invocations.mjs),
which fails the run rather than let a session read something ambient or skip a block:

```yaml
---
sandbox: ../../../crates/urollup-core/tests/fixtures/claude-project/<case> # or `true`
path:
  - $UROLLUP_BIN # exactly this: tryscript's path: falls through to the real PATH
env:
  CLAUDE_CONFIG_DIR: . # the sandbox copy of the case
  CODEX_HOME: $GOLDEN_EMPTY_ROOT # every agent the case does not use
---
```

- Commands are bare `urollup …` invocations with no shell syntax, because sessions run
  under `/bin/sh` and `cmd.exe` alike; paths reach them through `sandbox:` and front
  matter, never through `$VAR` in a command.
- `env:` and `path:` expand only harness variables: `$GOLDEN_HOME`,
  `$GOLDEN_EMPTY_ROOT`, `$GOLDEN_FIXTURES`, `$GOLDEN_SAMPLES`, `$UROLLUP_BIN` and
  tryscript’s own `$TRYSCRIPT_*`.
- Every reading command expected to exit 0 passes `--timezone`, since the default is the
  system zone and Windows ignores `TZ`.
- `<!-- skip -->` and `<!-- only -->` are refused: tryscript reports a skipped block as
  passed, and `only` drops the rest.
  A block that should not run yet is deleted, with a bead.
- Unknown wildcards (`[??]`, `???`) are scaffolding for `--expand`, never committed.

## Normalization

Elide only what genuinely varies, and name what it is (tryscript’s wildcard categories,
in order of preference: named patterns, then unknown wildcards, then generic ones).
Fixture-derived values, including record timestamps, session IDs and every token count,
stay literal: a pattern over them would hide the behavior under test.
`tryscript.config.mjs` defines the patterns that do vary:

| Pattern | Use |
| --- | --- |
| `[GOLDEN_HOME]`, `[GOLDEN_FIXTURES]` | Absolute harness paths, if one ever reaches output |
| `[CWD]`, `[ROOT]` | tryscript’s own sandbox and session directories |
| `[NOW]` | A wall-clock instant, such as when a report was generated |
| `[ELAPSED]` | Measured elapsed time |
| `[UROLLUP_VERSION]` | The package version away from `urollup --version`, which pins it literally once |

## Updating

1. Make the change, then run `make golden`. A failure prints the diff.
2. If the new behavior is right, run `make golden-update`
   (`GOLDEN=tests/golden/e2e/<dialect>/<case>.tryscript.md` limits it to one session).
   It refuses to run while tracked goldens have unstaged changes, so the diff afterwards
   is exactly what tryscript wrote.
3. Read `git diff tests/golden/` line by line.
   `--update` writes what it saw, so it happily expands a pattern into a local literal;
   `scripts/check-portability.mjs` refuses home and temporary directories, and the
   corpus lint refuses the HOME canary.
4. Commit the sessions with the code change, so review sees both.

## Fixture Cases and Their Goldens

Fixture cases live one directory per case under
`crates/urollup-core/tests/fixtures/<dialect>/<case>/`, laid out like a real discovery
root, each with an `expected.json` of reconciled truth.
The mapping is fixed and checked:

| Case | Transcript golden | Result check |
| --- | --- | --- |
| `<dialect>/<case>/` | `tests/golden/e2e/<dialect>/<case>.tryscript.md` | Automatic: every case with an `expected.json` is discovered |

A new dialect case therefore gets a golden this way:

```bash
node scripts/new-e2e-golden.mjs <dialect>/<case>          # scaffold with ??? placeholders
node scripts/run-golden.mjs --expand tests/golden/e2e/<dialect>/<case>.tryscript.md
make golden                                                # read every expanded line first
```

The scaffold records `report`, `daily` and `sessions` as tables and as JSON against a
sandbox copy of the case.
`make e2e-results` fails on a golden whose case is gone, and, once `urollup report`
exists, on a case that has no golden.

## The Results Contract

`scripts/check-e2e-results.mjs` reads each case’s `expected.json`. Results live under
`reconciled`, or at the top level in a flat style; an unrecognized key inside
`reconciled` is an error, so nothing a case asserts goes unchecked.

```json
{
  "case": "claude-project/block-records",
  "dialect": "claude-project",
  "description": "What this case demonstrates.",
  "reconciled": {
    "requests": 1,
    "ownership": { "owned": 1, "ambiguous": 0, "unknown": 0 },
    "tokens": { "uncached_input": 3, "cache_read": 40000, "cache_write": 0, "output": 600, "reasoning": 0 },
    "copies_excluded": 3,
    "unresolved": { "requests": 0, "extents": 0 },
    "possible": { "requests": 0 },
    "limit_observations": 0,
    "diagnostics": [{ "code": "claude.block_usage_differs", "count": 1 }]
  },
  "naive_sum": { "requests": 4, "tokens": { "uncached_input": 12, "cache_read": 160000, "output": 1224 } }
}
```

- Counts compare exactly; a result the report does not carry is a failure, never a skip.
- Diagnostics compare as a set of codes: a missing one and an unexpected one both fail,
  and a count compares when the case gives one.
  A diagnostic may also be written as a plain string.
- `naive_sum` is context, never an assertion: every case prints how far naive summing
  would have overcounted, which is the reason urollup exists.
- Accepted spellings: `unique_requests`, `ownership_counts`, `token_totals`,
  `excluded_copies`, `expected_diagnostics` and `naive`.

Each command runs twice and must print identical bytes, so nondeterministic ordering
fails on the first case rather than on a later machine.

## Pending States

Until the commands and the fixture corpus exist, the checker reports `PENDING` with the
bead that unblocks each item, and every pending entry is a ratchet
(`tests/golden/e2e.config.json`):

- A command that still exits 2 as the scaffold stub is pending only while its `pending`
  entry names a bead; once it is implemented, that entry fails the run until it is
  deleted, so results are checked from the first build that can produce them.
- The same holds for `pendingFixtures`: an existing corpus with the entry still present
  fails, and an existing corpus with no cases fails.
- A pending run never reports success: its summary says how many cases were checked and
  that the rest stay unverified.

## Harness Samples

`samples/` holds the harness’s own inputs, not accounting fixtures: miniature cases with
the layouts discovery must recognize, and synthetic report outputs, including a
naive-sum output that must fail.
`node --test scripts/check-e2e-results.test.mjs` runs the checker against them, so the
comparison, the diff and the ratchets are tested without any real fixture or binary.

Values there follow the research brief’s
[synthetic double-counting example](../../docs/project/research/research-2026-09-13-portable-agent-usage.md#synthetic-double-counting-example);
the report shapes and diagnostic codes are provisional until
`urollup report --format json` lands, and the bead that lands it updates the extractors
and these samples together.

<!-- This document follows common-doc-guidelines.md.
See github.com/jlevy/practical-prose and review guidelines before editing.
-->
