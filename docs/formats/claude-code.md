# Claude Code Transcript Contract

## Support and Evidence

Runtime dialect: `claude-project`. The
[adapter](../../crates/urollup-core/src/adapters/claude_project.rs) and
[typed decoder](../../crates/urollup-core/src/adapters/claude_project/line.rs) read
persistent transcripts and subagent metadata.
Claude streaming output is researched, not an implemented runtime adapter.

Claude’s transcript format is not treated as a single versioned public schema.
The
[source review](../project/research/research-2026-09-14-agent-tool-source-reviews.md)
records evidence from Anthropic’s plugins at `f0dce59`, ccusage at `bd7f89b`, and other
pinned readers. Those readers provide evidence and comparison behavior, not an
authoritative accounting oracle.
Synthetic fixtures establish particular supported shapes; unverified Bedrock and
side-question variants remain explicitly unverified.

## Discovery and Framing

Default project roots include the conventional Claude configuration locations; explicit
roots are selected by the CLI. Main transcripts and nested subagent transcripts are
separate files, not separate billing accounts.
Root locations and their environment-variable precedence are in the
[discovery module](../../crates/urollup-core/src/adapters/discovery.rs); session
selection is in the [selection layer](../../crates/urollup-core/src/selection.rs).

An optional sibling `.meta.json` supplies subagent metadata.
It is not a usage record.
Missing sidecars are allowed; unreadable, malformed or over-limit sidecars fail
explicitly. The current sidecar read ceiling is 1 MiB. Compressed transcript names
resolve to the same sidecar name.
Transcript framing follows the
[shared reader contract](README.md#shared-reader-contract).

## Record Mapping

| Record/field | Interpretation |
| --- | --- |
| `assistant` with unsigned `message.usage.output_tokens` | Usage-bearing response record; zero is a recorded value |
| `progress` with `data.message.message.usage.output_tokens` | Nested assistant copy, not additional usage |
| `sessionId`, `uuid`, `requestId`, `message.id` | Distinct session, record, request and response evidence; do not deduplicate solely by file or timestamp |
| `message.model`, recorded effort | Served-model and effort evidence |
| `apiBlockIndex`, timestamp, evidence position | Revision selection inputs for repeated response blocks |
| `isSidechain`, subagent paths, tool-use IDs and sidecar metadata | Ownership and relationship evidence, not additive usage |
| `message.usage.iterations` entries of type `advisor_message` | Additional model-specific usage components |
| `quotaLimits`, synthetic limit messages | Provider-limit evidence, not token charges |

The typed parser reads all JSON values for validity, discards irrelevant payloads and
keeps only accounting fields.
Repeated keys follow document semantics: the last value wins.
Each request retains `message.usage.speed`, `service_tier` and `inference_geo` as its
recorded pricing context, including values no price table knows.
A missing field in one request does not inherit the previous request’s value.
The agent name alone does not prove provider, billing channel or subscription cost, so
no provider is recorded.

## Accounting Rules

One response can appear in several block records.
Select one complete usage revision: largest output count, then highest block index, then
last in file order, then lowest source ID. Do not sum the blocks or take a field-wise
maximum. Input/cache disagreements produce diagnostics.

Nested progress is always a copy.
Cross-thread `message.id` and `uuid` replays are copies of the established owner’s
request. Main-session records naming another `sessionId` are resumed replays and cannot
take ownership from the original.
Ownership is established in canonical order, not discovery or worker order.
Conflicting models under a reused message ID narrow that identity to the recorded
session and produce an identity diagnostic.
Detailed precedence belongs to
[design §3.4](../urollup-design.md#34-dialect-reconciliation-rules).

| Native usage field | Normalized meaning |
| --- | --- |
| `input_tokens` | Ordinary input, excluding cache reads and writes |
| `cache_read_input_tokens` | Cache-read input, added to inclusive input |
| `cache_creation_input_tokens` | Total cache writes; not an extra addend when the lifetime breakdown is used |
| `cache_creation.ephemeral_5m_input_tokens` | Five-minute cache writes |
| `cache_creation.ephemeral_1h_input_tokens` | One-hour cache writes |
| `output_tokens` | Output, including its reasoning subset |
| `output_tokens_details.thinking_tokens` | Reasoning subset, never added twice |

When the flat cache-write total and breakdown agree, retain the lifetime buckets.
When they disagree, diagnose the mismatch and retain the flat count as unspecified
lifetime instead of summing both representations.
Missing categories remain unknown.

Advisor iterations are separate per-model usage components of the request, each kept
under its own model; the request’s combined total is never counted again beside them.
Pricing each component under its own model is follow-up work (`uro-neii`); no cost path
exists yet.

## Failure and Coverage Behavior

An assistant-shaped line without the required unsigned output field is not currently a
usage-bearing record.
This is a supported-shape boundary, not proof that no upstream usage occurred.
A newly observed native shape needs a documented decision and fixture.
Source-reported costs, API list-price estimates and subscription charges are distinct;
one must not silently substitute for another.

Copies with no reconciled original stay excluded from counted totals with a
`claude-nested-copy-without-original` diagnostic and do not change completeness; whether
they should is the open decision `uro-xpd0`. A transcript that cannot be read completely
makes coverage partial through `source-incomplete`
([shared reader contract](README.md#shared-reader-contract)). Malformed/missing pricing
values still need distinct treatment before final pricing acceptance (`uro-381f`).

## Regression Coverage and Open Work

The [fixture corpus](../../crates/urollup-core/tests/fixtures/README.md) includes
`block-record-selection`, `progress-nested-subagent`, `fork-subagent-uuid-replay`,
`btw-side-question`, `cache-creation-breakdown`, `advisor-iterations` and
`ambiguous-owner`. Their READMEs state assumptions and their expected results state
accounting truth. [Adapter tests](../../crates/urollup-core/tests/adapters.rs),
[worker-count tests](../../crates/urollup-core/tests/worker_count.rs), and the
[CLI golden suite](../../tests/golden/README.md) exercise normalization and
presentation.
[Pricing context tests](../../crates/urollup-core/tests/pricing_context.rs)
cover context retention and metadata that is never inherited from a previous request.

Open format verification includes `uro-01xj` and `uro-89s7`; cache lifetime policy is
tracked in `uro-je0v`. Full private-history acceptance remains `uro-d36a` / `uro-ky6c`.
Do not expand supported shapes or turn unknown values into zeros just to match another
tool’s total.

<!-- This document follows common-doc-guidelines.md.
See github.com/jlevy/practical-prose and review guidelines before editing.
-->
