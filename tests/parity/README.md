# ccusage parity harness

This harness runs urollup and the native executable from the exactly pinned ccusage
20.0.20 package over temporary copies of every frozen Claude Code and Codex fixture.
It compares daily and session token rows in UTC and America/Los_Angeles, with and
without ccusage’s `--since` filter, plus the unified daily report.

Run `make parity`. The target installs the isolated package with lifecycle scripts
disabled, checks the native binary’s version, runs the comparator, and writes
regenerated JSON reports below `target/parity/`. CI uploads that directory as an
artifact; reports are not committed.

Every observed difference must match exactly one cited behavior in
[ledger.toml](ledger.toml).
A behavior records its cause, evidence, design treatment, and retirement condition.
Its nested rules declare the exact fixture deltas and how many matrix cells must match
each pattern. Unexplained differences, double matches, wrong deltas, changed match
counts, and rules made stale by an upstream fix all fail the target.

The fixture process receives empty home and XDG directories, dedicated Claude and Codex
roots, no `.ccusage` configuration, `--offline --json`, the same explicit timezone on
both sides, and no inherited agent-discovery variables.
The harness invokes the native platform executable directly rather than `npx`, `bunx`,
`pnpm dlx`, or ccusage’s JavaScript launcher.

## Local acceptance

Two maintainer-only targets exercise default local roots.
They are absent from `test`, `check` and CI, and both refuse to run unless the caller
supplies explicit consent:

```bash
make e2e-local CONSENT_LOCAL_LOGS=1
make parity-local CONSENT_LOCAL_LOGS=1
```

`e2e-local` writes `target/acceptance/local-aggregate.json` in the
`urollup.local-aggregate/v2` format.
It starts four urollup processes however many sessions the history holds: `--version`
and whole-history `daily`, `report` and `sessions`. The record includes only complete
days before the current local day: per-day and total request counters, every urollup
token field including the 5-minute, 1-hour and unspecified cache-write lifetimes,
coverage counters, internal diagnostic codes and counts of stable sessions.
A token field that no counted request reported is null, not zero.
`totals.token_availability` marks each metric `observed` when every summed day reported
it, `partial` when only some did, and `unknown` when none did; urollup does not yet
count the requests inside a day that lacked a field.
A session is stable when its whole-history `sessions` row has a counted request, no
`undated_requests` and a `last_date` before the current local day.
urollup dates those rows by the rule `daily` uses, so the classification is the one a
per-session `daily` query over the same whole-history ledger gives, without one ingest
per session. `parity-local` writes `target/parity/local.json` with tool versions,
platform, timezone, the half-open interval, token totals and deltas, per-day deltas, the
safe `other` model bucket, matched and one-sided session counts, a relative-delta
histogram and the unexplained residual.
Session counts come from one whole-history urollup `sessions` run and one ccusage
`session` run per agent, joined in memory on the native `session` field; a Codex rollout
path joins on its trailing thread ID. Only sessions whose ccusage last activity falls
before the current local day are compared or counted as ccusage-only.
A urollup row that joins nothing counts as urollup-only when it has requests, unless it
is a Claude subagent of an active session.

Both scripts may hold paths and identifiers in memory while joining results, but their
output builders use fixed allowlists.
CI unit tests feed unique markers through paths, projects, sessions, requests, prompts
and custom model names and fail if any marker reaches the rendered record.
They also fail if the number of urollup processes `e2e-local` starts changes with the
number of sessions. Subprocess failures are reported without forwarding tool diagnostics
that could contain a private path.
Generated reports remain under the ignored `target/` tree until a maintainer reviews an
aggregate for release records.

<!-- This document follows common-doc-guidelines.md.
See github.com/jlevy/practical-prose and review guidelines before editing.
-->
