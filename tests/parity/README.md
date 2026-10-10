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
It starts four urollup processes however many sessions the history holds: `--version`,
then whole-history `sessions`, `daily` and `report`. A binary that cannot report its
version fails before any ingest, and one whose `sessions` rows lack the calendar fields
stops after that first ingest.
The record includes only complete days before the current local day: per-day and total
request counters, every urollup token field including the 5-minute, 1-hour and
unspecified cache-write lifetimes, coverage counters, internal diagnostic codes and
session counts.

A token field that no counted request reported is null, not zero.
`totals.token_day_coverage` labels each metric `all_days` when every summed day row
carried it, `some_days` when only some did, and `no_days` when none did.
The unit is the day row, not the request: a urollup day row carries a field when any of
its counted requests reported it.
A day that mixes Claude and Codex requests therefore counts as carrying an
agent-specific field such as `reasoning` although the other agent’s requests lacked it,
so a window of such days can read `all_days`, and the sum covers only the requests that
reported the field. Request-level availability waits on per-metric request counts in
urollup’s query rows (`uro-r67m`).

Sessions are classified on the whole-history ledger, the one the record’s totals use.
A session is stable when its `sessions` row owns a counted request, has no
`undated_requests` and has a `last_date` before the current local day.
urollup dates those rows by the rule `daily` uses.
`sessions.excluded` counts the rest: `active_or_undated` for sessions with usage on or
after the cutoff day or without a timestamp, `without_owned_requests` for sessions that
own no counted request, and `unowned_or_unrecognized_agent` for each agent’s unowned
row, which has no thread, and for rows whose agent token this tool does not recognize.
A request that two sessions both prove they own is ambiguous in the whole-history ledger
and belongs to neither, so a finished session whose requests are all ambiguous counts as
`without_owned_requests`. A per-session `urollup daily --session` run narrows discovery
to that session’s file family and can own the same requests, so it may count such a
session as stable.

`parity-local` writes `target/parity/local.json` with tool versions, platform, timezone,
the half-open interval, token totals and deltas, per-day deltas, the safe `other` model
bucket, matched and one-sided session counts, a relative-delta histogram and the
unexplained residual.
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
