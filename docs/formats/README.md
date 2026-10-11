# Agent Log Format Contracts

These contracts describe the fields urollup relies on to count usage, not every field an
agent might write. Each supported adapter has one maintained reference with the same
sections: support and evidence, discovery and framing, record mapping, accounting rules,
failure behavior, and regression coverage.

| Agent and dialect | Runtime support | Contract |
| --- | --- | --- |
| Claude Code `claude-project` | Persistent transcripts and subagent metadata | [Claude Code](claude-code.md) |
| Codex `codex-rollout` | Persistent and archived rollouts, including compressed files | [Codex](codex.md) |
| Claude stream and Codex exec stream | Researched; not current runtime adapters | [Design source rules](../urollup-design.md#34-dialect-reconciliation-rules) and [source review](../project/research/research-2026-09-14-agent-tool-source-reviews.md) |
| Pi and Gemini CLI | Planned for Phase 2, outside the frozen Codex/Claude alpha | [Product plan](../project/specs/active/plan-2026-09-13-urollup-cli-and-web.md) |

[Accounting incidents](accounting-incidents.md) explains the confirmed bugs, their
causes, and the tests that prevent recurrence.
It distinguishes bugs in urollup from format hazards and incomplete features.

## Evidence and Support Boundaries

A documented field is not a promise that every producer version emits it.
Preserve these distinctions when changing a contract:

- **Native evidence:** a pinned producer implementation, published schema, or observed
  shape. Private observations stay local; public examples must be synthetic.
- **Accounting policy:** urollup’s interpretation of identity, ownership, revisions,
  copies and incomplete coverage.
  Link the design and any unresolved decision.
- **Implementation:** the adapter and tests that implement that policy.
  A researched stream or a fixture does not establish universal version compatibility.
- **Acceptance:** the exact revision and checks that establish a release’s behavior.
  Passing fixtures do not establish full real-history parity or memory safety.

Which accounting fixes have merged and which remain open is recorded once, in the
[release readiness record](../project/specs/active/plan-2026-09-16-first-release-publishing.md#current-readiness-and-critical-path).
Requests retain their recorded pricing context, but no reviewed rate table, price
matching or cost report exists yet (`uro-wuby`, `uro-neii`), so nothing here implies CLI
cost reports.

## Shared Reader Contract

The [reader](../../crates/urollup-core/src/sources/reader.rs) captures a per-file
cutoff; later appends belong to the next snapshot.
JSONL, `.jsonl.zst` and `.jsonl.gz` inputs use the same decoded-record accounting.
A plain file and its compressed twins are one logical source, read plain first, then
zstd, then gzip, and never summed as independent histories.
A twin whose first record differs is reported as a different, unread source, and one
whose first record cannot be read is reported as unreadable
([design §2.2](../urollup-design.md#22-snapshot-boundary)).

Malformed records, pending unterminated tails, oversized records, truncation,
replacement and missing files are distinct conditions.
Preserve their manifest/coverage behavior; do not reinterpret a failed read as an empty
successful history. See the reader tests for the exact disposition of each condition.
A source whose read failed or that changed under the snapshot (an oversized record, a
damaged or truncated compressed stream, an unreadable or mismatched twin, an I/O error,
or a file that vanished, was replaced, truncated or rewritten) makes coverage partial
and is counted once in its agent’s `source-incomplete` diagnostic, whose detail names
each kind of loss; one unreadable file does not stop the report.
Interior malformed lines and an unterminated tail are reported as `malformed-line` and
`pending-tail` and leave coverage complete.
Unknown native counters remain unknown, not observed zeros.

Thread identity, source identity and request identity are different.
File paths locate evidence; a file is not a request, and moving or copying history must
not create billable work.
The [ledger](../../crates/urollup-core/src/ledger/reconcile.rs) reconciles originals and
copies before the [accounting layer](../../crates/urollup-core/src/accounting/totals.rs)
applies session selection and totals.
A request’s pricing context (provider, service tier, speed and inference geography)
comes from its original records only: originals fill each other’s missing fields,
originals that record different values produce `conflicting-pricing-context`, and copies
never contribute ([design §3.1](../urollup-design.md#31-entities)).

## Maintaining a Dialect

For an adapter change, the implementation PR must:

1. Update the affected field/rule here and identify the producer revision or evidence
   source. Label inferred behavior and unresolved cases; do not invent a version range.
2. Add a synthetic regression with expected totals, ownership and coverage.
   Include duplicate/replay forms, missing or invalid fields, and parent-present/absent
   cases where applicable.
   Test one/eight-worker and source-order equivalence when ownership or reconciliation
   changes.
3. Update an incident entry for a confirmed miscount: triggering shape, violated
   assumption, consequence, fix status, bead and regression.
   Never use private values.
4. Check the typed decoder against its document-parsing oracle, then run fixture result
   checks and [CLI goldens](../../tests/golden/README.md).
   Review changed expectations; regenerating a golden does not prove a new total is
   correct.
5. Run `make check`, review the layer’s diff, and record hosted CI at the published
   head. Run the [full-history QA playbook](../../tests/qa/full-history-rollup.qa.md) for
   alpha acceptance, retaining private evidence outside the repository.

The adapter maintainer owns its contract and regression links.
The design owns shared accounting policy; the dated research owns historical source
evidence. Link those documents instead of copying their entire contents or maintaining
another field list. Track this documentation work as `uro-hlas`.

<!-- This document follows common-doc-guidelines.md.
See github.com/jlevy/practical-prose and review guidelines before editing.
-->
