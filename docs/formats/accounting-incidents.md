# Accounting Incidents and Regression Evidence

All numbers and identifiers in these explanations are synthetic.
Private comparisons motivated investigation but are not reproduced here.
This log records confirmed causes; an unexplained difference against another tool is not
automatically a urollup bug.

## Codex Inherited-Prefix Double Counting

**Tracking:** `uro-kpbp` (closed); fixed by
[PR #16](https://github.com/jlevy/urollup/pull/16), merged to `main` at `bdaa7ce` on
2026-10-09.

**Trigger:** a child rollout declares `subagent_history_start_ordinal: 3`, contains
inherited usage at ordinal 1, and starts its own usage at ordinal 3, but does not embed
a foreign parent `session_meta`.

**Wrong assumption:** the parser initialized the active owner to the child and relied on
a foreign header to enter inherited-history state.
The explicit ordinal boundary only switched ownership back to the child; it did not
classify the preceding prefix.

**Consequence:** a parent’s 100 tokens followed by 20 new child tokens could produce 220
combined tokens instead of 120, with three requests instead of two and incorrectly
complete coverage.

**Correction:** classify the prefix using the explicit boundary, independently of a
foreign header. Retain inherited counters as the child’s starting baseline, never as
child originals. Preserve unknown parent ownership.
Unnamed usage (a counter, or a record without `thread_id`) that an invalid boundary or a
missing ordinal cannot place, and a first own counter step whose `last_token_usage` does
not verify it against the inherited baseline, are excluded rather than counted: each
affected rollout gets one `codex-history-boundary-unverified` diagnostic and a coverage
gap, and every other session still reports.

**Prevention:**
[paginated fork tests](../../crates/urollup-core/tests/paginated_forks.rs) cover direct
and counter records, compacted copies, absent parents, unknown owners, counter resets
and worker/source-order equivalence.
The `paginated-counter-prefix` and `unverified-fork-boundary`
[fixture cases](../../crates/urollup-core/tests/fixtures/README.md) pin the two-file
shape and the per-rollout degrade with expected results and CLI goldens, and a
[process test](../../crates/urollup/tests/cli_process.rs) requires a report beside an
unverifiable rollout to succeed.
The earlier fixtures did not cover the declared-boundary-without-foreign-header
combination.
Tests must vary evidence independently instead of assuming those two signals
always occur together.

## Codex Counter-Only Usage Beside Usage Records

**Tracking:** `uro-h2sf` (closed); fixed for root rollouts by
[PR #26](https://github.com/jlevy/urollup/pull/26). Forks and subagents keep the gap
(`uro-r8si`).

**Trigger:** a session that a Codex release before 0.153 started and a later release
resumed, or one an older release appended turns to, holds counter-only turns beside its
`token_usage_record` lines.

**Wrong assumption:** a rollout with any usage record was accounted from its records
alone, so every cumulative `token_count` in it counted nothing.

**Consequence:** the counter-only turns were lost with complete coverage and no
diagnostic; the `mixed-counter-direct` fixture reported 90,000 instead of 139,700
tokens.

**Correction and prevention:** in a root rollout a counter counts through the counter
rules unless it is the twin of an adjacent usage record with equal native usage, which
adds no request ([Codex contract](codex.md#accounting-rules)). The
[mixed usage tests](../../crates/urollup-core/tests/codex_mixed_usage.rs) and the
`mixed-counter-direct` fixture case, with its results check and CLI golden, pin it.

## Codex Migrated Children Excluded as Copied History

**Tracking:** `uro-jqc3` (closed); fixed by
[PR #32](https://github.com/jlevy/urollup/pull/32). The related migrated user-fork
recount is open as `uro-p9ua`.

**Trigger:** Codex’s legacy-to-paginated migration sets a child’s
`subagent_history_start_ordinal` one past the last line it rewrote, so the child’s own
records and counters sit before the boundary.

**Wrong assumption:** the inherited-prefix correction above treated every line before an
explicit boundary as inherited history, including records that name the child.

**Consequence:** migrated Guardian reviews and subagents lost their own usage with
complete coverage; the `guardian-migrated-boundary` fixture reported 9,600 instead of
21,190 tokens.

**Correction and prevention:** a record names its owner wherever it sits, the turns a
complete parent root recorded decide a counter-only child’s unassigned prefix, and a
`compacted` record that is the only record of its response counts as the original; what
none of this decides is an unverified coverage gap
([Codex contract](codex.md#accounting-rules)). The `guardian-migrated-boundary` and
`guardian-native-boundary` fixture cases and the migrated-child
[paginated fork tests](../../crates/urollup-core/tests/paginated_forks.rs) pin it.

## Codex Cache-Write Double Counting

**Tracking:** `uro-381f`; corrected by `InputSemantics::IncludesCache` in
[PR #19](https://github.com/jlevy/urollup/pull/19). Before it, Codex normalization
subtracted cache reads only.

**Wrong assumption:** inclusive native input was normalized by subtracting cache reads
only, leaving cache-write tokens in ordinary input while also recording them as writes.

**Consequence:** input 100, reads 20 and writes 10 became ordinary input 80 plus the
same 10 writes. The correct ordinary input is 70; inclusive input remains 100. The error
affects both totals and any estimate applying separate cache-write rates.

**Evidence:** Codex copies `input_tokens` and both cache counts from the Responses API
usage verbatim, the cache counts from `input_tokens_details`, and already treats the
cached count as a subset of `input_tokens`
([source review](../project/research/research-2026-09-14-agent-tool-source-reviews.md#codex-token-usage)).
Pi’s Responses mapping subtracts both cached and cache-write tokens from `input_tokens`
([Pi facts](../project/research/research-2026-09-14-agent-tool-source-reviews.md#pi-session-dialect-facts)),
and ccusage models writes inside input from commit `15b3bef`
([ccusage facts](../project/research/research-2026-09-14-agent-tool-source-reviews.md#ccusage-dialect-facts)).
Codex source does not state the inclusion, so the rule is an inference from these
sources, not a stated contract.
Live provider documentation is not a historical rate table and cannot establish
effective pricing dates.

**Correction and prevention:** checked subtraction of both categories, with tests for
nonzero writes, underflow and maximum counters in
[token normalization](../../crates/urollup-core/src/ledger/tokens.rs), counter ingestion
coverage in [pricing context tests](../../crates/urollup-core/tests/pricing_context.rs),
and the `cache-write-input` fixture case, whose results check and CLI golden fail under
the old rule by its 1,000 cache-write tokens.
An input below the two cache categories stops ingestion, as one below cache reads
already did, so if the inference is wrong for some producer the run fails instead of
miscounting. Zero-write fixtures could not expose this defect.
Every separately billed category needs a nonzero conservation test, not just a parsing
assertion.

## Codex Lowered Total Counted in Full

**Tracking:** `uro-v1c9` (closed); fixed on `main` by PR #18, with its fixture case from
PR #22.

**Trigger:** a legacy rollout’s cumulative `token_count` total falls next to a
`compacted` record without restarting from zero, while the decreasing record’s
`last_token_usage` carries the request’s own usage.

**Wrong assumption:** any decrease was a counter restart, so the new cumulative total
was the request’s usage.

**Consequence:** totals of 100,000, 200,000 and 300,000 followed by a lowered total of
250,000 with last usage 10,000 charged that request 250,000 tokens instead of 10,000.

**Correction:** a decrease still opens a new counter epoch with
`codex-counter-epoch-reset`, but the decreasing record counts its own
`last_token_usage`, and one whose last usage has zero input and output adds no request.

**Prevention:** the results check of the `compaction-lowered-total`
[fixture case](../../crates/urollup-core/tests/fixtures/README.md) fails under the old
rule by exactly the whole-new-total overcount, and a CLI golden pins its report.
[Paginated fork tests](../../crates/urollup-core/tests/paginated_forks.rs) pin how the
rule meets a child’s first own counter step.

## Codex Subagent Requests Without a Model

**Tracking:** `uro-5nkv` (closed); fixed on `main` by PR #18.

**Trigger:** a multi-agent subagent’s `token_usage_record` carries its own `turn_id` and
a `root_turn_id` that names the parent’s turn.

**Wrong assumption:** turn context was looked up by `root_turn_id`, which agrees with
`turn_id` only in top-level rollouts.

**Consequence:** every such subagent request reported model and effort unknown.
Token totals were unchanged, but any per-model view or list-price estimate would lose or
misattribute those requests.

**Correction and prevention:** a record takes its context from its own `turn_id`, and
from `root_turn_id` only when it has no `turn_id`; its retained provider and service
tier come from the same turn.
The e2e results check compares per-request model and effort buckets, so the
`archived-rename` and `auto-review-model` cases fail under the old lookup (`uro-dp4x`).

## Copy-Only Coverage

**Tracking:** `uro-xpd0` (open decision).

Excluding copies prevents duplication, but a copy whose original is missing also shows
that some usage was not observed.
PR #16 briefly made such copy-only requests mark whole-history and selected-owner
coverage partial (`b79d96c`). That rule was reverted before PR #16 merged (`1346709`):
it fired on Codex forks whose parent is present, because copied `token_count` events in
direct-usage files cannot be keyed, and on nested paginated subagents whose originals
all exist, and it changed Claude coverage without a recorded decision.
A copy-only request is excluded from counted totals, keeps its copy evidence, and does
not change completeness.
Claude Code reports it as `claude-nested-copy-without-original`; the Codex adapter emits
no diagnostic for it.
Whether it should also make coverage partial remains the open decision.

## Related Open Problems

| Issue | Consequence | Tracking |
| --- | --- | --- |
| Open Codex accounting fixes | Listed with their synthetic reproductions in the [release readiness record](../project/specs/active/plan-2026-09-16-first-release-publishing.md#open-correctness-fixes) | `uro-r8si`, `uro-eh0d`, `uro-p9ua`, `uro-3b12` |
| Exact `--session` selection misses another session’s claim on a shared response | A single-session run counts a response that whole history reports as ambiguous | `uro-s71z` |
| Missing rates, price matching and cost output | Token reports do not yet provide monetary rollups | `uro-wuby`, `uro-neii` |
| Unexplained real-history comparator differences | Prevents claiming accounting acceptance solely from passing synthetic tests | `uro-d36a`, `uro-ky6c` |

Complete each fix with a minimal synthetic reproduction, its violated invariant, and an
input-to-report regression.
Do not label all comparator residuals explained by the fork fix or describe the alpha as
accepted before controlled full-history QA.

<!-- This document follows common-doc-guidelines.md.
See github.com/jlevy/practical-prose and review guidelines before editing.
-->
