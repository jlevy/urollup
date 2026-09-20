---
title: PR Stack and Memory Audit
description: Published-head validation, reproduced findings, merge order, and the remaining 0.1 acceptance work.
date: 2026-09-19
status: Implementation fixes validated locally; final per-layer CI and milestone acceptance pending
---
# PR Stack and Memory Audit

The audit and follow-up review identified eleven implementation findings and four Cursor
planning findings. R1–R11 now have local fixes on their owning branches; the stack is
rebased and both intended coverage goldens have been reviewed.
The full integrated handoff gate passed; final per-layer CI remains before merge
approval. The whole-history memory and speed targets remain unproven on the updated
heads. Audit evidence is recorded in `uro-y7zm`; `uro-28fc` governs implementation and
validation. No maintainer decision has been accepted or silently closed.

## Stabilization Control

This is the governing review for the published stack.
`uro-28fc` tracks the response; `uro-y7zm` records the original audit.
The findings and head table below preserve the audit snapshot.
Append dated dispositions with the fixing commit, tests, and CI outcome; do not erase an
original finding when it is fixed.
A bead stays open until its acceptance conditions are demonstrated.

| Work | Beads | Completion evidence |
| --- | --- | --- |
| Decode admission | `uro-y1lp` | Early bounded refusal, cancellation, successful accounting parity |
| Compact representations | `uro-ww39`, `uro-u6in` | Multi-source provenance and full-domain sequence regressions |
| CI and RAM probing | `uro-a8fk`, `uro-qvy0` | Measured CI workload and completed cross-platform tests |
| Independent review | `uro-nncx`, `uro-syzt`, `uro-jw7u`, `uro-dhek` | Scope and exact-head verdicts; every finding has a disposition |
| Performance acceptance | `uro-zrr0`, `uro-lsaz`, `uro-a3fo` | Representative 512 MiB / 10 s evidence, with documented methodology |
| Final consistency and integration | `uro-erqo`, `uro-knnz` | Specs match implemented behavior; Cursor docs integrate separately |
| Milestone acceptance | `uro-d36a`, `uro-ky6c`, `uro-n8h5` | Consented aggregate validation and explicit maintainer decisions |

## Status Addendum: 2026-09-20 UTC

R1–R11 were implemented and cherry-picked onto their owning local branches.
The local #10 tree committed as `d16542e` (audit head `827dad2` plus R1) passed the
complete `make check`, including all 30 negative gate probes.
A subsequent comment clarification does not change the tested code.
R4 (`6a5d0d6`) was added afterward; that complete check is not evidence for the
restacked tree or the new hosted scale jobs.

The earlier disk-space interruption has been resolved.
The two R11 golden updates were reviewed and committed as `2f9afa3`. Current work is
propagation through #8/#10/#11/#12 and fresh integrated checks and CI. No new
whole-history measurement or private-log access is claimed.
The six policy decisions and the unverified-shape evidence remain pending; see
[Pending Maintainer Decisions and Evidence](#pending-maintainer-decisions-and-evidence).

## Restacked Implementation Heads

These revisions contain the final implementation fixes, before the evidence-only review
update on #12. The entire #12 tree equals the fully tested `296f12b` tree after the last
restack; the subsequent commit updates only this review document.
Final CI is reported on each PR and its linked check runs.

| PR | Branch | Validated code revision |
| --- | --- | --- |
| #4 | `milestone-0.1` | `2f9afa38acbf` |
| #8 | `codex/v0.1-terminal-ux-acceptance` | `4a8ef34cd284` |
| #10 | `scalable-ingestion` | `b390982f0efb` |
| #11 | `ingest-compact` | `e79980583d87` |
| #12 | `ingest-capacity` | `8ad8f94d350a` |

The final propagation repaired two standalone-layer compilation gaps: #8 needed a
mutable binding when consuming identity keys, and #10 needed its new coverage tests
ported to its own API. Standalone #8 passed 156 core tests and all-target core Clippy;
#10 passed 198 core tests and the same Clippy gate.
These corrections leave the tested top-layer tree unchanged.
Independent review found no blocker in the conflict resolutions or configured admission.
PR11 passed 215 core tests before PR12 replay; the new PR12 test covers 18 early-refusal
and nine exact-capacity success cases, and all 11 CLI process tests pass.

## Local Finding Dispositions

All rows describe implemented fixes validated locally; final per-layer CI is tracked
separately. Commit IDs below preserve the original fixing commits before restacking,
rather than naming the current published heads.

| Finding | Owning layer and local commit | Implemented behavior and regression evidence |
| --- | --- | --- |
| R1 | #10 `d16542e` | Shared admission during decode rejects over-budget retained rows and propagates cancellation; early-refusal and successful-accounting checks passed in the full local #10 gate. Row admission remains distinct from process RSS. |
| R2 | #11 `2827da0` | Remaps Claude malformed-record evidence through the final source table before cloning snapshots; multi-source/worker regression covers both manifest and source artifacts. |
| R3 | #11 `2827da0` | Nine-byte optional big-endian sequence preserves every `u64` and numeric order; boundary, property and adapter-total regressions cover the maximum value. |
| R4 | #10 `6a5d0d6` | Explicit Ubuntu/macOS scale jobs wait on supply-chain, preserve pipeline failures and archive logs; config-contract regression and five mutation probes pass. Hosted workload evidence is pending. |
| R5 | #12 `082dbd7` | Explicit budgets avoid RAM probing; native platform queries replace unbounded helper processes. Probe-selection and platform-boundary tests accompany the fix; completed Windows CI remains required. |
| R6 | #4 `6b21587` | Direct Node launch of pinned tryscript preserves literal paths; 28 harness tests pass, and enabling shell interpolation makes the spaced/metacharacter child-process regression fail. |
| R7 | #8 `7f82a56` | Local QA entrypoints emit fixed diagnostics for OS failures; 14 parity tests pass, including two synthetic executable-failure regressions that read no agent logs. |
| R8 | #4 `bb1b7b6` | Canonicalizes keys before reread comparison; order/duplicate-key regression preserves identical-observation classification. |
| R9 | #4 `f5a9685` | Reset replaces the prior counter baseline; partial-field tests distinguish a new epoch from omitted fields within one epoch. |
| R10 | #4 `1d03a76` | Oversized unfinished tails remain pending without advancing past the snapshot; plain/compressed and multibuffer regressions cover the boundary. |
| R11 | #4 `4a49fc7` | Missing-usage count derives from selected counted requests and completeness reuses accounting semantics. Public-report regressions and workspace Rust/clippy checks pass; the two expected golden coverage changes were reviewed and committed as `2f9afa3`. |

R11 corrects a counter/selection contract.
It does **not** decide that copies with missing originals should make accounting
incomplete. Under current accounting semantics, `progress-nested-subagent` and
`legacy-subagent-prefix` change missing usage from 1 to 0 and completeness from false to
true. Copies remain excluded and diagnosed.
If the maintainer accepts an explicit missing-original coverage policy (`uro-xpd0`),
implement that as a separate accounting partial reason or gap, keeping the
counted-request missing-usage counter at zero.
The recommendation below remains a proposal.

Typed-sidecar follow-up `5103373` (#11; independent patch `aaa906f`) retains document
acceptance semantics without retaining a JSON tree.
Independent inspection found no blocker in it or the compact fixes.
This refactor is not evidence that the remaining whole-history performance gap has
closed.

## Published Heads and Review Coverage at the Audit Snapshot

| PR | Head | Base | CI snapshot | Review verdict |
| --- | --- | --- | --- | --- |
| [#4](https://github.com/jlevy/urollup/pull/4) | `c08986d` | `main` | 13 checks passed | Full implementation review remains open |
| [#8](https://github.com/jlevy/urollup/pull/8) | `efa5ef8` | `milestone-0.1` | 13 checks passed | Full terminal, privacy, and source-path review remains open |
| [#10](https://github.com/jlevy/urollup/pull/10) | `827dad2` | `codex/v0.1-terminal-ux-acceptance` | 13 checks passed | Changes required: early capacity enforcement and CI scale wiring |
| [#11](https://github.com/jlevy/urollup/pull/11) | `d44476b` | `scalable-ingestion` | 13 checks passed | Changes required: source evidence and sequence boundaries |
| [#12](https://github.com/jlevy/urollup/pull/12) | `d446732` | `ingest-compact` | 12 passed; Windows goldens still running | Changes required: RAM-probe behavior and complete Windows validation |
| [#14](https://github.com/jlevy/urollup/pull/14) | `6c3fc36` | `main` | No checks; local formatting passed | Separate docs review and integration follow-up remain |

GitHub had zero formal reviews and zero inline review threads on all six PRs at the
start of the audit. PR #14 had one CI-retrigger comment.
No earlier independent full implementation review was located in the repository or
tracking record.

This audit inspected the current PR descriptions, bases, exact heads, review history, CI
jobs, memory-related diffs and their surrounding code, and recorded benchmark and
acceptance evidence.
It traced capacity enforcement, worker-result retention, source-index remapping, compact
token measures, sequence packing, interned names, identity grouping, and the scale
harness. It ran synthetic reproductions and the local handoff gate.
The original audit was focused rather than a line-by-line review of the entire 297-file
base implementation.
The subsequent independent passes are recorded below; `uro-nncx` remains open for final
integrated-head verdicts and finding dispositions.

## Findings

### R1: Capacity enforcement occurs after unbounded decode retention

**High; `uro-y1lp`; introduced in #10, exposed as a RAM budget in #12.** Both adapters
collect all worker results before normalization.
The only observation ceiling check is at
`crates/urollup-core/src/ledger/reconcile.rs:505`, after the full observation input
already exists. The relevant callers are `adapters/claude_project.rs:550` and
`adapters/codex_rollout.rs:579`.

A 32 MiB, seed-1 synthetic corpus contains 21,253 usage records.
A release report with `--max-rows 1` still constructed 19,757 Codex observations before
refusing.
It reached 35.44 MiB peak physical footprint and 37.14 MiB maximum RSS, exiting
1 after 0.73 s. This is a small, bounded reproduction of the late check, not a claim
that a one-row budget should cover the process’s fixed startup cost.
Larger inputs can exhaust memory before the intended diagnostic occurs.
The row-shell calculation also excludes payloads, intern tables, requests, other ledger
tables, and the other agent’s retained ledger.

**Fix:** enforce a shared per-agent budget before retaining decoded observations,
including pending representations, and propagate cancellation and capacity errors.
Document precisely that a row budget is not a process RSS limit.
Test early refusal, worker-count behavior, and unchanged successful accounting.

### R2: Claude malformed-record references retain source index zero

**High; `uro-ww39`; #11.** The reader now emits local `EvidenceRef.source = 0`. Claude
normalization builds a global source table but does not remap
`manifest.entries[].first_malformed` before cloning those snapshots into source
artifacts (`crates/urollup-core/src/adapters/claude_project.rs:980`, `:1030`). Codex
performs this remap at `codex_rollout.rs:1165`.

Two synthetic Claude files, each with a valid record and malformed second line, produced
only **1/2** malformed references resolving to the correct source ID. The bug corrupts
provenance; it does not change the aggregate malformed-record count.

**Fix:** stamp these references through the frozen source table before copying the
manifest. Test both manifest entries and source artifacts across multiple sources and
worker counts.

### R3: Packed sequence loses the maximum accepted integer

**Medium; `uro-u6in`; #11.** `NativeSequence::new` at
`crates/urollup-core/src/ledger/reconcile.rs:68` returns `None` for `u64::MAX`. The
Claude adapter silently uses that result at `adapters/claude_project.rs:1397`, although
the typed decoder accepts that integer.

For two revisions of one synthetic request with equal output of 10, block indices
`[18446744073709551615, 1]`, and input counts `[100, 1]`, the report selects 1 input
token and totals 11. The documented largest-block-index tie-break selects 100 input
tokens and totals 110. This is a boundary regression, not evidence that ordinary
provider logs use such a large block index.

**Fix:** preserve the accepted domain and ordering, or explicitly diagnose a narrower
supported dialect domain.
Test missing, zero, maximum-minus-one, maximum, reversed arrival order, and final
totals.

### R4: GitHub CI does not execute the synthetic scale workload

**High; `uro-a8fk`; #10.** `Makefile:55` includes `scale-gate` in `make test`, but
`.github/workflows/ci.yml` calls the individual Rust, QA-tool, golden and result
commands. No workflow invokes `make test`, `make check`, `make scale-gate`, or
`scripts/check-scale.py`. QA unit tests exercise the scale gate’s logic, and the
failing-test probe exits before reaching the scale workload.
Neither measures the release engine in CI.

**Fix:** add an explicit POSIX release scale step behind the supply-chain gate and
protect that wiring with a regression check.
Record its measured results.
Keep the small synthetic regression gate distinct from whole-history acceptance.

### R5: RAM detection can block even when no RAM query is needed

**High; `uro-qvy0`; #12.** `crates/urollup/src/cli.rs:370` always evaluates
`physical_memory_bytes()`, including when an exact `--max-rows` or byte budget was
supplied. Both platform helpers use unbounded `Command::output`
(`ledger/capacity.rs:309`, `:318`). A synthetic three-second `sysctl` delayed an
explicit `--max-rows 1` invocation to 4.16 seconds.
A stuck helper can prevent the fallback from ever being reached.

Separately,
[the Windows CI job](https://github.com/jlevy/urollup/actions/runs/35478104257/job/105990831189)
was still in CLI goldens at the audit snapshot, having entered that step at 2026-09-20
00:13:45 UTC. Its Rust tests had passed, but goldens and fixture results had not
completed and the job log was unavailable.
This historical stall has no proven PowerShell root cause; the updated head needs its
own completed Windows evidence.

**Fix:** resolve explicit budgets without probing the host, bound and reap any helper or
use a reviewed native facility, preserve read-only HOME behavior, and obtain a completed
green Windows run. Add tests with injected slow and failed probes.

## Follow-up Review Findings

These findings extend the original audit.
They were found by independent review of the published stack and are governed by
`uro-28fc` alongside R1–R5.

### R6: Windows golden launcher interpolates paths through a shell

**Medium; `uro-y2x9`; #4.** `scripts/run-golden.mjs` launched the Windows `.cmd` shim
with `shell: true`. Spaces split executable/session paths and shell metacharacters are
interpreted. **Fix:** launch the locked `tryscript/dist/bin.mjs` with `process.execPath`
and `shell: false`. A real-child regression uses paths containing spaces and `&` and
checks argument preservation; enabling shell interpolation reproduces the failure.

### R7: Local QA operating-system failures expose private paths

**Medium; `uro-onrq`; #8.** The `tests/parity/local_aggregate.py` and `local_diff.py`
entrypoints let operating-system exceptions escape as tracebacks, including configured
executable and output paths.
**Fix:** convert these boundary failures into fixed, path-free diagnostics and exit 1.
Synthetic nonexecutable paths reproduce the leak without reading agent logs.

### R8: Key order changes reread classification

**Medium; `uro-p1o7`; #4.** `RequestObservation.keys` accepts arbitrary order, but
`dedupe_rereads` compared records before `resolve_identities` canonicalized keys.
Identical evidence and values with keys `[A, B]`, `[B, A]` or duplicated keys produced a
false `ConflictingReread`. **Fix:** sort/deduplicate keys before comparing rereads.

### R9: A partial counter reset retains the previous epoch

**High; `uro-hnd0`; #4.** `RunningTotal::observe` merged absent categories from the old
baseline even after resetting the counter epoch.
The sequence `(input=100, output=100)`, `(input=5, output=absent)`,
`(input=10, output=1)` falsely resets twice and adds 10 instead of 5 input tokens on the
final update. **Fix:** replace the baseline on reset; merge omitted categories only
within the same epoch.
The regression checks both cases.

### R10: An oversized unfinished line crosses the snapshot boundary

**Medium; `uro-x6np`; #4.** `sources/reader.rs` classified an oversized unterminated
tail as a complete oversized record, then added a nonexistent newline.
With a four-byte record limit, `{}\n12345` reported cutoff 9 for eight input bytes and
lost its pending tail.
**Fix:** preserve the pending tail and its true offset while recording the size failure.
Plain and compressed regressions cover the size boundary and a multibuffer tail.

### R11: Report completeness disagrees with selected counted requests

**Medium; `uro-6q5d`; #4.** The report field `requests_without_usage` promises counted
requests but copied a global reconciliation count that includes uncounted copies and
unselected threads. `coverage_summary` also independently reconstructed completeness
instead of using the selected accounting result.
**Fix:** derive missing usage from selected counted requests and reuse accounting
completeness. Copy exclusion and provenance diagnostics remain.
Any policy to make missing originals an explicit coverage gap must use that concept,
rather than misclassifying copies as counted requests without usage.

## Cursor Planning Review Findings

A bounded independent review of PR #14 at `6c3fc36` found four additional planning
issues. Commit `c8befd9` fixes them on the separate `cursor-dialect` branch; all four
changed files passed pinned Flowmark checks and `git diff --check`. No private data was
re-surveyed.

| Finding | Bead | Disposition |
| --- | --- | --- |
| R12: plan contradicts confirmed database-input ordering | `uro-g7da` | Retain product Phase 3 ordering; an earlier named database adapter requires an explicit exception to Decision 20. |
| R13: JSONL coverage aggregates do not reconcile | `uro-6unm` | Preserve and qualify the historical counts, identify the unexplained difference, and forbid an exact missing-session claim without evidence. |
| R14: current model selection can relabel historical usage | `uro-oxnw` | Attribute each measurement using its own model evidence; leave missing historical attribution unknown. Current picker/list state remains metadata. |
| R15: current-session wording describes unimplemented behavior | `uro-1vfu` | Label Cursor detection and its unsupported-dialect diagnostic as planned; require an exact signal before either is implemented. |

PR #14 still needs its implementation-dependent integration check (`uro-knnz`). Five
distinct targets require the implementation stack: golden README, discovery source,
scalable/publishing specs, and the product golden-test anchor.
The known product-plan conflict must preserve both acceptance goals and the Cursor
pointer. Its current `main` base has no workflow; local docs checks do not constitute a
hosted CI pass.

## Independent Review Scope

Three Astra passes examined the published stack: accounting/identity/reconciliation;
readers/adapters/selection/CLI; and CI/harness/privacy/portability.
The review checked resource/error boundaries, source identity and cutoffs, cumulative
epochs, deduplication, copy and ambiguity rules, compact representations, native
RAM-query ABIs, hermetic fixtures and gate execution.
The compact and typed-sidecar fixes received a second independent inspection.
No extra blocker was found in those two patches.

This is risk-based review with executable regressions, not a claim that every line of
every historical PR commit was inspected.
Final integrated validation and the measured whole-history acceptance gates remain
separate requirements.

## What the Existing Evidence Establishes

- At the original audit snapshot, `make check` on head `d446732` exited 0, including all
  30 negative gate proofs, Rust 1.98 default/no-default tests, Rust 1.85 MSRV tests,
  format/lint/docs, fixture/golden/parity checks, supply-chain/dependency audits and the
  local scale gate. This is historical local macOS evidence, not validation of the
  subsequent fixes or their Windows behavior.
- That fixture validation covered 29 cases; the CLI goldens cover 30 sessions and 239
  blocks. Pinned ccusage parity covers 290 comparisons and 403 explained differences.
- Worker-count tests compare one worker with 2, 3, 8 and 64 workers, both per fixture
  and with each dialect’s fixtures combined.
  They also check first-error selection.
- Typed Claude and Codex parser tests compare against document-parsing oracles, with
  2,000 generated cases each.
  Compact measures retain values beyond `u32`, and tests exercise their value, ordering
  and hashing behavior.
- `EvidenceRef` uses explicit source/offset/length order, independent of field layout.
  Its 16-byte representation and 224-byte observation rows have compile-time guards.
- The original local scale run passed: padded-content peak difference 1.91 MiB, fitted
  slope 2,565 B/usage record with a 3.45 MiB intercept, and a 32 MiB daily corpus
  containing 21,253 usage records under the watchdog at 52.7 MiB sampled RSS.

These checks are useful regression evidence.
They do not prove the maintainer corpus fits within 512 MiB, or establish an integrated
verdict for the updated stack.
The newer #10 gate result and its exact scope are recorded in the status addendum.

The historical standing whole-history measurement is **653 MiB / 17.3 s**, with a repeat
at 661 MiB / 18.6 s; Codex-only is **586 MiB / 13.2 s**. A later quiet profile recorded
648 MiB / 18.2 s and 573 MiB / 14.3 s. These are recorded earlier measurements, not new
measurements of this audit’s head.
Those historical runs implied roughly 141 MiB and 7–8 seconds of remaining gap; they
must not be presented as measurements of the updated heads.
No private logs were read for this audit.

## Stabilization Measurement Evidence

The 32 MiB, seed-1 synthetic reproduction contains 21,253 usage records across 4,012
files. At integrated implementation `296f12b`, `report --all --max-rows 1` exits 1 with
**2** request observations, a one-row budget label and empty stdout.
It records 0.36 seconds and 10,469,760 bytes (9.98 MiB) peak physical footprint via
macOS `/usr/bin/time -l`; the separate 50 ms process-group RSS sampler saw 5.03 MiB. The
original audit retained 19,757 Codex observations before refusal and recorded 35.44 MiB
physical footprint. These bounded, loaded local runs establish early refusal; they are
not whole-history performance acceptance or a zero-overhead row-budget claim.

The new hosted synthetic jobs on `296f12b` both passed:

| Platform | Padded-content peak delta | Fitted bytes/record | Daily watchdog sampled peak |
| --- | ---: | ---: | ---: |
| [macOS 15](https://github.com/jlevy/urollup/actions/runs/35490373538/job/106024196668) | +0.23 MiB | 1,798 | 36.4 MiB |
| [Ubuntu 24.04](https://github.com/jlevy/urollup/actions/runs/35490373538/job/106024196669) | +0.79 MiB | 1,549 | 37.4 MiB |

The local gate on that tree passed with +1.78 MiB padded-content delta, a 2,445 B/record
fit and 50.9 MiB daily sampled RSS. These metrics use different OS accounting and
sampling; do not treat them as interchangeable peak-footprint measurements.
No result here claims that the maintainer corpus meets 512 MiB or 10 seconds.

## Design Assessment and Remaining Limits

Compact in-place rows and one global reconciliation preserve Claude’s cross-session
ownership semantics.
Per-family reconciliation is not an interchangeable optimization.
The row layout work has meaningful invariance tests, but R2 and R3 show why end-to-end
boundary cases must accompany representation changes.

Process-global name and overflow-measure intern tables deliberately retain distinct
values until process exit.
Provider-limit JSON can have high cardinality.
This is a material constraint for repeated library ingestion or a future server; small
row sizes do not establish bounded lifetime or total memory.
Scope or measure those tables before claiming support for a long-lived process.
Linux RAM detection reads host `MemTotal`, not a container memory limit.

The 512 MiB target should be pursued from measurements of live allocation ownership and
representative record density.
Do not repeat the reverted field/ID relocation, primitive-number, mimalloc,
zero-copy-acceptance, or larger-read-window experiments.
`uro-nzo1` is held. `uro-lsaz` owns profiling; the sidecar implementation tracked by
`uro-a3fo` is now prepared locally.
Its integration and representative measurements remain separate from implementation, and
neither workstream yet proves the full target.

## Remaining Work and Merge Order

1. Preserve the reviewed R11 golden update `2f9afa3` while propagating the local fixes
   bottom to top: **#4 → #8 → #10 → #11 → #12**. Record final per-layer SHAs after
   rebasing; the historical published-head table is not current integration evidence.
   No merge is authorized by a merely green ancestor.
2. The integrated checks passed.
   Obtain completed CI on each final head, including Windows goldens/results and the new
   Ubuntu/macOS scale jobs.
   Preserve negative-probe failures and workload artifacts.
   The passed #10 gate on `d16542e` does not replace this step.
   Record exact-head independent verdicts and close findings only when their acceptance
   evidence is complete (`uro-nncx`, `uro-syzt`, `uro-jw7u`, `uro-dhek`, governed by
   `uro-28fc`).
3. Complete the documentation consistency pass (`uro-erqo`) against the final behavior:
   row-budget versus process-memory limits, actual CI wiring, intern-table lifetime and
   dated measurement provenance.
   Follow the [technical disposition](#pending-maintainer-decisions-and-evidence) below.
4. Demonstrate the representative **512 MiB and 10-second** gates on the accepted head
   (`uro-zrr0`; `uro-n1cp` waits on it).
   Small synthetic CI workloads are regression checks, not whole-history acceptance.
   Then run separately consented G1 (`uro-d36a`) and full-history QA (`uro-ky6c`),
   retaining aggregates only, confirming one/eight-worker equality and explaining parity
   residuals. No additional local-log access is granted by this review.
5. Obtain explicit acceptance or documented deferral of the six maintainer decisions
   (`uro-mzvt`, `uro-wt2q`, `uro-je0v`, `uro-xpd0`, `uro-c7ro`, `uro-01xj`) and a
   disposition for the unverified Claude shapes (`uro-89s7`). The
   [recommendations](#pending-maintainer-decisions-and-evidence) distinguish policy from
   missing evidence. No response has been received; none is silently settled by a fix.
6. Merge the implementation stack in order once its applicable review/CI conditions
   pass, keeping milestone acceptance explicitly dependent on the unresolved
   performance, consented QA and maintainer conditions.
   Stack #9 records the five related PRs; refresh local/remote metadata after
   propagation rather than relying on the audit snapshot.
7. Integrate separate Cursor documentation #14 after the implementation stack
   (`uro-knnz`). The known product-plan conflict must retain both End-to-End Acceptance
   Goals and the later-Cursor pointer.
   Recheck links, research claims, scope and docs CI against its new base.
   Cursor implementation stays outside 0.1.
8. Only after milestone acceptance, proceed with packaging `uro-30ef`: native builds and
   smoke tests (`uro-uj8w`), publishing workflow (`uro-080d`), provenance/version checks
   (`uro-co0m`), release docs (`uro-2025`) and rehearsal/publication/installation
   validation (`uro-vboa`).

## Pending Maintainer Decisions and Evidence

The following are recommendations for maintainer acceptance, not recorded decisions.
No decision bead was closed.
Review used repository code, design, synthetic fixtures and committed goldens; no
private logs were read.
These choices do not establish the 512 MiB or 10-second whole-history acceptance gates.

- **`uro-mzvt` — Reserve a config override; retain the harness variable.** Reserve
  `UROLLUP_CONFIG_DIR` for the directory containing `sources.yaml` and `prices.yaml`,
  taking precedence over platform config-directory APIs.
  Keep `UROLLUP_BIN` explicitly harness-only; the CLI must ignore it.
  This avoids changing every golden for a naming preference.
  Current source discovery does not load these config files, so there is no present
  Windows config-file leak to fix.
  The harness can create an isolated config directory and set the reserved override now;
  when loading is implemented, require a Windows test proving a stray platform-default
  config is ignored. Redirecting `APPDATA` alone is insufficient for shell APIs.
  Evidence: `scripts/golden-env.mjs`, `scripts/run-golden.mjs`, source discovery, design
  §2.1 and §2.5. **Choice needed:** accept this namespace reservation, or request a
  harness-variable rename.
  Configuration loading itself stays out of scope.

- **`uro-wt2q` — Retain conservative conflict handling for 0.1.** Keep each observation
  in a conflicting shared-key group artifact-local, count one candidate, and expose the
  rest as unresolved. This can undercount agreeing block subgroups, but changing it
  requires a specified equivalence rule, not merely a different grouping loop.
  A future partition rule must settle missing invariant fields, session/model
  disagreement and whether distinct partitions are proven requests or still candidates.
  Evidence: `reconcile.rs` at `conflicting_fields` and
  `a_shared_key_with_disagreeing_invariants_is_ambiguous_not_merged`; design §3.6 and
  the Candidate “Conflicting Shared Keys” decision.
  **Choice needed:** confirm the conservative 0.1 policy or authorize a separately
  specified partition policy.
  No partition implementation is justified solely by the current two-observation test.

- **`uro-je0v` — Confirm `cache_write_unspecified`.** Preserve a third disjoint category
  for recorded writes whose duration is unknown.
  Inferring five minutes changes evidence into a pricing assumption.
  The current `TokenMeasures` representation already separates five-minute, one-hour and
  unspecified writes, and sums them without double counting.
  Claude records lacking a breakdown and Codex cache-write values use the unspecified
  category. Evidence: `ledger/tokens.rs`, `claude_usage`, and the cache-creation fixture.
  **Choice needed:** confirm the name and semantics before the 0.2 summary contract; the
  technical follow-up is documenting them in design §4.1. Future pricing can label an
  explicit default assumption separately; it must not rewrite measured tokens.

- **`uro-xpd0` — Keep copies uncounted and make missing originals explicit.** Retain
  `Counting::CopyOnly`, evidence and the missing-original diagnostic.
  Do not promote copied usage into an observed request.
  Before R11, reports became incomplete by incorrectly including copies in a counter
  documented as counted requests without usage.
  R11 removes that mismatch and preserves existing accounting semantics: copies alone do
  not add a partial reason.
  That repair is distinct from the proposed missing-original policy.
  **Choice needed:** confirm or reject the recommendation that missing originals should
  make coverage incomplete.
  **Technical follow-up only after acceptance:** add a dedicated accounting partial
  reason or `CoverageGap`, propagate it through selected totals and reports, and test it
  while keeping counted missing usage at zero.
  No proposed copy policy has been implemented or accepted.

- **`uro-c7ro` — Keep the newline boundary absolute in 0.1.** An explicit source path or
  an old modification time does not prove no writer exists.
  Treat an unterminated final record as pending even when its bytes currently parse.
  This matches the reader, its pending-tail tests, design §2.2, and the capture rule in
  §2.5 that unfinished lines are never captured.
  **Choice needed:** confirm this conservative boundary.
  Supporting finalized captures without a newline should be a separate explicit
  import/finalization contract with source-immutability evidence; no idle heuristic is
  needed for 0.1.

- **`uro-01xj` — Confirm the three fixture interpretations together.** For conflicting
  cache-write counts, retain the flat count as the additive total, preserve both native
  values and the diagnostic, and classify the counted amount as unspecified duration.
  `claude_usage` already does this, avoiding duration buckets whose sum disagrees with
  the chosen total. For an orphaned nested progress copy, retain zero counted usage and
  apply a missing-original coverage policy only if accepted under `uro-xpd0`. For the
  synthetic quota error, retain one synthetic event rather than a provider request, and
  preserve its error text as a limit observation with unknown window details.
  The quota fixture cites a pinned public parser and keeps native evidence; accepting
  this interpretation does not justify inventing utilization or window length.
  **Choice needed:** confirm these policies.
  **Technical follow-up:** synchronize design §3.1/§4.1, fixture notes and any changed
  assertions in one commit; remove “open question” wording only after acceptance.
  Existing golden success proves implementation consistency, not that a policy has been
  approved.

- **`uro-89s7` — Keep the provider-shape claims unverified.** The exact `/btw` filename
  label `aside_question` and the association between missing `requestId` and
  Bedrock-style `msg_bdrk_` IDs remain modeled examples.
  Their fixture READMEs and `expected.json` notes say so explicitly.
  The fixtures still test useful general behavior: sidechain copies do not double count,
  and response IDs can identify requests without a request ID. **Evidence needed:** a
  versioned public upstream test or documented, sanitized observation establishing each
  exact shape. No maintainer preference or passing synthetic test supplies that evidence.
  Keep the bead open, retain the modeled labels, and avoid claiming provider-specific
  verification in release material.
  If acceptance permits deferral, record that deferral explicitly.

- **`uro-erqo` — Correct present-tense claims now; measure only accepted heads.** This
  is technical consistency work, not a product-policy choice.
  Once the stack is integrated, synchronize `AGENTS.md`, `README.md`, design §7, the
  scalable-ingestion plan, `scale-measurement.md`, and the scale scripts.
  Replace present-tense fixed 2 GiB claims with the implemented row-budget policy,
  including its fallback and flags; distinguish that admission budget from a
  process-memory ceiling.
  Keep historical measurements labeled with their original commit and conditions.
  Record fresh `sessions`, `daily` and `report` results against exact accepted SHAs,
  separating sampled watchdog RSS, OS maximum RSS and macOS physical footprint, plus
  quiet/loaded runs and synthetic/real corpora.
  CI scale execution proves a small synthetic regression bound, not whole-history
  acceptance. Real-history measurements require the separately authorized acceptance
  workflow; this review performed none.
  Also document process-lifetime interned storage (`ledger/names.rs`) and track
  cardinality/lifetime before repeated library or server ingestion; “per corpus” and “a
  few dozen names” must not substitute for measured bounds.
  Keep the bead open until both documentation and current-head evidence are complete.

<!-- This document follows common-doc-guidelines.md.
See github.com/jlevy/practical-prose and review guidelines before editing.
-->
