# Log Volume and Throughput Spike

An exploratory Rust prototype, kept as reference, that measures how much Claude Code and
Codex log data a machine holds and how fast usage can be extracted from it, uncached and
from content-stripped captured records.
It informed the decision to ship urollup’s default-on capture store in Phase 1 and defer
its cache read path to Phase 2. Results and conclusions are in the portable research
brief’s
[Local Log Volume and Throughput](../../docs/project/research/research-2026-09-13-portable-agent-usage.md#local-log-volume-and-throughput)
section. It is not part of the urollup workspace.

The binary opens logs read-only and prints aggregate numbers only.
The manifest and captured records it writes hold local paths and IDs, so keep them in an
owner-only scratch directory and delete them afterwards.

## Commands

```bash
cargo build --release --locked
BIN=target/release/log-throughput-spike
WORK=/path/to/private/scratch   # owner-only, deleted afterwards

# 1. Freeze a snapshot manifest (default roots: ~/.claude/projects,
#    ~/.codex/sessions, ~/.codex/archived_sessions) and print file volume.
$BIN manifest --out $WORK/manifest.tsv --days 30

# 2. Composition survey: classify and strip every record, write one captured
#    zstd JSONL file per source, and print sizes at zstd levels 3 and 19.
$BIN capture --manifest $WORK/manifest.tsv --out $WORK/capture --threads 10 --level19

# 3. Throughput matrix (3 reps x slices x threads x modes), then medians.
./bench.sh $WORK/manifest.tsv $WORK/capture $WORK/runs.jsonl
$BIN summarize --results $WORK/runs.jsonl
```

## Workloads

| Mode | What one run does per source file |
| --- | --- |
| `read` | Read and decompress all bytes, count newlines |
| `value` | Parse every complete line into `serde_json::Value` and extract usage |
| `typed` | Parse every line with borrowed typed structs (`RawValue` for polymorphic fields) |
| `prefilter` | `memmem` for usage markers, typed parse only of matching lines |
| `cache` | Stat the source, typed parse of its captured records |
| `cache-verify` | As `cache`, plus read the source’s first record and final 64 KiB |

Every mode stats each source first.
Parsing modes then merge per-file observations in manifest order, deduplicating Claude
requests by `message.id` plus `requestId` and Codex `token_usage_record` by
`response_id`, walking Codex cumulative `token_count` snapshots per file, and bucketing
by UTC day. `summarize` checks that every parsing mode produced identical totals.

A `--slice window` run reads files modified within the manifest’s `--days`; `all` reads
every file. Legacy pre-JSONL Codex `rollout-*.json` files are counted but not parsed.

## Known Divergences from the urollup Contracts

This spike is a measurement tool, not a reference implementation.
Its rules were chosen to size volume and parse cost, and they differ from the
[design specification](../../docs/urollup-design.md), so do not copy its extraction or
stripping logic:

- **Claude dedupe key:** requests are keyed by `message.id` plus `requestId`, whereas
  the contract’s `req-` key precedence uses the response ID first, so the `dup_*`
  diagnostics do not measure the design’s rule.
  Conflicting block records also keep the larger total usage rather than the largest
  `output_tokens`, then last in file order.
- **Codex counters:** `sum_last` adds every non-identical snapshot’s `last_token_usage`,
  including compaction estimates and context-window-full fills (zero input and output
  with nonzero `total_tokens`) that the contract treats as estimate diagnostics.
- **Stub digest:** `$d` comes from `DefaultHasher::new()`, which has fixed SipHash keys,
  so it is an unkeyed, deterministic stand-in for the contract’s keyed HMAC-SHA-256 and
  is identical across machines.
- **Strip policy:** stripping uses a deny-list of known content keys and keeps strings
  of up to 16 bytes under those keys and up to 256 bytes under any other key, whereas
  the contract stubs known content fields completely in the local capture store (keeping
  unknown keys there) and applies a strict allow-list to every export.

## Tests

```bash
cargo test --release --locked
```

Unit tests use synthetic records only: chunk-boundary line reads, decoder and prefilter
agreement, Codex counter epochs, stripping, and captured records reproducing source
totals.

## Dependencies

Versions are pinned exactly, and every `Cargo.lock` entry was published at least 14 days
before 2026-09-14 (checked against the crates.io API), per the tbd supply-chain
cool-off.
