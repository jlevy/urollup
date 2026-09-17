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
    use super::{
        InputBelowCacheRead, InputSemantics, NativeInput, TokenMeasures, TokenOverflow,
        normalize_input,
    };

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
