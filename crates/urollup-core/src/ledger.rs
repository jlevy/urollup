//! Ledger and identity: entities, relationships, reconciliation and analytical IDs
//! (design §3).
//!
//! - [`identity`], [`scope`] and [`linking`] derive analytical IDs from recorded keys,
//!   enforce key scope and resolve linked sets (§3.6), over the RFC 8785 subset in
//!   [`canonical_json`].
//! - [`entities`] defines the normalized ledger entities and value bases (§3.1),
//!   [`tokens`] the disjoint token measures they carry (§4.1), and [`names`] the interned
//!   model and effort names.
//! - [`reconcile`] merges observations into logical requests (§3.3), using [`counters`]
//!   for running totals and reporting [`diagnostics`] and [`coverage`].

pub mod canonical_json;
pub mod counters;
pub mod coverage;
pub mod diagnostics;
pub mod entities;
pub mod identity;
pub mod inline_list;
pub mod linking;
pub mod names;
pub mod reconcile;
pub mod scope;
pub mod tokens;
