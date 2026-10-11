//! The admission cost model: what each unit of retained state is charged, from type
//! sizes and one costing rule (scalable-ingestion plan, "Reservation Model").
//!
//! Every function returns an upper bound of the heap bytes urollup requests for its
//! component, under the costing rule:
//!
//! - an allocation of `n` bytes costs `round_up(n, 16) + 16` ([`allocation`]);
//! - a vector that grows by doubling costs 3 × its elements, counting at least the
//!   standard minimum capacity, plus both buffers' allocation costs, because the old and
//!   new buffers are both live while it doubles or shrinks to fit ([`grown_vec`]); a vector
//!   sized once costs its exact allocation ([`sized_vec`]);
//! - a hash map or set costs its hashbrown table plus the half-size table that is live
//!   while it resizes ([`hash_table`]); a push site charges [`hash_entry`] per entry and
//!   [`hash_base`] once per map;
//! - a B-tree costs one node per five entries plus the root, because every non-root node
//!   of the standard B-tree holds at least five ([`btree_map`]); a push site charges
//!   [`btree_entry`] per entry and one node per map;
//! - a stable sort allocates scratch for up to all of its elements ([`sort_scratch`]).
//!
//! Each map and B-tree term states the precondition under which it bounds the standard
//! structure; the model-bound harness checks every primitive against a counting allocator
//! at every count up to a few thousand. Totals are monotone in their counts, so a push site
//! can charge the difference between the total after and before it. Nothing in ingestion
//! charges these yet.

use std::mem::size_of;

use super::Component;
use crate::ledger::diagnostics::Diagnostic;
use crate::ledger::entities::{ProviderLimitObservation, Request, Thread};
use crate::ledger::identity::{AnalyticalId, IdentityKey};
use crate::ledger::names::Name;
use crate::ledger::reconcile::{LineageLink, RequestObservation};
use crate::ledger::scope::{DerivedKey, IdentityBasis};
use crate::selection::Agent;
use crate::sources::evidence::EvidenceRef;
use crate::sources::reader::ReadOptions;

/// The process baseline `F`: 1.25 × the larger of the macOS and the Linux baseline of a
/// release `report`, rounded up to a KiB (7,175 KiB).
///
/// - Linux: the CI `Synthetic scale (ubuntu-24.04)` job of run 38084742044 (merge
///   `26a33e1` of `b610fa4` into `57cdc3d`, Rust 1.98.0, `x86_64-unknown-linux-gnu`) fits
///   the `/usr/bin/time -v` maximum RSS of `scripts/check-scale.py`'s generated corpora as
///   5.60 MiB plus 1,562 B per usage record. That intercept, at most 5.605 MiB, stands in
///   for the smallest fixture's maximum RSS, which was not measured directly.
/// - macOS: `/usr/bin/time -l urollup report --all --no-default-sources --source
///   crates/urollup-core/tests/fixtures/claude-project/missing-request-id --timezone UTC`
///   with a release build of `380233e` (Rust 1.98.0, `aarch64-apple-darwin`) peaked at a
///   1,409,336-byte physical footprint (3,375,104-byte maximum RSS); the CI macOS scale
///   job fits a 1.98 MiB footprint intercept.
///
/// Per-allocator calibration (slice 8) replaces it.
pub const BASELINE_BYTES: u64 = 7_175 << 10;

const _: () = assert!(BASELINE_BYTES * 4 >= 5 * (5_605 << 20) / 1_000);

const ALIGN: u64 = 16;
const OVERHEAD: u64 = 16;
/// The most one allocation adds to its requested bytes: rounding and overhead.
const ALLOCATION_COST: u64 = ALIGN + OVERHEAD;

/// The cost of one allocation of `bytes`: `round_up(bytes, 16) + 16`, or nothing for none.
pub const fn allocation(bytes: u64) -> u64 {
    if bytes == 0 {
        return 0;
    }
    bytes.div_ceil(ALIGN).saturating_mul(ALIGN).saturating_add(OVERHEAD)
}

/// An `Arc<str>` or `Arc<[u8]>` of `len` bytes: one allocation of its strong and weak
/// counts and its data.
pub const fn arc(len: u64) -> u64 {
    allocation(len.saturating_add(2 * size_of::<usize>() as u64))
}

/// The smallest non-zero capacity the standard vector allocates for elements of
/// `element` bytes.
const fn minimum_capacity(element: u64) -> u64 {
    if element == 1 {
        8
    } else if element <= 1024 {
        4
    } else {
        1
    }
}

const fn max(left: u64, right: u64) -> u64 {
    if left > right { left } else { right }
}

/// A vector of `len` elements of `element` bytes that grew by doubling or will shrink to
/// fit: 3 × its elements, at least the minimum capacity, and both buffers' allocation
/// costs.
pub const fn grown_vec(len: u64, element: u64) -> u64 {
    if len == 0 || element == 0 {
        return 0;
    }
    let slots = max(len, minimum_capacity(element));
    slots.saturating_mul(element).saturating_mul(3).saturating_add(2 * ALLOCATION_COST)
}

/// A vector allocated once with exactly `capacity` elements of `element` bytes.
pub const fn sized_vec(capacity: u64, element: u64) -> u64 {
    allocation(capacity.saturating_mul(element))
}

/// The heap a stable sort (`sort`, `sort_by`, `sort_by_key`) of `len` elements of
/// `element` bytes allocates while it runs: the standard driftsort's scratch of at most
/// `max(len, 48)` elements. Scratch of 4 KiB or less stays on the stack, so this is an
/// upper bound. `sort_unstable` allocates nothing.
pub const fn sort_scratch(len: u64, element: u64) -> u64 {
    if len < 2 {
        return 0;
    }
    sized_vec(max(len, 48), element)
}

/// The buckets a hash table holding `entries` of `entry` bytes allocates, as hashbrown
/// 0.17 computes them with 16-byte control groups: tables below 15 entries take 4, 8 or 16
/// buckets, with a larger minimum for entries of 1 to 3 bytes, and larger tables the next
/// power of two at or above 8/7 × the entries.
const fn buckets(entries: u64, entry: u64) -> u64 {
    if entries < 15 {
        let minimum = if entry <= 1 {
            14
        } else if entry <= 3 {
            7
        } else {
            3
        };
        let capacity = max(entries, minimum);
        if capacity < 4 {
            4
        } else if capacity < 8 {
            8
        } else {
            16
        }
    } else {
        (entries.saturating_mul(8) / 7).next_power_of_two()
    }
}

/// One hash table of `buckets` buckets of `entry` bytes: the buckets, padded to the
/// 16-byte control alignment, then one control byte per bucket and a 16-byte group.
const fn table(buckets: u64, entry: u64) -> u64 {
    let data = buckets.saturating_mul(entry).div_ceil(ALIGN).saturating_mul(ALIGN);
    allocation(data.saturating_add(buckets).saturating_add(16))
}

/// A hash map or set holding `entries` of `entry` bytes (key and value together): its
/// table and the half-size table that is live while it resizes.
///
/// Precondition: the map is built by inserts only, and `entries` counts every insert. A
/// map built with `with_capacity` or `reserve` costs [`hash_with_capacity`], and one whose
/// entries are also removed costs [`hash_churned`].
pub const fn hash_table(entries: u64, entry: u64) -> u64 {
    if entries == 0 {
        return 0;
    }
    let buckets = buckets(entries, entry);
    let previous = if buckets > 4 { table(buckets / 2, entry) } else { 0 };
    table(buckets, entry).saturating_add(previous)
}

/// A hash map or set presized with `with_capacity(capacity)` or `reserve`, holding
/// `entries` inserted entries: its table is allocated at the capacity before the first
/// insert.
pub const fn hash_with_capacity(capacity: u64, entries: u64, entry: u64) -> u64 {
    hash_table(max(capacity, entries), entry)
}

/// A hash map or set that holds at most `live` entries at once while entries are also
/// removed. Removal leaves tombstones, and hashbrown then grows rather than rehashing in
/// place once more than half its capacity is used, so the table can reach the size for
/// twice the live entries.
pub const fn hash_churned(live: u64, entry: u64) -> u64 {
    hash_table(live.saturating_mul(2), entry)
}

/// The design's per-entry hash charge for push sites, 3.5 × (entry + 1), rounded up.
/// With [`hash_base`] once per map it bounds [`hash_table`] for every count, under the
/// same insert-only precondition.
pub const fn hash_entry(entry: u64) -> u64 {
    entry.saturating_add(1).saturating_mul(7).div_ceil(2)
}

/// The once-per-map remainder of [`hash_entry`]: the smallest tables are less than 7/8
/// full, and the minimum bucket counts for entries of 1 to 3 bytes.
pub const fn hash_base(entry: u64) -> u64 {
    entry.saturating_add(1).saturating_mul(16).saturating_add(128)
}

/// One standard B-tree node for `key` and `value` sizes, as an internal node: eleven
/// keys and values, twelve child pointers, the parent link and lengths, and padding.
pub const fn btree_node(key: u64, value: u64) -> u64 {
    allocation(key.saturating_add(value).saturating_mul(11).saturating_add(12 + 3 * 8 + 12 * 8))
}

/// A `BTreeMap` (or set, with `value` 0) of `entries`: at most one node per five entries
/// plus the root.
///
/// Precondition: the tree is built by inserts. A tree built by `collect()`, `from` or
/// `extend` from an unsorted iterator costs [`btree_collected`].
pub const fn btree_map(entries: u64, key: u64, value: u64) -> u64 {
    if entries == 0 {
        return 0;
    }
    (entries / 5).saturating_add(1).saturating_mul(btree_node(key, value))
}

/// The per-entry B-tree charge for push sites: the larger of the design's 2.5 × entry and
/// a fifth of a node, which is larger for entries under about 110 bytes. With one node per
/// map it bounds [`btree_map`].
pub const fn btree_entry(key: u64, value: u64) -> u64 {
    let entry = key.saturating_add(value);
    max(entry.saturating_mul(5).div_ceil(2), btree_node(key, value).div_ceil(5))
}

/// A `BTreeMap` or `BTreeSet` of `entries` built by `collect()` from an iterator: the
/// standard library first collects the entries into a vector and stable-sorts it, then
/// builds the tree from the sorted vector, whose nodes the per-entry rule bounds.
pub const fn btree_collected(entries: u64, key: u64, value: u64) -> u64 {
    if entries == 0 {
        return 0;
    }
    let pair = key.saturating_add(value);
    grown_vec(entries, pair)
        .saturating_add(sort_scratch(entries, pair))
        .saturating_add(entries.saturating_mul(btree_entry(key, value)))
        .saturating_add(btree_node(key, value))
}

/// The type sizes the model charges per unit.
pub mod sizes {
    use super::{
        AnalyticalId, DerivedKey, Diagnostic, EvidenceRef, IdentityBasis, IdentityKey, LineageLink,
        Name, ProviderLimitObservation, Request, RequestObservation, Thread, size_of,
    };

    /// A request observation row.
    pub const OBSERVATION: u64 = size_of::<RequestObservation>() as u64;
    /// A logical request row.
    pub const REQUEST: u64 = size_of::<Request>() as u64;
    /// A provider limit observation row.
    pub const LIMIT_ROW: u64 = size_of::<ProviderLimitObservation>() as u64;
    /// A diagnostic row.
    pub const DIAGNOSTIC: u64 = size_of::<Diagnostic>() as u64;
    /// An evidence reference.
    pub const EVIDENCE: u64 = size_of::<EvidenceRef>() as u64;
    /// An analytical ID.
    pub const ID: u64 = size_of::<AnalyticalId>() as u64;
    /// A derived key, as a split part's artifact-local key list holds it.
    pub const DERIVED_KEY: u64 = size_of::<DerivedKey>() as u64;
    /// A thread row.
    pub const THREAD: u64 = size_of::<Thread>() as u64;
    /// An identity key, as the identity registry clones it.
    pub const IDENTITY_KEY: u64 = size_of::<IdentityKey>() as u64;
    /// A lineage link.
    pub const LINK: u64 = size_of::<LineageLink>() as u64;
    /// A pointer, as the per-group member lists hold them.
    pub const POINTER: u64 = size_of::<usize>() as u64;
    /// A key's rank in the set that resolves a linked set's canonical ID.
    pub const KEY_RANK: u64 = size_of::<(u8, IdentityBasis, &AnalyticalId)>() as u64;
    /// One key-graph node across its three vectors: ID, check bits and parent index.
    pub const KEY_NODE: u64 = ID + size_of::<Option<[u8; 8]>>() as u64 + size_of::<u32>() as u64;
    /// One key-graph slot, a node index.
    pub const KEY_SLOT: u64 = size_of::<u32>() as u64;
    /// The key graph's open-addressed slots per node: at most four, since the table is the
    /// next power of two at or above twice the nodes.
    pub const KEY_SLOTS: u64 = 4 * KEY_SLOT;
    /// The first key of each observation, kept while the graph is built.
    pub const FIRST_KEY: u64 = size_of::<u32>() as u64;
    /// The grouping order per observation: its `(root, index)`.
    pub const ORDER: u64 = size_of::<(u32, u32)>() as u64;
    /// The permutation `Requests::from_unsorted` sorts, per request.
    pub const PERMUTATION: u64 = size_of::<usize>() as u64;
    /// An interned name, as rows and per-thread caches hold it.
    pub const NAME: u64 = size_of::<Name>() as u64;
    /// A `BTreeSet` or `Vec` header, as a candidate-set list holds it.
    pub const SET_HEADER: u64 = size_of::<std::collections::BTreeSet<AnalyticalId>>() as u64;
}

/// The most key-graph nodes one observation of `agent` can add, including the
/// artifact-local key of a split part. Codex observations carry at most one key and no
/// revision-invariant field, so they never split; Claude observations carry at most a
/// response and a request key and the model invariant, so a split adds one artifact-local
/// key. Unit tests on each adapter's observation builders assert both.
pub const fn key_bound(agent: Agent) -> u64 {
    match agent {
        Agent::Codex => 1,
        Agent::Claude => 3,
        Agent::Pi => 0,
    }
}

/// A decoded record of `agent`, retained until its source is observed: 3 × its size.
pub const fn decoded_record(agent: Agent) -> u64 {
    let size = match agent {
        Agent::Codex => crate::adapters::codex_rollout::DECODED_RECORD_BYTES,
        Agent::Claude => crate::adapters::claude_project::DECODED_RECORD_BYTES,
        Agent::Pi => 0,
    };
    3 * size
}

/// Codex's construction state for one decoded `turn_context` record, charged with it as a
/// payload: its digest in the root turns `normalize` collects and its entry in its
/// rollout's turn map.
pub const CODEX_TURN_STATE: u64 = crate::adapters::codex_rollout::TURN_STATE_PER_CONTEXT;

/// The forward charge of one decoded request-bearing record of `agent`, before its
/// payloads: the decoded record and κ.
pub const fn decoded_request(agent: Agent) -> u64 {
    decoded_record(agent).saturating_add(kappa(agent))
}

/// A Codex rollout's own observation vector, sized to its observation slots, and the tail
/// a contradicted inference splits off it: built on the decoding worker for a rollout with
/// usage records, or in `normalize` for one without, where it is a high-water term.
pub const fn source_observations(slots: u64) -> u64 {
    2 * sized_vec(slots, sizes::OBSERVATION)
}

/// The query reserve per request in a retained ledger: at most one selected-request item
/// and one request-size value per counted request, which a query may collect (`report`
/// collects both), each 8 B at 3 ×.
pub const QUERY_RESERVE: u64 = 2 * 3 * sizes::POINTER;

/// The per-record structure one request-bearing record of a dialect needs in each phase
/// after decode, excluding owned payloads.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RequestStructure {
    /// Building observations: the observation vector as it is appended (3 ×) and the
    /// dialect's per-record construction state: for Codex, a parent-totals digest or an
    /// own-response entry; for Claude, owner and ambiguity state and the eligibility
    /// vector's stable-sort scratch.
    pub construction: u64,
    /// Grouping: the observation, the key graph and one alias to the dialect's key bound,
    /// the first key and grouping order, the request vector as presized (33/32 of a request
    /// per linked set), and one evidence reference. Requests that conflicting keys split
    /// past the presized capacity are charged where they are pushed ([`split_growth`]).
    pub grouping: u64,
    /// Finalize: the request vector at 3 × (vector, sorted copy and shrink), its
    /// permutation, and the canonical-ID map's entry for each alias.
    pub finalize: u64,
    /// Retained and query: the retained request and its references, plus the query
    /// reserve.
    pub retained: u64,
}

impl RequestStructure {
    /// κ: the largest phase total, which a decode charge covers for every later phase.
    pub const fn kappa(&self) -> u64 {
        max(max(self.construction, self.grouping), max(self.finalize, self.retained))
    }
}

/// One request's references per observation: an evidence reference with half an
/// allocation's cost, and one alias with its share of an allocation per key.
pub const fn request_references(keys: u64) -> u64 {
    sizes::EVIDENCE + ALLOCATION_COST / 2 + keys * (sizes::ID + ALLOCATION_COST)
}

/// The key graph per node: three vectors at 3 × and the slots at 1.5 ×, old and new
/// tables during a rehash.
pub const KEY_GRAPH_PER_NODE: u64 = 3 * sizes::KEY_NODE + sizes::KEY_SLOTS * 3 / 2;

/// The request vector `reconcile` presizes, per linked set: one request and 1/32 more for
/// parts split from conflicting keys.
pub const PRESIZED_REQUEST: u64 = (sizes::REQUEST * 33).div_ceil(32);

/// The canonical-ID map's entry for one request alias, built during finalize.
pub const ALIAS_ENTRY: u64 = btree_entry(sizes::ID, sizes::ID);

/// [`RequestStructure`] for `agent`.
pub const fn request_structure(agent: Agent) -> RequestStructure {
    let keys = key_bound(agent);
    let construction_state = match agent {
        Agent::Codex => crate::adapters::codex_rollout::CONSTRUCTION_STATE_PER_RECORD,
        Agent::Claude => {
            crate::adapters::claude_project::OWNER_STATE_PER_RECORD
                + crate::adapters::claude_project::ELIGIBLE_ENTRY_BYTES
        }
        Agent::Pi => 0,
    };
    RequestStructure {
        construction: 3 * sizes::OBSERVATION + construction_state,
        grouping: sizes::OBSERVATION
            + keys * KEY_GRAPH_PER_NODE
            + sizes::FIRST_KEY
            + sizes::ORDER
            + PRESIZED_REQUEST
            + request_references(keys),
        finalize: FINALIZE_PER_REQUEST + keys.saturating_sub(1) * ALIAS_ENTRY,
        retained: sizes::REQUEST + request_references(keys) + QUERY_RESERVE,
    }
}

/// The request vector once split parts push it past its presized capacity: all of it at
/// the doubling rate. Charged where the first such part is pushed.
pub const fn split_growth(requests: u64) -> u64 {
    grown_vec(requests, sizes::REQUEST)
}

/// The structures a key whose observations conflict leaves beside its `parts` split
/// requests, charged where the split is made: the candidate graph's entry for each part,
/// the candidate set the ledger retains, and its components map while it is resolved.
pub const fn conflicting_key(parts: u64) -> u64 {
    parts
        .saturating_mul(btree_entry(sizes::ID, sizes::ID))
        .saturating_add(2 * btree_map(parts, sizes::ID, 0))
        .saturating_add(btree_entry(sizes::ID, sizes::SET_HEADER))
        .saturating_add(3 * sizes::SET_HEADER)
}

/// The candidate threads an ambiguous request keeps in its ownership, charged where its
/// `ConflictingOwners` diagnostic is pushed.
pub const fn ambiguous_owners(candidates: u64) -> u64 {
    btree_map(candidates, sizes::ID, 0)
}

/// Finalize per request: the request vector, which may have grown to twice the requests,
/// its shrunk copy, and the sort permutation.
pub const FINALIZE_PER_REQUEST: u64 = 3 * sizes::REQUEST + sizes::PERMUTATION;

/// κ for `agent`.
pub const fn kappa(agent: Agent) -> u64 {
    request_structure(agent).kappa()
}

/// One provider limit observation: 3 × its row as built, or during limit reconciliation
/// the input row, its sort-cache entry and the output row, whichever is larger.
pub const fn limit_row() -> u64 {
    max(
        3 * sizes::LIMIT_ROW,
        2 * sizes::LIMIT_ROW + crate::ledger::reconcile::LIMIT_SORT_CACHE_BYTES,
    )
}

/// One diagnostic whose detail text has `detail_capacity` bytes of capacity (a `format!`
/// detail's capacity, not its length) and which cites `evidence` references: its row at
/// 5 × (compaction iterates the input vector, of up to twice the rows, while it pushes
/// into a doubling vector, and its sorts fit within that), its text and its evidence
/// vector.
pub const fn diagnostic(detail_capacity: u64, evidence: u64) -> u64 {
    5 * sizes::DIAGNOSTIC + allocation(detail_capacity) + grown_vec(evidence, sizes::EVIDENCE)
}

/// A new process-interned [`Name`] of `text` bytes: the text, its leaked reference and
/// its entry in the name table.
pub const fn name_intern(text: u64) -> u64 {
    allocation(text)
        + allocation(size_of::<&str>() as u64)
        + btree_entry(size_of::<&str>() as u64, sizes::NAME)
}

/// The name table's one-time base, charged with the first name: its first B-tree node.
pub const NAME_TABLE_BASE: u64 = btree_node(size_of::<&str>() as u64, sizes::NAME);

/// A new interned overflow [`Measures`](crate::ledger::tokens::Measures) pattern: its row
/// in a doubling vector and its hash-map entry.
pub const fn overflow_intern() -> u64 {
    3 * crate::ledger::tokens::OVERFLOW_ROW_BYTES
        + hash_entry(crate::ledger::tokens::OVERFLOW_ENTRY_BYTES)
}

/// The overflow tables' one-time base, charged with the first pattern: the row vector's
/// minimum capacity and allocation costs beyond the per-row charge, and the hash map's
/// base.
pub const OVERFLOW_TABLE_BASE: u64 = grown_vec(1, crate::ledger::tokens::OVERFLOW_ROW_BYTES)
    - 3 * crate::ledger::tokens::OVERFLOW_ROW_BYTES
    + hash_base(crate::ledger::tokens::OVERFLOW_ENTRY_BYTES);

/// A thread's cache of recently interned names (`Name::new`'s `RECENT`), up to eight
/// names in a doubling vector, and the registration of its destructor, which std keeps in
/// a per-thread vector where the platform does not run thread-local destructors itself.
pub const NAME_CACHE: u64 = grown_vec(crate::ledger::names::RECENT_CAPACITY as u64, sizes::NAME)
    + grown_vec(1, 2 * sizes::POINTER);

/// Line-buffer capacity up to this decodes inside a worker slot; a larger line buffer
/// needs the large-record permit.
pub const SLOT_LINE_CAPACITY: u64 = 4 << 20;

/// The need of a line buffer of capacity `capacity`, `4 × C′`: growing to it holds the old
/// buffer and the new one (`C + C′`, at most `1.5 × C′`), and parsing holds the buffer
/// plus `serde_json`'s scratch for escaped strings, within `4 × C′`.
pub const fn line_need(capacity: u64) -> u64 {
    capacity.saturating_mul(4)
}

/// The capacity a line buffer of capacity `capacity` doubles to, `C′ = min(2 × C,
/// max_record_bytes)`. One doubling always suffices: a read appends at most one 128 KiB
/// read-buffer chunk, and capacity starts at the 256 KiB retained capacity.
pub const fn next_line_capacity(capacity: u64, max_record_bytes: u64) -> u64 {
    let doubled = capacity.saturating_mul(2);
    if doubled < max_record_bytes { doubled } else { max_record_bytes }
}

/// The heap a line buffer holds while it grows from `capacity`: the old and new buffers,
/// `C + C′`, which [`line_need`] of the new capacity covers.
pub const fn line_growth(capacity: u64, max_record_bytes: u64) -> u64 {
    capacity.saturating_add(next_line_capacity(capacity, max_record_bytes))
}

/// zstd windows up to this size decode inside a worker slot.
pub const SLOT_ZSTD_WINDOW: u64 = 8 << 20;

/// zstd's largest block, `ZSTD_BLOCKSIZE_MAX`.
const ZSTD_BLOCK: u64 = 128 << 10;

/// The input buffer of the reader's zstd frame loop, `ZSTD_DStreamInSize()`: one block and
/// its 3-byte header.
pub const ZSTD_INPUT_BUFFER: u64 = ZSTD_BLOCK + 3;

/// `sizeof(ZSTD_DCtx)` rounded up to a KiB: `ZSTD_sizeof_DCtx` reports 95,968 bytes before
/// any frame on aarch64 macOS and Windows and 95,976 on Linux (zstd 1.5.7). The model
/// harness re-measures it on every platform.
pub const ZSTD_CONTEXT: u64 = 96 << 10;

/// zstd's streaming decoder for frames of `window` bytes, by zstd 1.5.7's
/// `ZSTD_estimateDStreamSize` formula, computed in Rust because zstd-sys exposes that
/// function only behind its `experimental` feature: the decoder context, an input buffer
/// of one block, and an output buffer of the window plus two blocks and twice the 32-byte
/// wildcopy slack. A block is the window or 128 KiB, whichever is smaller.
pub const fn zstd_stream(window: u64) -> u64 {
    let block = if window < ZSTD_BLOCK { window } else { ZSTD_BLOCK };
    ZSTD_CONTEXT + block + window + 2 * block + 2 * 32
}

/// The slot's zstd decoder: the stream for an 8 MiB window and the frame loop's input
/// buffer, about 8.6 MiB.
pub const ZSTD_DECODER: u64 = zstd_stream(SLOT_ZSTD_WINDOW) + allocation(ZSTD_INPUT_BUFFER);

/// flate2's longest gzip header field, `MAX_HEADER_BUF`.
const GZIP_HEADER_FIELD: u64 = 65_535;

/// The gzip decoder's state and buffers: 80 KiB for flate2's 32 KiB input buffer and
/// `miniz_oxide`'s `InflateState`, of which the model harness measures 76,368 bytes with an
/// encoder's default header, plus the header fields flate2 keeps per member: `extra`,
/// sized exactly, and `filename` and `comment`, pushed byte by byte, each up to 65,535
/// bytes.
pub const GZIP_DECODER: u64 =
    (80 << 10) + allocation(GZIP_HEADER_FIELD) + 2 * grown_vec(GZIP_HEADER_FIELD, 1);

/// The read buffer `sources::reader::decode` wraps each source in.
pub const READ_BUFFER: u64 = 128 << 10;

/// The boxed reader `sources::reader::decode` returns and the decoder struct it wraps.
pub const READER_STATE: u64 = 512;

/// The default thread stack of a decoding worker.
pub const WORKER_STACK: u64 = 2 << 20;

/// `quotaLimits` documents built from up to this much text decode inside a worker slot.
pub const QUOTA_DOCUMENT_TEXT: u64 = 4 << 10;

/// The slot's allowance for one `quotaLimits` document.
pub const QUOTA_DOCUMENT_ALLOWANCE: u64 = 640 << 10;

/// The most a `quotaLimits` document costs per byte of its text: the allowance over the
/// text it holds, 160, above the densest document, one-entry objects nested in each other,
/// which cost one B-tree node per 5 bytes of text.
pub const QUOTA_COST_PER_TEXT_BYTE: u64 = QUOTA_DOCUMENT_ALLOWANCE / QUOTA_DOCUMENT_TEXT;

const _: () = assert!(
    QUOTA_COST_PER_TEXT_BYTE * 5
        >= btree_node(size_of::<String>() as u64, size_of::<serde_json::Value>() as u64)
);

/// The running cost of a `quotaLimits` document built from `text` bytes.
pub const fn quota_document(text: u64) -> u64 {
    text.saturating_mul(QUOTA_COST_PER_TEXT_BYTE)
}

/// The largest `(index, result)` pair of the two adapters' parallel joins.
pub const JOIN_PAIR: u64 = max(
    crate::adapters::codex_rollout::JOIN_PAIR_BYTES,
    crate::adapters::claude_project::JOIN_PAIR_BYTES,
);

/// A worker's `done` vector in the parallel join, at its minimum capacity: 12 × the pair
/// plus two allocations. A source's share of the join's later copies is charged with its
/// result ([`join_copies`]).
pub const JOIN_DONE: u64 = grown_vec(1, JOIN_PAIR);

/// The copies the join makes of one source's result pair: the worker's doubling `done`
/// vector, then `results` beside `values`, then `values` beside the unzipped vectors.
pub const fn join_copies() -> u64 {
    3 * JOIN_PAIR
}

/// One worker slot `b`: the read buffer and boxed reader, the retained line, the line need
/// at the slot's 4 MiB capacity (16 MiB, which also covers a Claude subagent's sidecar,
/// read up to 1 MiB and parsed after the scan drops its line buffer), the `quotaLimits`
/// allowance, the thread stack, the join's `done` vector, the thread's name cache, and the
/// zstd or gzip decoder when discovery found such a source.
pub const fn worker_slot(zstd: bool, gzip: bool) -> u64 {
    let mut slot = allocation(READ_BUFFER)
        + allocation(READER_STATE)
        + allocation(ReadOptions::RETAINED_LINE_CAPACITY as u64)
        + line_need(SLOT_LINE_CAPACITY)
        + QUOTA_DOCUMENT_ALLOWANCE
        + WORKER_STACK
        + JOIN_DONE
        + NAME_CACHE;
    if zstd {
        slot += ZSTD_DECODER;
    }
    if gzip {
        slot += GZIP_DECODER;
    }
    slot
}

/// The large-record permit's need for what its holder holds at once: a line buffer of
/// capacity `line_capacity` past the slot's 4 MiB (`4 × C′`), the decoder for a zstd
/// window past the slot's 8 MiB, and a `quotaLimits` document built from `quota_text`
/// bytes past the slot's allowance.
pub const fn large_record(line_capacity: u64, zstd_window: u64, quota_text: u64) -> u64 {
    let line = if line_capacity > SLOT_LINE_CAPACITY { line_need(line_capacity) } else { 0 };
    let window = if zstd_window > SLOT_ZSTD_WINDOW { zstd_stream(zstd_window) } else { 0 };
    let document = quota_document(quota_text);
    let document = if document > QUOTA_DOCUMENT_ALLOWANCE { document } else { 0 };
    line.saturating_add(window).saturating_add(document)
}

/// zstd's default window limit, `ZSTD_WINDOWLOG_LIMIT_DEFAULT` (2^27).
pub const ZSTD_DEFAULT_WINDOW_LIMIT: u64 = 1 << 27;

/// The largest line and window need at the default limits: a 64 MiB line buffer (256 MiB)
/// and the decoder for zstd's default 128 MiB window limit (about 128.5 MiB). A
/// `quotaLimits` document adds its own running cost.
pub const LARGEST_LARGE_RECORD: u64 =
    large_record(ReadOptions::DEFAULT_MAX_RECORD_BYTES as u64, ZSTD_DEFAULT_WINDOW_LIMIT, 0);

/// Query: one session, day or group row while it is accumulated and collected. A *guess*:
/// the model harness's sessions document of 201 rows holds about 650 bytes per row beyond
/// its per-request lists, its fixed part included; slice 7 calibrates it with the query
/// checkpoint.
pub const QUERY_PER_ROW: u64 = 1024;

/// Query: the document's fixed part, its metadata, summaries and maps. A *guess*, like
/// [`QUERY_PER_ROW`]; the model harness's empty report document holds under 1 KiB.
pub const QUERY_FIXED: u64 = 64 << 10;

/// A phase's modeled heap, broken down by the ledger's charge components, as slice 7's
/// statistics line prints it. [`PhaseEstimate::total`] is what a checkpoint compares.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PhaseEstimate {
    /// Decoded records.
    pub records: u64,
    /// Request structure: observations, the key graph, requests and their working sets.
    pub kappa: u64,
    /// Record and observation payloads and diagnostics.
    pub payloads: u64,
    /// Provider limit rows.
    pub limits: u64,
    /// Process interns.
    pub interns: u64,
    /// Source and thread rows and their reconciliation.
    pub sources: u64,
    /// The query and session-index reserves, and the query's working set.
    pub reserves: u64,
    /// Worker slots and the large-record permit.
    pub slots: u64,
}

impl PhaseEstimate {
    /// The sum of every component.
    pub const fn total(&self) -> u64 {
        self.records
            .saturating_add(self.kappa)
            .saturating_add(self.payloads)
            .saturating_add(self.limits)
            .saturating_add(self.interns)
            .saturating_add(self.sources)
            .saturating_add(self.reserves)
            .saturating_add(self.slots)
    }

    /// One component.
    pub const fn get(&self, component: Component) -> u64 {
        match component {
            Component::Records => self.records,
            Component::Kappa => self.kappa,
            Component::Payloads => self.payloads,
            Component::Limits => self.limits,
            Component::Interns => self.interns,
            Component::Sources => self.sources,
            Component::Reserves => self.reserves,
            Component::Slots => self.slots,
        }
    }
}

/// A query document over `requests` counted requests with `rows` rows: the per-request
/// lists, the rows and the fixed part.
pub const fn query_document(requests: u64, rows: u64) -> u64 {
    requests
        .saturating_mul(QUERY_RESERVE)
        .saturating_add(rows.saturating_mul(QUERY_PER_ROW))
        .saturating_add(QUERY_FIXED)
}

/// Rendering `rendered` bytes: the output buffer at 3 ×, as it grows by doubling.
pub const fn render(rendered: u64) -> u64 {
    rendered.saturating_mul(3)
}

/// The query's working set over `requests` counted requests that yields `rows` rows and
/// renders `rendered` bytes: the document and its rendering.
pub const fn query(requests: u64, rows: u64, rendered: u64) -> PhaseEstimate {
    PhaseEstimate {
        reserves: query_document(requests, rows).saturating_add(render(rendered)),
        ..PhaseEstimate::ZERO
    }
}

impl PhaseEstimate {
    /// An estimate of nothing.
    pub const ZERO: Self = Self {
        records: 0,
        kappa: 0,
        payloads: 0,
        limits: 0,
        interns: 0,
        sources: 0,
        reserves: 0,
        slots: 0,
    };
}

/// Exact counts at a construction checkpoint, before observations are built.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ConstructionCounts {
    /// Decoded records still held.
    pub decoded_records: u64,
    /// Request-bearing records, each of which becomes at most one observation.
    pub request_records: u64,
    /// Codex `turn_context` records, whose turn state construction builds.
    pub turn_contexts: u64,
    /// The observation slots of the largest Codex rollout observed during construction.
    pub largest_source_slots: u64,
    /// Payloads the decoded records and their sources own, by the costing rule.
    pub decoded_payload: u64,
    /// Observations already built by decoding workers, with their vectors and payloads.
    pub built_observations: u64,
    /// Provider limit observations.
    pub limit_rows: u64,
    /// The deep size of threads, relationships, diagnostics, gaps and the source table.
    pub metadata: u64,
}

/// The modeled heap while `agent`'s observations are built: the decoded records and
/// their payloads, which each observation may copy once, the construction share of κ per
/// request-bearing record, the eligibility sort's minimum scratch and the maps' bases,
/// Codex turn state and its largest rollout's own observation vectors, worker-built
/// observations, limit rows and metadata. The model harness cannot reach this phase from
/// outside the adapters, so this estimate is checked only once slices 5 and 6 charge
/// construction.
pub const fn construction_estimate(agent: Agent, counts: &ConstructionCounts) -> PhaseEstimate {
    let structure = request_structure(agent);
    let mut kappa = counts
        .request_records
        .saturating_mul(structure.construction)
        .saturating_add(counts.built_observations)
        // The base cost of the owner, uuid, model and ambiguity maps and the thread map,
        // the vectors' allocation costs, and the eligibility sort's 48-element minimum.
        .saturating_add(5 * hash_base(32))
        .saturating_add(4 * ALLOCATION_COST)
        .saturating_add(sort_scratch(48, 2 * sizes::POINTER));
    let mut payloads = counts.decoded_payload.saturating_mul(2);
    if let Agent::Codex = agent {
        kappa = kappa.saturating_add(source_observations(counts.largest_source_slots));
        payloads = payloads.saturating_add(counts.turn_contexts.saturating_mul(CODEX_TURN_STATE));
    }
    PhaseEstimate {
        records: counts.decoded_records.saturating_mul(decoded_record(agent)),
        kappa,
        payloads,
        limits: counts.limit_rows.saturating_mul(limit_row()),
        sources: counts.metadata.saturating_mul(3),
        ..PhaseEstimate::ZERO
    }
}

/// Exact counts at a reconciliation checkpoint, before grouping.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ReconcileCounts {
    /// Request observations.
    pub observations: u64,
    /// The capacity of the observation vector.
    pub observation_capacity: u64,
    /// The key-graph nodes the observations can add: each observation's keys, or its
    /// artifact-local key when it has none, plus one for each observation with a
    /// revision-invariant field, whose split part registers its artifact-local key, plus
    /// two per lineage link.
    pub key_nodes: u64,
    /// The observations' owned payloads by the costing rule: spilled keys and
    /// invariants and per-model usage.
    pub observation_payload: u64,
    /// The observations in the largest linked set, which bounds one group's scratch; at a
    /// checkpoint, where linked sets are not yet known, the observations.
    pub largest_group: u64,
    /// Provider limit observations.
    pub limit_rows: u64,
    /// Thread observations, which reconciliation sorts, registers, groups and merges.
    pub threads: u64,
    /// The heap the thread observations own, by the costing rule.
    pub thread_heap: u64,
    /// Lineage links, which reconciliation sorts.
    pub links: u64,
    /// The deep size of everything else the input holds: relationships, tool actions,
    /// links, gaps, diagnostics, the source table and the thread vector itself.
    pub metadata: u64,
}

/// The scratch one group of `members` observations allocates while its request is
/// built: member, original and revision lists, a split's parts, keys and evidence, and
/// the sets that rank its keys, check its fields and collect its owners and models.
pub const fn group_scratch(agent: Agent, members: u64) -> u64 {
    if members == 0 {
        return 0;
    }
    let pointer = sizes::POINTER;
    let keys = members.saturating_mul(key_bound(agent));
    sized_vec(members, pointer)
        .saturating_add(2 * grown_vec(members, pointer))
        .saturating_add(sized_vec(members, 3 * pointer))
        .saturating_add(members.saturating_mul(sized_vec(1, pointer)))
        .saturating_add(sized_vec(members, sizes::DERIVED_KEY))
        .saturating_add(sized_vec(members, sizes::EVIDENCE))
        .saturating_add(grown_vec(members, sizes::ID))
        .saturating_add(btree_collected(keys, sizes::KEY_RANK, 0))
        .saturating_add(grown_vec(keys, sizes::ID))
        .saturating_add(3 * btree_collected(members, 2 * pointer, pointer))
        .saturating_add(KEY_DERIVATION)
}

/// The structures thread reconciliation builds for `threads` thread observations that own
/// `thread_heap` bytes: the stable sort's scratch, the identity registry with a clone of
/// each key, the link graph, the groups and their vectors, the merged map with its
/// clones, and the canonical-ID map.
pub const fn thread_reconcile(threads: u64, thread_heap: u64) -> u64 {
    if threads == 0 {
        return 0;
    }
    sort_scratch(threads, sizes::THREAD)
        .saturating_add(btree_map(threads, sizes::ID, sizes::IDENTITY_KEY))
        .saturating_add(btree_map(threads, sizes::ID, sizes::ID))
        .saturating_add(btree_map(threads, sizes::ID, 3 * sizes::POINTER))
        .saturating_add(threads.saturating_mul(grown_vec(1, sizes::THREAD)))
        .saturating_add(btree_map(threads, sizes::ID, sizes::THREAD))
        .saturating_add(btree_map(threads, sizes::ID, sizes::ID))
        .saturating_add(thread_heap.saturating_mul(2))
}

/// The transient scratch of deriving one key's ID: the key's kind and components, the
/// canonical JSON array they become, and its text. A 4 KiB allowance, which the model
/// harness checks against an artifact-local key's derivation.
pub const KEY_DERIVATION: u64 = 4 << 10;

/// The key graph for `nodes` nodes over `observations` observations: its three vectors at
/// 3 × and its slots at 1.5 × with the eight-slot minimum before and after the first
/// rehash, and the first-key vector.
pub const fn key_graph(nodes: u64, observations: u64) -> u64 {
    nodes
        .saturating_mul(KEY_GRAPH_PER_NODE)
        .saturating_add(KEY_DERIVATION)
        .saturating_add(3 * 2 * ALLOCATION_COST)
        .saturating_add(2 * sized_vec(8, sizes::KEY_SLOT))
        .saturating_add(sized_vec(observations, sizes::FIRST_KEY))
}

/// What `Requests::from_unsorted` allocates for `requests` requests: the permutation it
/// sorts and, when the vector has more than 1/16 spare capacity, its shrunk copy. The
/// request rows themselves are already held, within [`FINALIZE_PER_REQUEST`].
pub const fn request_sort(requests: u64) -> u64 {
    sized_vec(requests, sizes::PERMUTATION).saturating_add(sized_vec(requests, sizes::REQUEST))
}

/// The canonical-ID map finalize builds from `aliases` request aliases.
pub const fn alias_map(aliases: u64) -> u64 {
    if aliases == 0 {
        return 0;
    }
    aliases.saturating_mul(ALIAS_ENTRY).saturating_add(btree_node(sizes::ID, sizes::ID))
}

/// The grouping order: one `(root, index)` per observation, sized once.
pub const fn grouping_order(observations: u64) -> u64 {
    sized_vec(observations, sizes::ORDER)
}

/// The request vector presized for `sets` linked sets, and each observation's evidence
/// reference and up to `keys` aliases.
pub const fn request_vector(sets: u64, observations: u64, keys: u64) -> u64 {
    sets.saturating_mul(PRESIZED_REQUEST)
        .saturating_add(ALLOCATION_COST)
        .saturating_add(observations.saturating_mul(request_references(keys)))
}

/// The modeled heap while `agent`'s observations group into requests, from the counts at
/// the grouping checkpoint. Linked sets are bounded by the observations, and the largest
/// group by `counts.largest_group`. Split growth, the structures conflicting keys leave and
/// the diagnostics grouping pushes are charged where they are pushed.
pub const fn grouping_estimate(agent: Agent, counts: &ReconcileCounts) -> PhaseEstimate {
    let observations = counts.observations;
    let sorts =
        max(sort_scratch(counts.threads, sizes::THREAD), sort_scratch(counts.links, sizes::LINK));
    PhaseEstimate {
        kappa: sized_vec(counts.observation_capacity, sizes::OBSERVATION)
            .saturating_add(key_graph(counts.key_nodes, observations))
            .saturating_add(grouping_order(observations))
            .saturating_add(request_vector(observations, observations, key_bound(agent)))
            .saturating_add(group_scratch(agent, counts.largest_group)),
        payloads: counts.observation_payload.saturating_mul(2),
        limits: counts.limit_rows.saturating_mul(limit_row()),
        sources: counts
            .metadata
            .saturating_mul(3)
            .saturating_add(thread_reconcile(counts.threads, counts.thread_heap))
            .saturating_add(sorts),
        ..PhaseEstimate::ZERO
    }
}

/// The modeled heap while `agent`'s requests are sorted and limits and diagnostics are
/// finalized; observations and the key graph are freed by then.
pub const fn finalize_estimate(agent: Agent, counts: &ReconcileCounts) -> PhaseEstimate {
    let aliases = counts.observations.saturating_mul(key_bound(agent).saturating_sub(1));
    PhaseEstimate {
        kappa: counts
            .observations
            .saturating_mul(FINALIZE_PER_REQUEST)
            .saturating_add(aliases.saturating_mul(ALIAS_ENTRY))
            .saturating_add(btree_node(sizes::ID, sizes::ID))
            .saturating_add(4 * ALLOCATION_COST),
        payloads: counts.observation_payload.saturating_mul(2),
        limits: counts.limit_rows.saturating_mul(limit_row()),
        sources: counts
            .metadata
            .saturating_mul(3)
            .saturating_add(thread_reconcile(counts.threads, counts.thread_heap)),
        ..PhaseEstimate::ZERO
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allocations_round_to_sixteen_and_add_sixteen() {
        assert_eq!(allocation(0), 0);
        assert_eq!(allocation(1), 32);
        assert_eq!(allocation(16), 32);
        assert_eq!(allocation(17), 48);
        assert_eq!(arc(5), 48);
    }

    #[test]
    fn vectors_cost_three_times_their_elements_and_at_least_the_minimum_capacity() {
        assert_eq!(grown_vec(0, 8), 0);
        assert_eq!(grown_vec(1, 8), 3 * 4 * 8 + 64);
        assert_eq!(grown_vec(100, 8), 3 * 800 + 64);
        assert_eq!(grown_vec(1, 1), 3 * 8 + 64);
        assert_eq!(sized_vec(10, 16), 176);
        assert_eq!(sort_scratch(1, 16), 0);
        assert_eq!(sort_scratch(10, 16), sized_vec(48, 16));
        assert_eq!(sort_scratch(1_000, 16), sized_vec(1_000, 16));
    }

    #[test]
    fn hash_tables_follow_hashbrown_and_the_per_entry_rule_bounds_them() {
        assert_eq!(buckets(1, 16), 4);
        assert_eq!(buckets(1, 1), 16);
        assert_eq!(buckets(1, 2), 8);
        assert_eq!(buckets(7, 16), 8);
        assert_eq!(buckets(8, 16), 16);
        assert_eq!(buckets(15, 16), 32);
        for entry in [1, 2, 4, 16, 28, 76, 80] {
            let mut previous = 0;
            for entries in 1..5000 {
                let total = hash_table(entries, entry);
                assert!(total >= previous, "monotone at {entries} × {entry}");
                assert!(
                    total <= entries * hash_entry(entry) + hash_base(entry),
                    "{entries} × {entry}: {total}"
                );
                previous = total;
            }
        }
        assert_eq!(hash_with_capacity(1_000, 3, 16), hash_table(1_000, 16));
        assert_eq!(hash_churned(1_000, 16), hash_table(2_000, 16));
    }

    #[test]
    fn btree_entries_bound_the_tree_for_every_size() {
        for (key, value) in [(16, 8), (17, 17), (24, 400), (16, 0)] {
            for entries in 1..2000 {
                assert!(
                    btree_map(entries, key, value)
                        <= entries * btree_entry(key, value) + btree_node(key, value),
                    "{entries} × ({key} + {value})"
                );
                assert!(btree_collected(entries, key, value) >= btree_map(entries, key, value));
            }
        }
    }

    /// κ is pinned to the current row sizes, so a change to any of these types fails here
    /// and has to update the plan's κ table with it.
    #[test]
    fn kappa_is_pinned_to_the_row_sizes() {
        assert_eq!(
            (sizes::OBSERVATION, sizes::REQUEST, sizes::ID, sizes::KEY_NODE, sizes::EVIDENCE),
            (224, 216, 17, 30, 16)
        );
        assert_eq!((decoded_record(Agent::Codex), decoded_record(Agent::Claude)), (240, 528));
        let codex = request_structure(Agent::Codex);
        let claude = request_structure(Agent::Claude);
        assert_eq!(
            (codex.construction, codex.grouping, codex.finalize, codex.retained),
            (792, 654, 656, 345)
        );
        assert_eq!(
            (claude.construction, claude.grouping, claude.finalize, claude.retained),
            (1074, 980, 868, 443)
        );
        assert_eq!((kappa(Agent::Codex), kappa(Agent::Claude)), (792, 1074));
        for structure in [codex, claude] {
            assert_eq!(
                structure.kappa(),
                structure
                    .construction
                    .max(structure.grouping)
                    .max(structure.finalize)
                    .max(structure.retained)
            );
        }
    }

    #[test]
    fn worker_slots_and_the_permit_follow_the_design_terms() {
        assert_eq!(line_need(SLOT_LINE_CAPACITY), 16 << 20);
        // Growth holds the old and new buffers, within the new capacity's need, which
        // also covers the earlier `C + 2 × C′` rule.
        let max_record = ReadOptions::DEFAULT_MAX_RECORD_BYTES as u64;
        let mut capacity = ReadOptions::RETAINED_LINE_CAPACITY as u64;
        while capacity < max_record {
            let next = next_line_capacity(capacity, max_record);
            assert!(line_growth(capacity, max_record) <= line_need(next));
            assert!(capacity + 2 * next <= line_need(next));
            capacity = next;
        }
        assert_eq!(capacity, max_record);
        assert_eq!(QUOTA_COST_PER_TEXT_BYTE, 160);
        assert_eq!(quota_document(QUOTA_DOCUMENT_TEXT), QUOTA_DOCUMENT_ALLOWANCE);
        let plain = worker_slot(false, false);
        let expected_plain = allocation(READ_BUFFER)
            + allocation(READER_STATE)
            + allocation(256 << 10)
            + (16 << 20)
            + (640 << 10)
            + (2 << 20)
            + JOIN_DONE
            + NAME_CACHE;
        assert_eq!(plain, expected_plain);
        assert!((19 << 20..20 << 20).contains(&plain), "{plain}");
        let zstd = worker_slot(true, false) - plain;
        assert_eq!(zstd, ZSTD_DECODER);
        assert!((8 << 20..9 << 20).contains(&zstd), "{zstd}");
        assert_eq!(large_record(4 << 20, 8 << 20, 4 << 10), 0);
        assert_eq!(large_record(8 << 20, 0, 0), 32 << 20);
        assert_eq!(large_record(0, 16 << 20, 0), zstd_stream(16 << 20));
        assert_eq!(large_record(0, 0, 5 << 10), 800 << 10);
        assert_eq!(zstd_stream(1 << 27), (1 << 27) + 3 * (128 << 10) + 64 + ZSTD_CONTEXT);
        assert_eq!(LARGEST_LARGE_RECORD, (256 << 20) + zstd_stream(1 << 27));
    }

    #[test]
    fn phase_estimates_total_their_components() {
        let counts = ReconcileCounts {
            observations: 1_000,
            observation_capacity: 1_000,
            key_nodes: 2_000,
            observation_payload: 4_000,
            largest_group: 3,
            limit_rows: 10,
            threads: 4,
            thread_heap: 400,
            links: 0,
            metadata: 2_000,
        };
        for agent in [Agent::Codex, Agent::Claude] {
            for estimate in [grouping_estimate(agent, &counts), finalize_estimate(agent, &counts)] {
                let sum: u64 =
                    Component::ALL.iter().map(|component| estimate.get(*component)).sum();
                assert_eq!(estimate.total(), sum);
            }
        }
        let saturated = ReconcileCounts { observations: u64::MAX, ..counts };
        assert_eq!(grouping_estimate(Agent::Claude, &saturated).total(), u64::MAX);
        assert_eq!(query(10, 2, 100).total(), 10 * 48 + 2 * 1024 + 300 + (64 << 10));
    }
}
