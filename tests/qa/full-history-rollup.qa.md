---
title: Full-History Rollup QA
description: Manual end-to-end validation that urollup rolls up the whole local Claude Code and Codex history on a real machine, fast and within its memory bound, with correct per-project, per-session and resumed-history totals.
date: 2026-09-16
author: Joshua Levy (github.com/jlevy) with LLM assistance
status: Active; blocked until Phase 1 of the scalable ingestion plan lands
---
# QA Playbook: Full-History Rollup

Manual QA playbook for urollup’s default whole-history reports on a machine with a large
real corpus of Claude Code and Codex logs.

**Purpose:** prove that `sessions`, `daily` and `report --all` read the entire local
history without an input-size limit, finish in seconds within a few hundred MiB, and
produce totals that are internally consistent, deterministic, correct for individual
sessions and projects, and not double counted or dropped across resumed and forked
sessions.

**Estimated time:** about 45 minutes: 5 for setup, 5 for whole-history runs, 10 for
invariants and determinism, 15 for project and session cross-checks, and 10 for resumed
history and recording.

> This is a manual test: the real corpus is private and machine-specific, so it cannot
> be a committed fixture.
> An agent can follow these steps, evaluate each checkpoint, and share results with a
> human as it progresses.

* * *

## Current Status (Last Update 2026-09-16)

| Phase | Status | Notes |
| --- | --- | --- |
| Phase 1: Setup | ⏸️ Blocked | Requires Phase 1 of the scalable ingestion plan |
| Phase 2: Whole-history runs | ⏸️ Blocked | The current engine refuses input above 512 MiB |
| Phase 3: Invariants and determinism | ⏸️ Blocked | Commands validated on fixtures |
| Phase 4: Per-project cross-checks | ⏸️ Blocked | Needs Phase 2 whole-history runs |
| Phase 5: Hand-summed sessions | ⏸️ Blocked | Commands validated on fixtures |
| Phase 6: Resumed and forked history | ⏸️ Blocked | Order bug `uro-sn1e` reproduced on fixtures |
| Phase 7: Live current session and local parity | ⏸️ Blocked | Needs Phase 2 whole-history runs |
| Phase 8: Record and clean up | ⏸️ Blocked |  |

**Status legend:** ✅ Passed | ❌ Failed | ⏳ Pending | ⏸️ Blocked

**Test results (last update 2026-09-16):**

- Invariant, hand-sum and measurement commands → ✅ validated against fixture corpora
  with the current engine.
- Whole-history runs → ⏸️ blocked by the 512 MiB input guard.

**Next steps:**

1. Land Phase 1 of the
   [scalable ingestion plan](../../docs/project/specs/active/plan-2026-09-16-scalable-ingestion.md).
2. Run Phases 1–8 of this playbook and record results in a dated QA report.

* * *

**Prerequisites:**

- A macOS machine with the maintainer’s real Claude Code and Codex history in the
  default locations, and the maintainer’s consent to read it.
- The pinned Rust toolchain, uv and npm dependencies from [AGENTS.md](../../AGENTS.md),
  `jq`, and `make parity` run once so the pinned ccusage 20.0.20 binary is installed.
- At least 3 GB of free disk and no heavy memory users running, such as other agents
  building or testing.

**Privacy rules:**

- Keep every capture under the ignored `target/qa/full-history/` directory.
- The dated QA report records only aggregate counts, timings, footprints, deltas and
  pass or fail results.
  Name this repository as `urollup`; name other repositories `Repo A`, `Repo B` and
  `Repo C`. Never record paths, session IDs, prompts or custom model names.

* * *

## Related Documentation: Read for Context

- [Scalable whole-history ingestion plan](../../docs/project/specs/active/plan-2026-09-16-scalable-ingestion.md):
  targets, memory model and semantic changes this playbook validates.
- [Milestone 0.1 local acceptance QA](milestone-0.1-local-acceptance.qa.md): terminal
  behavior, consent gates and bounded-corpus checks.
- [Milestone 0.1 QA report](../../docs/project/qa/qa-report-2026-09-16-milestone-0.1.md):
  the memory incidents that motivated this playbook.
- [Parity harness](../parity/README.md): ccusage token field mapping and the explained
  differences ledger.

## Phase 1: Setup

### 1.1 Build and Define Helpers

```bash
git rev-parse --short HEAD
cargo build --locked --release --workspace
df -h ~ | tail -1
memory_pressure | tail -1
mkdir -p target/qa/full-history
export UR=target/release/urollup
export QA=target/qa/full-history
export CCUSAGE=tests/parity/ccusage/node_modules/@ccusage/ccusage-darwin-arm64/bin/ccusage
unset CLAUDE_CODE_SESSION_ID CODEX_THREAD_ID
measure() {
  local name=$1; shift
  uv --config-file uv.toml run --frozen python scripts/run-rss-watchdog.py \
    --limit-mib 1024 --report "$QA/$name.watchdog.json" -- \
    /usr/bin/time -l "$@" > "$QA/$name.json" 2> "$QA/$name.stderr"
  local status=$?
  printf '%s exit=%s wall=%ss footprint=%sMiB\n' "$name" "$status" \
    "$(awk '$2 == "real" {print $1; exit}' "$QA/$name.stderr")" \
    "$(awk '/peak memory footprint/ {printf "%.1f", $1 / 1048576}' "$QA/$name.stderr")"
}
```

Always measure the freshly built `target/release/urollup`, never an installed copy.
`/usr/bin/time -l` reports **peak memory footprint**, which includes compressed pages
and is the number this playbook records.
The watchdog’s 1 GiB RSS limit is only a kill switch, because RSS omits compressed
memory on macOS.

**Verify:**

- [ ] The release build succeeds.
- [ ] At least 3 GB of disk is free, and `memory_pressure` reports a normal level.
- [ ] `"$CCUSAGE" --version` prints `ccusage 20.0.20`.

**Troubleshooting:**

- **Issue:** memory pressure is already warning or critical.
  **Fix:** stop other heavy processes first; footprint and timing results are not
  meaningful under pressure.

### 1.2 Record Corpus Volume

```bash
du -sh ~/.claude/projects ~/.codex/sessions ~/.codex/archived_sessions 2>/dev/null
find ~/.claude/projects -name '*.jsonl' | wc -l
find ~/.codex/sessions ~/.codex/archived_sessions -name 'rollout-*' 2>/dev/null | wc -l
```

**Verify:**

- [ ] The corpus size and file counts are recorded; on the reference machine they were
  about 2.7 GB in 2,920 Claude files and 12 GB in 8,900 Codex rollouts on 2026-09-16.

## Phase 2: Whole-History Runs

### 2.1 Default Commands Over All History

```bash
measure sessions "$UR" sessions --all --format json --no-progress
measure daily "$UR" daily --all --format json --no-progress
measure report "$UR" report --all --group-by project,model --format json --no-progress
for name in sessions daily report; do jq empty "$QA/$name.json" && echo "$name valid"; done
```

**Expected output:** three lines like `sessions exit=0 wall=8.42s footprint=301.5MiB`,
then three `valid` lines.

**Verify:**

- [ ] Every command exits 0; none prints a safety-limit or capacity error.
- [ ] Wall time is at most 25 s after plan Phase 1 and at most 10 s after plan Phase 2.
- [ ] Peak footprint is at most 512 MiB for every command.
- [ ] No watchdog report has `"killed_for_rss": true`.
- [ ] `jq '.coverage.limit_observations' "$QA/report.json"` is greater than 0 and
  `jq '.diagnostics | length' "$QA/report.json"` is at most 30.
- [ ] Negative check: `grep -c 'safety limit' "$QA"/*.stderr` prints 0 for every file.

**Troubleshooting:**

- **Issue:** the watchdog kills a run.
  **Fix:** mark the phase **Failed**; do not retry without the watchdog.
  Rerun with `UROLLUP_STATS=1` on a single project root to find the phase that grows.
- **Issue:** a run is much slower the first time.
  **Fix:** record the first run as cold and rerun once for a warm time; the thresholds
  apply to the warm run.

### 2.2 Row Counts

```bash
jq '.rows | length' "$QA/sessions.json" "$QA/daily.json"
jq '.rows | group_by(.agent) | map({agent: .[0].agent, sessions: length})' "$QA/sessions.json"
jq '.totals.requests' "$QA/report.json"
```

**Verify:**

- [ ] Both agents appear with session counts of the same order as the file counts from
  step 1.2.
- [ ] The daily row count roughly matches the number of active days in the corpus.

## Phase 3: Invariants and Determinism

### 3.1 Totals Agree Across Reports

```bash
for m in total output cache_read cache_write uncached_input; do
  printf '%-15s %s %s %s %s\n' "$m" \
    "$(jq --arg m "$m" '[.rows[].tokens[$m] // 0] | add' "$QA/daily.json")" \
    "$(jq --arg m "$m" '.totals.tokens[$m] // 0' "$QA/report.json")" \
    "$(jq --arg m "$m" '[.rows[].tokens[$m] // 0] | add' "$QA/sessions.json")" \
    "$(jq --arg m "$m" '[.breakdowns.project[].tokens[$m] // 0] | add' "$QA/report.json")"
done
jq '[.rows[].requests | .owned + .ambiguous + .unknown] | add' "$QA/daily.json" "$QA/sessions.json"
jq '.totals.requests | .owned + .ambiguous + .unknown' "$QA/report.json"
```

**Expected output:** each token line shows four identical numbers, and the three request
counts are identical.

**Verify:**

- [ ] Daily sum = report total = sessions sum = per-project sum, for every token class.
- [ ] Request counts agree across the three reports.
- [ ] The model breakdown also sums to the report total:
  `jq '[.breakdowns.model[].tokens.total // 0] | add' "$QA/report.json"`.

**Troubleshooting:**

- **Issue:** the model breakdown differs slightly from the total.
  **Fix:** multi-model requests split by component; compare with
  `jq '.breakdowns.model' "$QA/report.json"` and confirm the difference is exactly the
  advisor components before marking **Failed**.

### 3.2 Deterministic Output

```bash
measure report-rerun "$UR" report --all --group-by project,model --format json --no-progress
UROLLUP_JOBS=1 measure report-jobs1 "$UR" report --all --group-by project,model --format json --no-progress
cmp "$QA/report.json" "$QA/report-rerun.json" && echo "rerun identical"
cmp "$QA/report.json" "$QA/report-jobs1.json" && echo "one worker identical"
```

**Verify:**

- [ ] Both comparisons print `identical`. Logs that changed between runs, such as an
  active agent session, make a difference expected; rerun with other agents stopped
  before marking **Failed**.
- [ ] The one-worker run is slower but stays within the footprint limit.

## Phase 4: Per-Project Cross-Checks

Pick this repository and three other active repositories.
Use each repository directory’s basename as its urollup project and its encoded Claude
project directory, such as `-Users-me-wrk-github-urollup`, for ccusage.

### 4.1 Project Rows Match Session Rows

```bash
export PROJECT=urollup
jq --arg p "$PROJECT" '.breakdowns.project[] | select(.value == $p) | .tokens.total' "$QA/report.json"
jq --arg p "$PROJECT" '[.rows[] | select(.project == $p) | .tokens.total] | add' "$QA/sessions.json"
```

**Verify:**

- [ ] Both numbers are equal for each of the four projects.

### 4.2 Claude Project Totals Match ccusage

```bash
export CLAUDE_DIR=-Users-me-wrk-github-urollup
"$CCUSAGE" claude session --json --offline > "$QA/ccusage-claude-sessions.json"
jq --arg d "$CLAUDE_DIR" '[.sessions[] | select(.projectPath == $d)] | {sessions: length, uncached_input: (map(.inputTokens) | add), cache_read: (map(.cacheReadTokens) | add), cache_write: (map(.cacheCreationTokens) | add), output: (map(.outputTokens) | add)}' "$QA/ccusage-claude-sessions.json"
measure "claude-$PROJECT" "$UR" report --all --source ~/.claude/projects/"$CLAUDE_DIR" --no-default-sources --format json --no-progress
jq '.totals.tokens | {uncached_input, cache_read, cache_write, output}' "$QA/claude-$PROJECT.json"
```

**Verify:**

- [ ] For each project, every token class matches exactly or its delta is explained by a
  behavior in [the parity ledger](../parity/ledger.toml), such as gateway message-ID
  reuse or uuid replays.
- [ ] Record the relative total delta per project; any unexplained delta above 0.1% is
  **Failed** until investigated.

**Troubleshooting:**

- **Issue:** a project differs by exactly one session’s usage.
  **Fix:** check whether that session was resumed from another project directory; a
  single-project `--source` run cannot see the original, so repeat the comparison with
  whole-history session rows before marking **Failed**.

### 4.3 Codex Project Totals Match ccusage

```bash
"$CCUSAGE" codex session --json --offline > "$QA/ccusage-codex-sessions.json"
jq -r --arg p "$PROJECT" '.rows[] | select(.agent == "codex" and .project == $p) | .session' "$QA/sessions.json" | sort > "$QA/codex-$PROJECT.ids"
jq --rawfile ids "$QA/codex-$PROJECT.ids" '($ids | split("\n") | map(select(length > 0))) as $wanted | [.sessions[] | select(.sessionId | split("-") | .[-5:] | join("-") | IN($wanted[]))] | {sessions: length, total: (map(.totalTokens) | add), output: (map(.outputTokens) | add)}' "$QA/ccusage-codex-sessions.json"
jq --arg p "$PROJECT" '[.rows[] | select(.agent == "codex" and .project == $p)] | {sessions: length, total: (map(.tokens.total) | add), output: (map(.tokens.output) | add)}' "$QA/sessions.json"
```

**Verify:**

- [ ] Session counts match, and totals match or differ only by behaviors in the parity
  ledger, such as Codex compaction estimates that urollup excludes with a diagnostic.

## Phase 5: Hand-Summed Sessions

### 5.1 A Claude Session

Choose a recent main transcript of this repository with no `subagents` directory and no
records whose `sessionId` differs from the file name.

```bash
export SESSION_FILE=~/.claude/projects/"$CLAUDE_DIR"/<session-id>.jsonl
stem=$(basename "$SESSION_FILE" .jsonl)
jq -s --arg s "$stem" '[.[] | select(.type == "assistant" and .message.usage.output_tokens != null and ((.sessionId // $s) == $s) and (.isSidechain != true) and ((.message.model != "<synthetic>") or (.requestId != null)))] | group_by(.message.id) | map(sort_by(.message.usage.output_tokens, (.apiBlockIndex // -1)) | last) | {requests: length, uncached_input: (map(.message.usage.input_tokens // 0) | add), cache_read: (map(.message.usage.cache_read_input_tokens // 0) | add), cache_write: (map(.message.usage.cache_creation_input_tokens // 0) | add), output: (map(.message.usage.output_tokens) | add)}' "$SESSION_FILE"
"$UR" report --session "$SESSION_FILE" --scope self --format json --no-progress | jq '{requests: .totals.requests.owned, tokens: .totals.tokens}'
```

The `jq` program mirrors the Claude rule: one request per `message.id`, counting the
block record with the largest output, then the highest `apiBlockIndex`, then the last in
the file.

**Verify:**

- [ ] Request count and all four token classes match exactly.

### 5.2 A Codex Session

Choose a recent rollout of this repository without `forked_from_id` in its first line.

```bash
export ROLLOUT=<path-to-rollout.jsonl>
jq -sc '[.[] | select(.type == "event_msg" and .payload.type == "token_count" and .payload.info.total_token_usage != null)] | last | .payload.info.total_token_usage | {uncached_input: (.input_tokens - .cached_input_tokens), cache_read: .cached_input_tokens, output: .output_tokens, reasoning: .reasoning_output_tokens, total: .total_tokens}' "$ROLLOUT"
"$UR" report --session "$ROLLOUT" --scope self --format json --no-progress | jq -c '{tokens: (.totals.tokens | {uncached_input, cache_read, output, reasoning, total}), diagnostics: [.diagnostics[].code]}'
```

**Verify:**

- [ ] The token objects match exactly when urollup reports no
  `codex-counter-epoch-reset`, `codex-estimate-compaction`,
  `codex-estimate-context-window-fill`, `malformed-line` or `pending-tail` diagnostic.
- [ ] If one of those diagnostics appears, choose another rollout; those cases are
  covered by fixtures, where the last cumulative total intentionally differs.

## Phase 6: Resumed and Forked History

### 6.1 Find Resumed Claude Sessions

```bash
for f in ~/.claude/projects/*/*.jsonl; do
  stem=$(basename "$f" .jsonl)
  n=$(jq -c --arg s "$stem" 'select(.type == "assistant" and .sessionId != null and .sessionId != $s)' "$f" 2>/dev/null | wc -l | tr -d ' ')
  [ "$n" -gt 0 ] && printf '%s %s\n' "$n" "$f"
done | sort -rn | head -5 > "$QA/resumed.txt"
wc -l < "$QA/resumed.txt"
```

**Verify:**

- [ ] At least one resumed session exists; if none does, mark this phase **Blocked** and
  rely on the fixture and property tests.

### 6.2 Resumed Usage Is Neither Double Counted Nor Dropped

For one resumed file, find its original session, the foreign `sessionId` its replayed
records carry, and copy both transcripts into a scratch root under two naming orders.

```bash
export RESUMED=<resumed-session.jsonl>
export ORIGINAL=<original-session.jsonl>
work=$(mktemp -d "$QA/resume.XXXX")
mkdir -p "$work/forward/projects/p" "$work/reverse/projects/p"
cp "$ORIGINAL" "$RESUMED" "$work/forward/projects/p/"
"$UR" report --all --source "$work/forward/projects" --no-default-sources --format json --no-progress | jq -c '{requests: .totals.requests, total: .totals.tokens.total, copies: .coverage.copies_excluded}'
"$UR" report --session "$ORIGINAL" --scope self --format json --no-progress | jq '.totals.tokens.total'
"$UR" report --session "$RESUMED" --scope self --format json --no-progress | jq '.totals.tokens.total'
```

Then build the reverse order by renaming the resumed file so it sorts before the
original, rewriting only that file’s own `sessionId` values to the new name, and rerun
the first report on `$work/reverse/projects`. The fixture reproduction for `uro-sn1e` in
the plan uses the same transformation.

**Verify:**

- [ ] The two self totals sum to the combined total; the combined report has
  `copies_excluded` greater than 0 and no ambiguous requests.
- [ ] Forward and reverse orders give identical totals and request counts.
  This fails on the pre-plan engine and must pass after Phase 1.

### 6.3 Forked Codex Rollouts

```bash
for f in $(find ~/.codex/sessions -name 'rollout-*.jsonl' -mtime -30); do
  head -1 "$f" | jq -e '.payload.forked_from_id != null' > /dev/null 2>&1 && echo "$f"
done | head -3 > "$QA/forks.txt"
wc -l < "$QA/forks.txt"
```

For one fork and its parent, compare `"$UR" report --session <fork> --scope self`, the
parent’s self report, and a report with both `--session` flags.

**Verify:**

- [ ] Parent self total plus fork self total equals the combined total.
- [ ] The fork’s self total excludes its copied parent history: it is smaller than the
  fork’s last cumulative `total_tokens`, and `copies_excluded` is greater than 0.

## Phase 7: Live Current Session and Local Parity

### 7.1 Current Session

Run inside a live Claude Code session, and again inside a live Codex session if one is
available:

```bash
measure current "$UR" report --current --format json --no-progress
jq '.totals.requests' "$QA/current.json"
```

**Verify:**

- [ ] The command exits 0 in at most 2 s with a footprint of at most 128 MiB.
- [ ] Owned requests are greater than 0 and grow after another prompt in the same
  session.

### 7.2 Whole-History Parity With ccusage

```bash
make parity-local CONSENT_LOCAL_LOGS=1
jq '{totals, sessions, unexplained_residual, review_threshold_exceeded}' target/parity/local.json
```

The helper runs whole-history urollup `daily` and `sessions` once each and joins session
rows to ccusage on the native `session` field.

**Verify:**

- [ ] The target exits 0.
- [ ] Every token class in `unexplained_residual` is 0, or each nonzero residual is
  recorded with a bead.
- [ ] `review_threshold_exceeded` is false, and `sessions.urollup_only` and
  `sessions.ccusage_only` are explained.

## Phase 8: Record and Clean Up

### 8.1 Write the Dated Report

Create `docs/project/qa/qa-report-YYYY-MM-DD-full-history.md` with the commit, corpus
volume, wall times and footprints, invariant and determinism results, per-project and
hand-summed deltas, resumed and forked results, and parity residuals.

**Verify:**

- [ ] The report contains no paths, session IDs, prompts, custom model names, or project
  names other than `urollup`.
- [ ] This playbook’s status table and test results are updated.

### 8.2 Clean Up

```bash
rm -rf target/qa/full-history
git status --short
```

**Verify:**

- [ ] Only the intended report and playbook changes are present.

## Success Criteria

- [ ] Whole-history `sessions`, `daily` and `report --all` exit 0 within the wall-time
  threshold at no more than 512 MiB peak footprint.
- [ ] Totals agree across daily, report, sessions and project breakdowns.
- [ ] Output is byte-identical across reruns and worker counts.
- [ ] Four projects match ccusage or differ only by ledgered behaviors.
- [ ] Hand-summed Claude and Codex sessions match exactly.
- [ ] Resumed and forked history is neither double counted nor dropped, and results are
  invariant under file naming order.
- [ ] `report --current` works in a live session, and local parity has no unexplained
  residual.

<!-- This document follows common-doc-guidelines.md.
See github.com/jlevy/practical-prose and review guidelines before editing.
-->
