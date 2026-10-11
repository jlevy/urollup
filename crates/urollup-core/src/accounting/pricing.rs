//! Request-level list-price calculations, separate from source-reported charges.

use serde::Serialize;

use super::money::{Decimal, DecimalError};
use crate::ledger::tokens::TokenMeasures;

pub mod request;
pub mod table;
pub mod totals;

/// One model component priced before aggregation; unmatched values retain coverage.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComponentPrice {
    /// Currency of the selected rate, absent if no rate could be selected.
    pub currency: Option<String>,
    /// Known priced/unpriced tokens and unknown quantities.
    pub tokens: TokenPrice,
    /// Why the rate lookup failed, separate from missing individual category rates.
    pub unmatched: Option<table::UnpricedReason>,
}

/// Match a resolved pricing key, then price its disjoint token categories.
/// Missing counts cannot select a cheaper context band from a partial input sum.
pub fn price_component(
    table: &table::PriceTable,
    key: table::PriceKey,
    date: Option<jiff::Timestamp>,
    usage: &TokenMeasures,
) -> Result<ComponentPrice, PricingError> {
    let known_writes = usage.cache_write_unspecified.is_some()
        || (usage.cache_write_5m.is_some() && usage.cache_write_1h.is_some());
    let inclusive_input =
        if usage.uncached_input.is_some() && usage.cache_read.is_some() && known_writes {
            usage.inclusive_input().map_err(|_| PricingError::TokenOverflow)?
        } else {
            None
        };
    match table.lookup(key, date, inclusive_input) {
        Ok(row) => Ok(ComponentPrice {
            currency: Some(row.currency.clone()),
            tokens: price_tokens(usage, &row.rates)?,
            unmatched: None,
        }),
        Err(reason) => Ok(ComponentPrice {
            currency: None,
            tokens: price_tokens(usage, &TokenRates::default())?,
            unmatched: Some(reason),
        }),
    }
}

/// Decimal currency units per million tokens, after matching a reviewed rate row.
///
/// An unspecified write rate is explicit table data: it is not universally the
/// five-minute rate, since providers use different cache lifetimes and defaults.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TokenRates {
    /// Ordinary input, excluding reads and writes.
    pub uncached_input: Option<Decimal>,
    /// Cache reads.
    pub cache_read: Option<Decimal>,
    /// Five-minute cache writes.
    pub cache_write_5m: Option<Decimal>,
    /// One-hour cache writes.
    pub cache_write_1h: Option<Decimal>,
    /// Writes with no recorded lifetime, when the matched policy supports them.
    pub cache_write_unspecified: Option<Decimal>,
    /// Output including its reasoning subset.
    pub output: Option<Decimal>,
    /// A separately documented provider-specific token category.
    pub provider_only: Option<Decimal>,
}

/// A priced subtotal, not a claim that every token or charge is known.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
pub struct TokenPrice {
    /// Sum for categories with known counts and rates, in the matched row's currency.
    pub known_amount: Decimal,
    /// Known tokens to which a rate was applied.
    pub priced_tokens: u64,
    /// Known tokens without a rate; never treated as free tokens.
    pub unpriced_tokens: u64,
    /// Missing native counts: their token quantity and monetary value are unknown.
    pub unknown_usage: Vec<&'static str>,
}

/// Arithmetic fails explicitly rather than publishing a rounded or saturated total.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum PricingError {
    /// Exact monetary arithmetic overflowed.
    #[error(transparent)]
    Decimal(#[from] DecimalError),
    /// The known token population cannot be represented.
    #[error("pricing token count overflow")]
    TokenOverflow,
}

/// Price disjoint counts only. Call this per request/model component, before rollup.
///
/// Unknown rate matching is represented by empty rates. Missing usage remains separate
/// from known unpriced tokens. Reasoning is a subset of output, never another charge.
pub fn price_tokens(usage: &TokenMeasures, rates: &TokenRates) -> Result<TokenPrice, PricingError> {
    let mut result = TokenPrice::default();
    for (count, rate) in [
        (usage.uncached_input, rates.uncached_input),
        (usage.cache_read, rates.cache_read),
        (usage.cache_write_5m, rates.cache_write_5m),
        (usage.cache_write_1h, rates.cache_write_1h),
        (usage.cache_write_unspecified, rates.cache_write_unspecified),
        (usage.output, rates.output),
        (usage.provider_only, rates.provider_only),
    ] {
        let Some(count) = count else { continue };
        if let Some(rate) = rate {
            result.known_amount = result.known_amount.checked_add(rate.per_million(count)?)?;
            result.priced_tokens =
                result.priced_tokens.checked_add(count).ok_or(PricingError::TokenOverflow)?;
        } else {
            result.unpriced_tokens =
                result.unpriced_tokens.checked_add(count).ok_or(PricingError::TokenOverflow)?;
        }
    }
    for (name, count) in [
        ("uncached_input", usage.uncached_input),
        ("cache_read", usage.cache_read),
        ("output", usage.output),
    ] {
        if count.is_none() {
            result.unknown_usage.push(name);
        }
    }
    // A flat write count and a complete lifetime breakdown are alternative forms.
    // A partial breakdown cannot prove the missing part is zero.
    if usage.cache_write_unspecified.is_none()
        && (usage.cache_write_5m.is_none() || usage.cache_write_1h.is_none())
    {
        result.unknown_usage.push("cache_write");
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::{TokenRates, price_tokens};
    use crate::ledger::tokens::TokenMeasures;

    fn rates() -> TokenRates {
        TokenRates {
            uncached_input: Some("2".parse().expect("synthetic rate")),
            cache_read: Some("0.2".parse().expect("synthetic rate")),
            cache_write_5m: Some("2.5".parse().expect("synthetic rate")),
            cache_write_1h: Some("4".parse().expect("synthetic rate")),
            cache_write_unspecified: Some("2.5".parse().expect("synthetic rate")),
            output: Some("10".parse().expect("synthetic rate")),
            provider_only: None,
        }
    }

    fn usage() -> TokenMeasures {
        TokenMeasures {
            uncached_input: Some(1_000),
            cache_read: Some(2_000),
            cache_write_5m: Some(3_000),
            cache_write_1h: Some(4_000),
            cache_write_unspecified: Some(5_000),
            output: Some(6_000),
            reasoning: Some(1_000),
            provider_only: None,
        }
    }

    #[test]
    fn disjoint_categories_price_exactly_and_reasoning_is_not_added() {
        let priced = price_tokens(&usage(), &rates()).unwrap();
        assert_eq!(priced.known_amount.to_string(), "0.0984");
        assert_eq!(priced.priced_tokens, 21_000);
        assert_eq!(priced.unpriced_tokens, 0);
        assert!(priced.unknown_usage.is_empty());
    }

    #[test]
    fn a_missing_rate_leaves_known_tokens_unpriced() {
        let mut rates = rates();
        rates.cache_read = None;
        let priced = price_tokens(&usage(), &rates).unwrap();
        assert_eq!(priced.known_amount.to_string(), "0.098");
        assert_eq!(priced.priced_tokens, 19_000);
        assert_eq!(priced.unpriced_tokens, 2_000);
        assert!(priced.unknown_usage.is_empty());
    }

    #[test]
    fn absent_counts_remain_unknown_and_unspecified_writes_use_only_their_own_rate() {
        let mut usage = usage();
        usage.uncached_input = None;
        usage.cache_write_5m = None;
        usage.cache_write_1h = None;
        let mut rates = rates();
        rates.cache_write_unspecified = None;
        let priced = price_tokens(&usage, &rates).unwrap();
        assert_eq!(priced.known_amount.to_string(), "0.0604");
        assert_eq!(priced.unpriced_tokens, 5_000);
        assert_eq!(priced.unknown_usage, ["uncached_input"]);
    }

    #[test]
    fn partial_input_cannot_choose_a_cheaper_band_and_unmatched_tokens_are_retained() {
        use super::table::{PriceKey, PriceRow, PriceTable, UnpricedReason};
        use crate::ledger::names::Name;
        let key = PriceKey {
            provider: Name::new("test"),
            channel: Name::new("api"),
            model: Name::new("test"),
            tier: Name::new("standard"),
            speed: Name::new("standard"),
            geography: Name::new("global"),
        };
        let date = "2026-01-01T00:00:00Z".parse().unwrap();
        let base = PriceRow {
            key,
            effective_from: date,
            effective_until: None,
            input_over: None,
            currency: "USD".to_owned(),
            rates: rates(),
        };
        let band = PriceRow { input_over: Some(200_000), ..base.clone() };
        let table = PriceTable::new(vec![base, band]).unwrap();
        let mut usage = usage();
        usage.cache_read = None;
        let priced = super::price_component(&table, key, Some(date), &usage).unwrap();
        assert_eq!(priced.unmatched, Some(UnpricedReason::UnknownContextSize));
        assert_eq!(priced.currency, None);
        assert_eq!(priced.tokens.priced_tokens, 0);
        assert_eq!(priced.tokens.unpriced_tokens, 19_000);
        assert_eq!(priced.tokens.unknown_usage, ["cache_read"]);
        usage.cache_read = Some(2_000);
        let priced = super::price_component(&table, key, Some(date), &usage).unwrap();
        assert_eq!(priced.unmatched, None);
        assert_eq!(priced.currency.as_deref(), Some("USD"));
        assert_eq!(priced.tokens.known_amount.to_string(), "0.0984");
    }
}
