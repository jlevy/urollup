//! Token measures: disjoint categories normalized from native counters (design §4.1).
//!
//! Adapters normalize every dialect's usage into disjoint categories: uncached input,
//! cache reads, cache writes by lifetime, output and provider-only tokens, with reasoning
//! recorded as a subset of output that is never added again. Each category is `None` when
//! the source does not report it, which is different from a reported zero.
//!
//! Dialects disagree on what "input" means: Codex and other Responses API clients report
//! input that includes cache reads, while Claude reports input that excludes cache reads
//! and writes. [`InputSemantics`] makes that explicit, so no report has to guess.
//!
//! All arithmetic is checked; `clippy::arithmetic_side_effects` is denied in this module.

#![deny(clippy::arithmetic_side_effects)]

use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::num::NonZeroU16;
use std::sync::{Mutex, OnceLock, PoisonError};

/// Disjoint token categories. `None` means the source does not report the category.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct TokenMeasures {
    /// Input tokens neither read from nor written to a cache.
    pub uncached_input: Option<u64>,
    /// Input tokens read from a cache.
    pub cache_read: Option<u64>,
    /// Input tokens written to a cache with a 5-minute lifetime.
    pub cache_write_5m: Option<u64>,
    /// Input tokens written to a cache with a 1-hour lifetime.
    pub cache_write_1h: Option<u64>,
    /// Input tokens written to a cache whose lifetime the source does not record, such as
    /// a Codex `cache_write_tokens` value or a Claude `cache_creation_input_tokens` without
    /// its `cache_creation` breakdown.
    pub cache_write_unspecified: Option<u64>,
    /// Output tokens, including any reasoning tokens.
    pub output: Option<u64>,
    /// The reasoning subset of `output`; never added to totals a second time.
    pub reasoning: Option<u64>,
    /// Tokens a provider bills in a category no other provider has.
    pub provider_only: Option<u64>,
}

impl TokenMeasures {
    /// Every field's value, in declaration order.
    const fn fields(&self) -> [Option<u64>; 8] {
        [
            self.uncached_input,
            self.cache_read,
            self.cache_write_5m,
            self.cache_write_1h,
            self.cache_write_unspecified,
            self.output,
            self.reasoning,
            self.provider_only,
        ]
    }

    /// Measures from every field's value, in declaration order.
    const fn from_fields(fields: [Option<u64>; 8]) -> Self {
        Self {
            uncached_input: fields[0],
            cache_read: fields[1],
            cache_write_5m: fields[2],
            cache_write_1h: fields[3],
            cache_write_unspecified: fields[4],
            output: fields[5],
            reasoning: fields[6],
            provider_only: fields[7],
        }
    }
}

/// [`TokenMeasures`] as ledger rows store them: eight counters and a presence mask.
///
/// Each `Option<u64>` of a `TokenMeasures` spends 8 bytes on its tag, so the public type
/// takes 128 bytes. Counters that fit in `u32` stay inline (36 bytes, including the
/// `Option` niche). A counter above `u32::MAX` is interned once per distinct pattern so
/// rows do not allocate and values are not truncated.
///
/// An absent counter is stored as zero, so equality is value equality. Ordering and
/// hashing are those of the equivalent `TokenMeasures`: its derived order compares fields
/// in declaration order with an absent value first, and it is part of the canonical
/// observation order.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct Measures {
    /// Compact counters in [`TokenMeasures`] field order, or an overflow intern index in
    /// slot 0 when [`OVERFLOW_BIT`] is set.
    values: [u32; 8],
    /// Bit 0 is always set, which gives the niche; bit `n + 1` marks counter `n` present.
    present: NonZeroU16,
}

const _: () = assert!(std::mem::size_of::<Measures>() == 36);
const _: () = assert!(std::mem::size_of::<Option<Measures>>() == 36);

/// The presence bit of each counter, in field order.
const PRESENCE_BITS: [u16; 8] = [0x2, 0x4, 0x8, 0x10, 0x20, 0x40, 0x80, 0x100];

/// Set on [`Measures::present`] when [`Measures::values`]`[0]` is an overflow intern index.
const OVERFLOW_BIT: u16 = 0x8000;

/// One interned pattern whose counters do not fit in `u32`.
#[derive(Clone, Copy, Eq, Hash, PartialEq)]
struct OverflowRow {
    values: [u64; 8],
    present: u16,
}

/// Distinct overflow patterns, interned for the process like [`super::names::Name`]s.
static OVERFLOW: OnceLock<Mutex<HashMap<OverflowRow, u32>>> = OnceLock::new();
static OVERFLOW_ROWS: Mutex<Vec<OverflowRow>> = Mutex::new(Vec::new());

impl Measures {
    fn fields(&self) -> [Option<u64>; 8] {
        let present = self.present.get();
        if present & OVERFLOW_BIT != 0 {
            let rows = OVERFLOW_ROWS.lock().unwrap_or_else(PoisonError::into_inner);
            let Some(row) = rows.get(self.values[0] as usize) else {
                return [None; 8];
            };
            return fields_from(row.values, row.present);
        }
        fields_from(self.values.map(u64::from), present)
    }
}

fn fields_from(values: [u64; 8], present: u16) -> [Option<u64>; 8] {
    let mut fields = [None; 8];
    for ((field, value), bit) in fields.iter_mut().zip(values).zip(PRESENCE_BITS) {
        if present & bit != 0 {
            *field = Some(value);
        }
    }
    fields
}

fn intern_overflow(row: OverflowRow) -> u32 {
    let mut index = OVERFLOW
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    if let Some(id) = index.get(&row) {
        return *id;
    }
    let mut rows = OVERFLOW_ROWS.lock().unwrap_or_else(PoisonError::into_inner);
    let id = u32::try_from(rows.len()).expect("fewer than 2^32 overflow measure patterns");
    rows.push(row);
    index.insert(row, id);
    id
}

impl Default for Measures {
    fn default() -> Self {
        Self { values: [0; 8], present: NonZeroU16::MIN }
    }
}

impl From<TokenMeasures> for Measures {
    fn from(measures: TokenMeasures) -> Self {
        let mut values = [0_u64; 8];
        let mut present = 1_u16;
        let mut overflow = false;
        for ((field, value), bit) in
            measures.fields().into_iter().zip(&mut values).zip(PRESENCE_BITS)
        {
            if let Some(field) = field {
                *value = field;
                present |= bit;
                overflow |= field > u64::from(u32::MAX);
            }
        }
        if overflow {
            let id = intern_overflow(OverflowRow { values, present });
            let mut compact = [0_u32; 8];
            compact[0] = id;
            return Self {
                values: compact,
                present: NonZeroU16::new(present | OVERFLOW_BIT)
                    .expect("overflow measures keep the niche bit"),
            };
        }
        Self {
            values: values
                .map(|value| u32::try_from(value).expect("the compact path has no u32 overflow")),
            present: NonZeroU16::new(present).expect("compact measures keep the niche bit"),
        }
    }
}

impl From<Measures> for TokenMeasures {
    fn from(measures: Measures) -> Self {
        Self::from_fields(measures.fields())
    }
}

impl Ord for Measures {
    fn cmp(&self, other: &Self) -> Ordering {
        self.fields().cmp(&other.fields())
    }
}

impl PartialOrd for Measures {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Hash for Measures {
    fn hash<H: Hasher>(&self, state: &mut H) {
        TokenMeasures::from(*self).hash(state);
    }
}

impl fmt::Debug for Measures {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        TokenMeasures::from(*self).fmt(f)
    }
}

/// A token counter overflowed.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[error("token counter overflow in {category}")]
pub struct TokenOverflow {
    /// The category that overflowed.
    pub category: &'static str,
}

/// A category name with a getter, so category-wise operations stay in one table.
type Category =
    (&'static str, fn(&TokenMeasures) -> Option<u64>, fn(&mut TokenMeasures) -> &mut Option<u64>);

/// Every category with its getters, reasoning last because it is a subset, not an addend.
const CATEGORIES: [Category; 8] = [
    ("uncached_input", |m| m.uncached_input, |m| &mut m.uncached_input),
    ("cache_read", |m| m.cache_read, |m| &mut m.cache_read),
    ("cache_write_5m", |m| m.cache_write_5m, |m| &mut m.cache_write_5m),
    ("cache_write_1h", |m| m.cache_write_1h, |m| &mut m.cache_write_1h),
    ("cache_write_unspecified", |m| m.cache_write_unspecified, |m| &mut m.cache_write_unspecified),
    ("output", |m| m.output, |m| &mut m.output),
    ("provider_only", |m| m.provider_only, |m| &mut m.provider_only),
    ("reasoning", |m| m.reasoning, |m| &mut m.reasoning),
];

/// The categories summed by [`TokenMeasures::total`]: all but reasoning.
const ADDITIVE: usize = 7;

impl TokenMeasures {
    /// Each category's name and value, with the reasoning subset last.
    pub fn categories(&self) -> [(&'static str, Option<u64>); 8] {
        CATEGORIES.map(|(name, get, _)| (name, get(self)))
    }

    /// Total cache writes across lifetimes, `None` when no lifetime is reported.
    pub fn cache_write(&self) -> Result<Option<u64>, TokenOverflow> {
        sum_known(
            "cache_write",
            [self.cache_write_5m, self.cache_write_1h, self.cache_write_unspecified],
        )
    }

    /// Input including cache reads and writes, `None` when no input category is reported.
    pub fn inclusive_input(&self) -> Result<Option<u64>, TokenOverflow> {
        sum_known(
            "inclusive_input",
            [
                self.uncached_input,
                self.cache_read,
                self.cache_write_5m,
                self.cache_write_1h,
                self.cache_write_unspecified,
            ],
        )
    }

    /// The sum of every reported additive category. Reasoning is a subset of output and is
    /// never added.
    pub fn total(&self) -> Result<Option<u64>, TokenOverflow> {
        let mut values = [None; ADDITIVE];
        for (slot, (_, get, _)) in values.iter_mut().zip(&CATEGORIES[..ADDITIVE]) {
            *slot = get(self);
        }
        sum_known("total", values)
    }

    /// Adds another measure category-wise. A category is `None` only when both are.
    pub fn checked_add(&self, other: &Self) -> Result<Self, TokenOverflow> {
        self.zip_with(other, |category, a, b| match (a, b) {
            (Some(a), Some(b)) => a.checked_add(b).map(Some).ok_or(TokenOverflow { category }),
            (Some(a), None) | (None, Some(a)) => Ok(Some(a)),
            (None, None) => Ok(None),
        })
    }

    /// Combines two measures category by category, passing each category's name.
    pub(crate) fn zip_with<E>(
        &self,
        other: &Self,
        mut combine: impl FnMut(&'static str, Option<u64>, Option<u64>) -> Result<Option<u64>, E>,
    ) -> Result<Self, E> {
        let mut out = Self::default();
        for (name, get, get_mut) in CATEGORIES {
            *get_mut(&mut out) = combine(name, get(self), get(other))?;
        }
        Ok(out)
    }

    /// Whether every category is `None`.
    pub fn is_unreported(&self) -> bool {
        CATEGORIES.iter().all(|(_, get, _)| get(self).is_none())
    }
}

/// What a dialect's native input count includes.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum InputSemantics {
    /// Native input includes cache reads (Codex and the Responses API): uncached input is
    /// the native value minus cache reads.
    IncludesCacheRead,
    /// Native input excludes cache reads and writes (Claude): it is the uncached input.
    ExcludesCache,
}

/// Native input-side counters as a dialect records them.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct NativeInput {
    /// The native input count, whose meaning [`InputSemantics`] gives.
    pub input: Option<u64>,
    /// Native cache reads.
    pub cache_read: Option<u64>,
    /// Native cache writes whose lifetime is not recorded.
    pub cache_write: Option<u64>,
}

/// A native input count smaller than the cache reads it claims to include.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[error("native input {input} is smaller than the {cache_read} cache reads it includes")]
pub struct InputBelowCacheRead {
    /// The native input count.
    pub input: u64,
    /// The native cache reads.
    pub cache_read: u64,
}

/// Derives disjoint input categories from native counters.
///
/// With [`InputSemantics::IncludesCacheRead`], an input smaller than its cache reads is an
/// error rather than a silent zero, so the caller records a diagnostic and keeps the native
/// values.
pub fn normalize_input(
    semantics: InputSemantics,
    native: NativeInput,
) -> Result<TokenMeasures, InputBelowCacheRead> {
    let uncached_input = match (semantics, native.input, native.cache_read) {
        (InputSemantics::IncludesCacheRead, Some(input), Some(cache_read)) => {
            Some(input.checked_sub(cache_read).ok_or(InputBelowCacheRead { input, cache_read })?)
        }
        (InputSemantics::IncludesCacheRead | InputSemantics::ExcludesCache, input, _) => input,
    };
    Ok(TokenMeasures {
        uncached_input,
        cache_read: native.cache_read,
        cache_write_unspecified: native.cache_write,
        ..TokenMeasures::default()
    })
}

fn sum_known<const N: usize>(
    category: &'static str,
    values: [Option<u64>; N],
) -> Result<Option<u64>, TokenOverflow> {
    let mut total: Option<u64> = None;
    for value in values.into_iter().flatten() {
        total = Some(total.unwrap_or(0).checked_add(value).ok_or(TokenOverflow { category })?);
    }
    Ok(total)
}

#[cfg(test)]
mod tests {
    use std::cmp::Ordering;
    use std::hash::{BuildHasher, BuildHasherDefault, DefaultHasher};

    use proptest::prelude::*;

    use super::{
        InputBelowCacheRead, InputSemantics, Measures, NativeInput, TokenMeasures, TokenOverflow,
        normalize_input,
    };

    /// Measures from a small universe, so equal and adjacent values are common, with the
    /// extremes of the counter range.
    fn arbitrary_measures() -> impl Strategy<Value = TokenMeasures> {
        let counter = prop::option::of(prop_oneof![0u64..3, Just(u64::MAX), any::<u64>()]);
        prop::array::uniform8(counter).prop_map(TokenMeasures::from_fields)
    }

    proptest! {
        #[test]
        fn compact_measures_order_compare_and_hash_like_token_measures(
            left in arbitrary_measures(),
            other in arbitrary_measures(),
            shared in prop::array::uniform8(any::<bool>()),
        ) {
            // Fields `right` shares with `left` make equal and nearly equal pairs common.
            let mut fields = other.fields();
            for ((field, left_field), shared) in fields.iter_mut().zip(left.fields()).zip(shared) {
                if shared {
                    *field = left_field;
                }
            }
            let right = TokenMeasures::from_fields(fields);
            let (compact_left, compact_right) = (Measures::from(left), Measures::from(right));
            prop_assert_eq!(TokenMeasures::from(compact_left), left);
            prop_assert_eq!(compact_left.cmp(&compact_right), left.cmp(&right));
            prop_assert_eq!(compact_left == compact_right, left == right);
            let hasher = BuildHasherDefault::<DefaultHasher>::default();
            prop_assert_eq!(hasher.hash_one(compact_left), hasher.hash_one(left));
            prop_assert_eq!(format!("{compact_left:?}"), format!("{left:?}"));
        }
    }

    #[test]
    fn counters_above_u32_round_trip_and_intern_once() {
        let original = TokenMeasures {
            uncached_input: Some(u64::from(u32::MAX).saturating_add(1)),
            output: Some(u64::MAX),
            ..TokenMeasures::default()
        };
        let first = Measures::from(original);
        let second = Measures::from(original);
        assert_eq!(TokenMeasures::from(first), original);
        assert_eq!(first, second);
        assert_eq!(first.cmp(&second), Ordering::Equal);
    }

    #[test]
    fn compact_measures_default_to_unreported() {
        assert_eq!(TokenMeasures::from(Measures::default()), TokenMeasures::default());
        assert_eq!(Measures::from(TokenMeasures::default()), Measures::default());
    }

    #[test]
    fn codex_input_includes_cache_reads_and_claude_input_does_not() {
        // The same logical usage as the two dialects record it (qm's mixed-semantics defect:
        // adding cache reads to Codex input counts them twice).
        let codex = normalize_input(
            InputSemantics::IncludesCacheRead,
            NativeInput { input: Some(1_000), cache_read: Some(800), cache_write: None },
        )
        .unwrap();
        let claude = normalize_input(
            InputSemantics::ExcludesCache,
            NativeInput { input: Some(200), cache_read: Some(800), cache_write: None },
        )
        .unwrap();
        assert_eq!(codex, claude);
        assert_eq!(codex.uncached_input, Some(200));
        assert_eq!(codex.inclusive_input().unwrap(), Some(1_000));
    }

    #[test]
    fn inclusive_input_below_its_cache_reads_is_an_error_not_zero() {
        let error = normalize_input(
            InputSemantics::IncludesCacheRead,
            NativeInput { input: Some(10), cache_read: Some(11), cache_write: None },
        )
        .unwrap_err();
        assert_eq!(error, InputBelowCacheRead { input: 10, cache_read: 11 });
    }

    #[test]
    fn reasoning_is_never_added_to_the_total() {
        let measures = TokenMeasures {
            uncached_input: Some(5),
            cache_read: Some(7),
            cache_write_1h: Some(3),
            output: Some(11),
            reasoning: Some(9),
            ..TokenMeasures::default()
        };
        assert_eq!(measures.total().unwrap(), Some(26));
        assert_eq!(measures.cache_write().unwrap(), Some(3));
    }

    #[test]
    fn unreported_categories_stay_unknown_rather_than_zero() {
        let empty = TokenMeasures::default();
        assert!(empty.is_unreported());
        assert_eq!(empty.total().unwrap(), None);
        let output_only = TokenMeasures { output: Some(0), ..TokenMeasures::default() };
        assert_eq!(output_only.total().unwrap(), Some(0));
        assert_eq!(output_only.inclusive_input().unwrap(), None);
    }

    #[test]
    fn addition_is_checked() {
        let big = TokenMeasures { output: Some(u64::MAX), ..TokenMeasures::default() };
        let one =
            TokenMeasures { output: Some(1), cache_read: Some(2), ..TokenMeasures::default() };
        assert_eq!(big.checked_add(&one), Err(TokenOverflow { category: "output" }));
        let sum = one
            .checked_add(&TokenMeasures { output: Some(4), ..TokenMeasures::default() })
            .unwrap();
        assert_eq!((sum.output, sum.cache_read, sum.uncached_input), (Some(5), Some(2), None));
        let totals = TokenMeasures {
            uncached_input: Some(u64::MAX),
            output: Some(1),
            ..TokenMeasures::default()
        };
        assert_eq!(totals.total(), Err(TokenOverflow { category: "total" }));
    }
}
