//! The admission cost model: what each unit of retained state is charged, from type
//! sizes and one costing rule (scalable-ingestion plan, "Reservation Model").
//!
//! Every function returns an upper bound of the heap bytes urollup requests for its
//! component, under the costing rule:
//!
//! - an allocation of `n` bytes costs `round_up(n, 16) + 16` ([`allocation`]);
//! - an element of a vector that grows by doubling costs 3 × its size, because the old and
//!   new buffers are both live while it doubles or shrinks to fit ([`grown_vec`]); a vector
//!   sized once costs its exact allocation ([`sized_vec`]);
//! - a hash table entry costs about 3.5 × (entry + one control byte), the table at 7/8
//!   load plus the previous table while it resizes ([`hash_table`]);
//! - a B-tree costs one node per five entries plus the root, because every non-root node
//!   of the standard B-tree holds at least five ([`btree_map`]).
//!
//! Totals are monotone in their counts, so a push site can charge the difference between
//! the total after and before it. Nothing in ingestion charges these yet.

use std::mem::size_of;

use crate::ledger::diagnostics::Diagnostic;
use crate::ledger::entities::{ProviderLimitObservation, Request};
use crate::ledger::identity::AnalyticalId;
use crate::ledger::reconcile::RequestObservation;
use crate::selection::Agent;
use crate::sources::evidence::EvidenceRef;
use crate::sources::reader::ReadOptions;

/// The process baseline `F`: 1.25 × the largest peak of a release `report` over the
/// smallest fixture, rounded up to a MiB. On the reference macOS laptop the physical
/// footprint peaked at 1.41 MB and the maximum RSS at 3.38 MB; Linux maximum RSS is not
/// measured yet, so the macOS RSS stands in for it until calibration (slice 8).
pub const BASELINE_BYTES: u64 = 5 << 20;

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
/// fit: 3 × each element, at least the minimum capacity, and both buffers' allocation
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

/// The buckets a hash table holding `entries` allocates.
const fn buckets(entries: u64) -> u64 {
    if entries < 4 {
        4
    } else if entries < 8 {
        8
    } else {
        (entries.saturating_mul(8) / 7).next_power_of_two()
    }
}

/// One hash table of `buckets` buckets of `entry` bytes, with its control bytes.
const fn table(buckets: u64, entry: u64) -> u64 {
    allocation(buckets.saturating_mul(entry).saturating_add(buckets).saturating_add(16))
}

/// A hash map or set holding `entries` of `entry` bytes (key and value together): its
/// table and the half-size table that is live while it resizes.
pub const fn hash_table(entries: u64, entry: u64) -> u64 {
    if entries == 0 {
        return 0;
    }
    let buckets = buckets(entries);
    let previous = if buckets > 4 { table(buckets / 2, entry) } else { 0 };
    table(buckets, entry).saturating_add(previous)
}

/// The design's per-entry hash charge, 3.5 × (entry + 1), rounded up. With
/// [`hash_base`] once per map it bounds [`hash_table`] for every count.
pub const fn hash_entry(entry: u64) -> u64 {
    entry.saturating_add(1).saturating_mul(7).div_ceil(2)
}

/// The once-per-map remainder of [`hash_entry`]: the smallest tables are less than 7/8
/// full.
pub const fn hash_base(entry: u64) -> u64 {
    entry.saturating_add(1).saturating_mul(12).saturating_add(96)
}

/// One standard B-tree node for `key` and `value` sizes, as an internal node: eleven
/// keys and values, twelve child pointers, the parent link and lengths, and padding.
const fn btree_node(key: u64, value: u64) -> u64 {
    allocation(key.saturating_add(value).saturating_mul(11).saturating_add(12 + 3 * 8 + 12 * 8))
}

/// A `BTreeMap` (or set, with `value` 0) of `entries`: at most one node per five entries
/// plus the root.
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

/// The type sizes the model charges per unit.
pub mod sizes {
    use super::{
        AnalyticalId, Diagnostic, EvidenceRef, ProviderLimitObservation, Request,
        RequestObservation, size_of,
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
    /// One key-graph node across its three vectors: ID, check bits and parent index.
    pub const KEY_NODE: u64 = ID + size_of::<Option<[u8; 8]>>() as u64 + size_of::<u32>() as u64;
    /// The key graph's open-addressed slots per node: at most four `u32` slots, since the
    /// table is the next power of two at or above twice the nodes.
    pub const KEY_SLOTS: u64 = 4 * size_of::<u32>() as u64;
    /// The grouping order per observation: its first key's node and its `(root, index)`.
    pub const GROUPING_ORDER: u64 = size_of::<u32>() as u64 + size_of::<(u32, u32)>() as u64;
    /// The permutation `Requests::from_unsorted` sorts, per request.
    pub const PERMUTATION: u64 = size_of::<usize>() as u64;
}

/// The most key-graph nodes one observation of `agent` can add, including the
/// artifact-local key of a split part. Codex observations carry one key and no
/// revision-invariant field, so they never split; Claude observations carry a response
/// and a request key and the model invariant, so a split adds one artifact-local key.
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

/// The forward charge of one decoded request-bearing record of `agent`, before its
/// payloads: the decoded record and κ.
pub const fn decoded_request(agent: Agent) -> u64 {
    decoded_record(agent) + kappa(agent)
}

/// The per-record structure one request-bearing record of a dialect needs in each phase
/// after decode, excluding owned payloads.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RequestStructure {
    /// Building observations: the observation vector as it is appended (3 ×) and, for
    /// Claude, the owner and ambiguity state.
    pub construction: u64,
    /// Grouping: the observation, the key graph to the dialect's key bound, the grouping
    /// order, the request vector as presized (33/32 of a request per linked set), and the
    /// request's evidence and alias references. Requests that conflicting keys split past
    /// the presized capacity are charged where they are pushed ([`split_growth`]).
    pub grouping: u64,
    /// Finalize: the request vector, its permutation and its shrunk copy.
    pub finalize: u64,
}

impl RequestStructure {
    /// κ: the largest phase total, which a decode charge covers for every later phase.
    pub const fn kappa(&self) -> u64 {
        max(self.construction, max(self.grouping, self.finalize))
    }
}

/// One request's references per observation: an evidence reference with half an
/// allocation's cost, and one alias with its share of an allocation per key.
const fn request_references(keys: u64) -> u64 {
    sizes::EVIDENCE + ALLOCATION_COST / 2 + keys * (sizes::ID + ALLOCATION_COST)
}

/// The key graph per node: three vectors at 3 × and the slots at 1.5 ×, old and new
/// tables during a rehash.
const KEY_GRAPH_PER_NODE: u64 = 3 * sizes::KEY_NODE + sizes::KEY_SLOTS * 3 / 2;

/// [`RequestStructure`] for `agent`.
pub const fn request_structure(agent: Agent) -> RequestStructure {
    let keys = key_bound(agent);
    let owner = match agent {
        Agent::Claude => crate::adapters::claude_project::OWNER_STATE_PER_RECORD,
        Agent::Codex | Agent::Pi => 0,
    };
    RequestStructure {
        construction: 3 * sizes::OBSERVATION + owner,
        grouping: sizes::OBSERVATION
            + keys * KEY_GRAPH_PER_NODE
            + sizes::GROUPING_ORDER
            + PRESIZED_REQUEST
            + request_references(keys),
        finalize: FINALIZE_PER_OBSERVATION,
    }
}

/// The request vector `reconcile` presizes, per observation: one request per linked set
/// and 1/32 more for parts split from conflicting keys.
const PRESIZED_REQUEST: u64 = (sizes::REQUEST * 33).div_ceil(32);

/// The request vector once split parts push it past its presized capacity: all of it at
/// the doubling rate. Charged where the first such part is pushed.
pub const fn split_growth(requests: u64) -> u64 {
    grown_vec(requests, sizes::REQUEST)
}

/// Finalize per observation: the request vector, which may have grown to twice the
/// requests, its shrunk copy, and the sort permutation.
const FINALIZE_PER_OBSERVATION: u64 = 3 * sizes::REQUEST + sizes::PERMUTATION;

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

/// One diagnostic with `detail` bytes of text and `evidence` references: its row while
/// compaction copies it (3 ×), its text and its evidence vector.
pub const fn diagnostic(detail: u64, evidence: u64) -> u64 {
    3 * sizes::DIAGNOSTIC + allocation(detail) + grown_vec(evidence, sizes::EVIDENCE)
}

/// A new process-interned [`Name`](crate::ledger::names::Name) of `text` bytes: the text,
/// its leaked reference and its B-tree entry.
pub const fn name_intern(text: u64) -> u64 {
    allocation(text) + allocation(size_of::<&str>() as u64) + btree_entry(16, 8)
}

/// A new interned overflow [`Measures`](crate::ledger::tokens::Measures) pattern: its
/// 72-byte row in a doubling vector and its hash-map key.
pub const fn overflow_intern() -> u64 {
    3 * 72 + hash_entry(80)
}

/// Lines up to this capacity grow inside a worker slot; a larger line buffer needs the
/// large-record permit.
pub const SLOT_LINE_CAPACITY: u64 = 4 << 20;

/// zstd windows up to this size decode inside a worker slot.
pub const SLOT_ZSTD_WINDOW: u64 = 8 << 20;

/// The read buffer `sources::reader::decode` wraps each source in.
pub const READ_BUFFER: u64 = 128 << 10;

/// The boxed reader `sources::reader::decode` returns and the decoder struct it wraps.
pub const READER_STATE: u64 = 512;

/// The default thread stack of a decoding worker.
pub const WORKER_STACK: u64 = 2 << 20;

/// zstd's streaming decoder for an 8 MiB window, as `ZSTD_estimateDStreamSize` computes
/// it (the window plus two blocks and wildcopy slack, one block of input buffer and the
/// decoder context), plus the zstd crate's input buffer. The context size is measured by
/// the model harness.
pub const ZSTD_DECODER: u64 =
    SLOT_ZSTD_WINDOW + 3 * (128 << 10) + 64 + ZSTD_CONTEXT + (128 << 10) + 3;

/// `sizeof(ZSTD_DCtx)` rounded up to a KiB: `ZSTD_sizeof_DCtx` reported 95,968 bytes before
/// any frame (zstd 1.5.7, aarch64 macOS), and 8,877,856 after an 8 MiB-window frame
/// started, exactly this context plus the buffers [`ZSTD_DECODER`] adds.
pub const ZSTD_CONTEXT: u64 = 96 << 10;

/// The gzip decoder's state and buffers: the model harness measured 76,368 bytes of Rust
/// heap for `flate2`'s `MultiGzDecoder` beside the read buffer.
pub const GZIP_DECODER: u64 = 80 << 10;

/// One worker slot `b`: the read buffer, the retained line, lines up to
/// [`SLOT_LINE_CAPACITY`] (1.5 × while the buffer grows, plus one parse-owned copy), the
/// thread stack, and the zstd or gzip decoder when discovery found such a source.
pub const fn worker_slot(zstd: bool, gzip: bool) -> u64 {
    let line = SLOT_LINE_CAPACITY * 3 / 2 + SLOT_LINE_CAPACITY;
    let mut slot = allocation(READ_BUFFER)
        + allocation(READER_STATE)
        + allocation(ReadOptions::RETAINED_LINE_CAPACITY as u64)
        + line
        + WORKER_STACK;
    if zstd {
        slot += ZSTD_DECODER;
    }
    if gzip {
        slot += GZIP_DECODER;
    }
    slot
}

/// The large-record permit's need for a line buffer of `line_capacity` above the slot's
/// lines (2.5 ×: growth and a parse-owned copy) and a zstd window above the slot's.
pub const fn large_record(line_capacity: u64, zstd_window: u64) -> u64 {
    let line = if line_capacity > SLOT_LINE_CAPACITY { line_capacity * 5 / 2 } else { 0 };
    let window = if zstd_window > SLOT_ZSTD_WINDOW { zstd_window } else { 0 };
    line + window
}

/// The largest large-record need at the default limits: a 64 MiB record and zstd's
/// default 128 MiB window limit.
pub const LARGEST_LARGE_RECORD: u64 =
    large_record(ReadOptions::DEFAULT_MAX_RECORD_BYTES as u64, 1 << 27);

/// Query and render: per counted request, the list of selected requests and the list of
/// request sizes, each a vector of 8-byte items grown by doubling.
pub const QUERY_PER_REQUEST: u64 = 2 * 3 * 8;

/// Query and render: one session, day or group row while it is accumulated and collected.
pub const QUERY_PER_ROW: u64 = 1024;

/// Query and render: the document's fixed part, its metadata, summaries and maps.
pub const QUERY_FIXED: u64 = 64 << 10;

/// The modeled heap of a query over `requests` counted requests that yields `rows` rows
/// and renders `rendered` bytes, with 3 × the rendered bytes for the output buffer.
pub const fn query(requests: u64, rows: u64, rendered: u64) -> u64 {
    requests * QUERY_PER_REQUEST + rows * QUERY_PER_ROW + 3 * rendered + QUERY_FIXED
}

/// Exact counts at a construction checkpoint, before observations are built.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ConstructionCounts {
    /// Decoded records still held.
    pub decoded_records: u64,
    /// Request-bearing records, each of which becomes at most one observation.
    pub request_records: u64,
    /// Payloads the decoded records and their sources own, by the costing rule.
    pub decoded_payload: u64,
    /// Observations already built by decoding workers, with their payloads.
    pub built_observations: u64,
    /// Provider limit observations.
    pub limit_rows: u64,
    /// The deep size of threads, relationships, diagnostics, gaps and the source table.
    pub metadata: u64,
}

/// The modeled heap while `agent`'s observations are built: the decoded records and
/// their payloads, which each observation may copy once, the construction share of κ per
/// request-bearing record, worker-built observations, limit rows and metadata. The model
/// harness cannot reach this phase from outside the adapters, so this estimate is checked
/// only once slices 5 and 6 charge construction.
pub const fn construction_estimate(agent: Agent, counts: &ConstructionCounts) -> u64 {
    counts.decoded_records * decoded_record(agent)
        + 2 * counts.decoded_payload
        + counts.request_records * request_structure(agent).construction
        + counts.built_observations
        // The base cost of the owner, uuid, model and ambiguity maps and the thread map.
        + 5 * hash_base(32)
        + counts.limit_rows * limit_row()
        + 3 * counts.metadata
}

/// Exact counts at a reconciliation checkpoint, before grouping.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ReconcileCounts {
    /// Request observations.
    pub observations: u64,
    /// The capacity of the observation vector.
    pub observation_capacity: u64,
    /// The observations' owned payloads by the costing rule: spilled keys and
    /// invariants and per-model usage.
    pub observation_payload: u64,
    /// The observations in the largest linked set, which bounds one group's scratch.
    pub largest_group: u64,
    /// Provider limit observations.
    pub limit_rows: u64,
    /// The deep size of everything else the input holds: threads, relationships, tool
    /// actions, links, gaps, diagnostics and the source table.
    pub metadata: u64,
}

/// The scratch one group of `members` observations allocates while its request is
/// built: member, original and revision lists, a split's parts, keys and evidence, and
/// the sets that rank its keys and check its fields.
pub const fn group_scratch(agent: Agent, members: u64) -> u64 {
    if members == 0 {
        return 0;
    }
    let pointer = size_of::<usize>() as u64;
    let keys = members * key_bound(agent);
    sized_vec(members, pointer)
        + 2 * grown_vec(members, pointer)
        + grown_vec(members, 3 * pointer)
        + members * sized_vec(1, pointer)
        + sized_vec(members, 28)
        + sized_vec(members, sizes::EVIDENCE)
        + grown_vec(members, sizes::ID)
        + btree_map(keys, 3 * pointer, 0)
        + 3 * btree_map(members, 2 * pointer, pointer)
}

/// The modeled heap while `agent`'s observations group into requests.
pub const fn grouping_estimate(agent: Agent, counts: &ReconcileCounts) -> u64 {
    let observations = counts.observations;
    let structure = request_structure(agent);
    let per_observation = structure.grouping - sizes::OBSERVATION;
    sized_vec(counts.observation_capacity, sizes::OBSERVATION)
        + 2 * counts.observation_payload
        + observations * per_observation
        // The allocation costs of the graph, order and request vectors, and the smallest slot
        // table before and after its first rehash.
        + 6 * ALLOCATION_COST
        + 2 * table(16, 4)
        + group_scratch(agent, counts.largest_group)
        + counts.limit_rows * limit_row()
        + 3 * counts.metadata
}

/// The modeled heap while `agent`'s requests are sorted and limits and diagnostics are
/// finalized; observations and the key graph are freed by then.
pub const fn finalize_estimate(counts: &ReconcileCounts) -> u64 {
    counts.observations * FINALIZE_PER_OBSERVATION
        + 2 * counts.observation_payload
        + 4 * ALLOCATION_COST
        + counts.limit_rows * limit_row()
        + 3 * counts.metadata
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
    }

    #[test]
    fn vectors_cost_three_times_their_elements_and_at_least_the_minimum_capacity() {
        assert_eq!(grown_vec(0, 8), 0);
        assert_eq!(grown_vec(1, 8), 3 * 4 * 8 + 64);
        assert_eq!(grown_vec(100, 8), 3 * 800 + 64);
        assert_eq!(grown_vec(1, 1), 3 * 8 + 64);
        assert_eq!(sized_vec(10, 16), 176);
    }

    #[test]
    fn hash_tables_follow_their_bucket_counts_and_the_per_entry_rule_bounds_them() {
        assert_eq!(buckets(1), 4);
        assert_eq!(buckets(7), 8);
        assert_eq!(buckets(8), 16);
        assert_eq!(buckets(15), 32);
        for entry in [4, 16, 28, 76] {
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
            }
        }
    }

    #[test]
    fn kappa_is_the_largest_phase_and_matches_the_derivation() {
        for agent in [Agent::Codex, Agent::Claude] {
            let structure = request_structure(agent);
            assert_eq!(
                structure.kappa(),
                structure.construction.max(structure.grouping).max(structure.finalize)
            );
        }
        let codex = request_structure(Agent::Codex);
        let claude = request_structure(Agent::Claude);
        assert_eq!(codex.construction, 3 * sizes::OBSERVATION);
        assert_eq!(
            claude.construction,
            codex.construction + crate::adapters::claude_project::OWNER_STATE_PER_RECORD
        );
        assert_eq!(codex.finalize, 3 * sizes::REQUEST + sizes::PERMUTATION);
        assert_eq!(
            claude.grouping - codex.grouping,
            2 * (KEY_GRAPH_PER_NODE + sizes::ID + ALLOCATION_COST)
        );
        // Indicatively under 1.2 KB per request-bearing record (672 and 1,058 bytes at the
        // current row sizes, recorded in the plan).
        assert!(kappa(Agent::Codex) < kappa(Agent::Claude) && kappa(Agent::Claude) < 1200);
    }

    #[test]
    fn worker_slots_and_the_permit_match_the_design() {
        let plain = worker_slot(false, false);
        assert!((12 << 20..13 << 20).contains(&plain), "{plain}");
        let zstd = worker_slot(true, false) - plain;
        assert!((8 << 20..9 << 20).contains(&zstd), "{zstd}");
        assert_eq!(large_record(4 << 20, 8 << 20), 0);
        assert_eq!(large_record(5 << 20, 16 << 20), (25 << 20) / 2 + (16 << 20));
        assert_eq!(LARGEST_LARGE_RECORD, 160 * (1 << 20) + (128 << 20));
    }
}
