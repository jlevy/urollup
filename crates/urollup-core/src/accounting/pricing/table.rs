//! Validated, offline date and context-band matching for explicit model/rate keys.

use std::collections::BTreeMap;

use jiff::Timestamp;

use super::TokenRates;
use crate::ledger::names::Name;

/// Every recorded/default-resolved dimension must match exactly.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct PriceKey {
    /// Provider, not the coding agent.
    pub provider: Name,
    /// Billing channel such as the first-party API.
    pub channel: Name,
    /// Exact model ID or an explicitly expanded table alias.
    pub model: Name,
    /// Service tier.
    pub tier: Name,
    /// Inference speed, independent of service tier.
    pub speed: Name,
    /// Inference geography, independent of machine timezone.
    pub geography: Name,
}

/// One reviewed or configured interval and whole-request context band.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PriceRow {
    /// Exact match dimensions.
    pub key: PriceKey,
    /// Inclusive UTC lower bound.
    pub effective_from: Timestamp,
    /// Exclusive UTC upper bound, or open-ended when the source states no end.
    pub effective_until: Option<Timestamp>,
    /// This band applies only when inclusive input exceeds the threshold.
    pub input_over: Option<u64>,
    /// Three-letter uppercase currency; never inferred or converted.
    pub currency: String,
    /// Disjoint per-million rates.
    pub rates: TokenRates,
}

/// Malformed or ambiguous table data is rejected before any request is priced.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum PriceTableError {
    /// The currency is not a three-letter uppercase code.
    #[error("invalid pricing currency for model {0}")]
    InvalidCurrency(Name),
    /// An interval ends at or before its start.
    #[error("invalid effective pricing interval for model {0}")]
    InvalidInterval(Name),
    /// Two rows could select different rates for the same request.
    #[error("overlapping effective rates for model {0}")]
    OverlappingRates(Name),
}

/// Why an exact lookup cannot safely select a rate.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UnpricedReason {
    /// No exact model identity is recorded.
    MissingModel,
    /// Original records disagree about a pricing dimension.
    ConflictingContext,
    /// Historical pricing requires a recorded request timestamp.
    MissingDate,
    /// A relevant context band exists but inclusive input is not known.
    UnknownContextSize,
    /// No exact model/context key and dated interval match.
    NoMatchingRate,
}

/// A validated offline index. Lookup allocates no strings or per-request rate rows.
#[derive(Clone, Debug)]
pub struct PriceTable {
    rows: BTreeMap<PriceKey, Vec<PriceRow>>,
}

impl PriceTable {
    /// Validate all rows and index them by exact dimensions, including explicit aliases.
    pub fn new(rows: Vec<PriceRow>) -> Result<Self, PriceTableError> {
        let mut indexed: BTreeMap<PriceKey, Vec<PriceRow>> = BTreeMap::new();
        for row in rows {
            if row.currency.len() != 3
                || !row.currency.bytes().all(|byte| byte.is_ascii_uppercase())
            {
                return Err(PriceTableError::InvalidCurrency(row.key.model));
            }
            if row.effective_until.is_some_and(|until| until <= row.effective_from) {
                return Err(PriceTableError::InvalidInterval(row.key.model));
            }
            indexed.entry(row.key).or_default().push(row);
        }
        for rows in indexed.values_mut() {
            rows.sort_by_key(|row| (row.input_over, row.effective_from));
            for pair in rows.windows(2) {
                if pair[0].input_over == pair[1].input_over
                    && pair[0].effective_until.is_none_or(|until| pair[1].effective_from < until)
                {
                    return Err(PriceTableError::OverlappingRates(pair[0].key.model));
                }
            }
        }
        Ok(Self { rows: indexed })
    }

    /// Match at a request date (or an explicitly chosen counterfactual date).
    /// Inclusive input must include ordinary input, cache reads and cache writes.
    pub fn lookup(
        &self,
        key: PriceKey,
        date: Option<Timestamp>,
        inclusive_input: Option<u64>,
    ) -> Result<&PriceRow, UnpricedReason> {
        let date = date.ok_or(UnpricedReason::MissingDate)?;
        let rows = self.rows.get(&key).ok_or(UnpricedReason::NoMatchingRate)?;
        let active = rows.iter().filter(|row| {
            row.effective_from <= date && row.effective_until.is_none_or(|until| date < until)
        });
        let mut selected: Option<&PriceRow> = None;
        for row in active {
            let eligible = if let Some(threshold) = row.input_over {
                inclusive_input.ok_or(UnpricedReason::UnknownContextSize)? > threshold
            } else {
                true
            };
            if eligible && selected.is_none_or(|previous| row.input_over > previous.input_over) {
                selected = Some(row);
            }
        }
        selected.ok_or(UnpricedReason::NoMatchingRate)
    }
}

#[cfg(test)]
mod tests {
    use super::{PriceKey, PriceRow, PriceTable, UnpricedReason};
    use crate::accounting::pricing::TokenRates;
    use crate::ledger::names::Name;

    fn row(from: &str, until: Option<&str>, threshold: Option<u64>, rate: &str) -> PriceRow {
        PriceRow {
            key: PriceKey {
                provider: Name::new("synthetic-provider"),
                channel: Name::new("api"),
                model: Name::new("exact-model"),
                tier: Name::new("standard"),
                speed: Name::new("standard"),
                geography: Name::new("global"),
            },
            effective_from: from.parse().expect("synthetic timestamp"),
            effective_until: until.map(|date| date.parse().expect("synthetic timestamp")),
            input_over: threshold,
            currency: "USD".to_owned(),
            rates: TokenRates {
                uncached_input: Some(rate.parse().expect("synthetic rate")),
                ..TokenRates::default()
            },
        }
    }

    #[test]
    fn dates_are_half_open_and_bands_apply_only_above_the_threshold() {
        let base = row("2026-01-01T00:00:00Z", Some("2026-02-01T00:00:00Z"), None, "2");
        let key = base.key;
        let table = PriceTable::new(vec![
            base,
            row("2026-01-01T00:00:00Z", Some("2026-02-01T00:00:00Z"), Some(200_000), "4"),
            row("2026-02-01T00:00:00Z", None, None, "3"),
        ])
        .unwrap();
        for (date, input, expected) in [
            ("2026-01-01T00:00:00Z", 200_000, "2"),
            ("2026-01-31T23:59:59Z", 200_001, "4"),
            ("2026-02-01T00:00:00Z", 200_001, "3"),
        ] {
            let matched = table.lookup(key, Some(date.parse().unwrap()), Some(input)).unwrap();
            assert_eq!(matched.rates.uncached_input.unwrap().to_string(), expected);
        }
        assert_eq!(table.lookup(key, None, Some(1)), Err(UnpricedReason::MissingDate));
        assert_eq!(
            table.lookup(key, Some("2026-01-01T00:00:00Z".parse().unwrap()), None),
            Err(UnpricedReason::UnknownContextSize)
        );
        let wrong = PriceKey { model: Name::new("exact-model-extra"), ..key };
        assert_eq!(
            table.lookup(wrong, Some("2026-01-01T00:00:00Z".parse().unwrap()), Some(1)),
            Err(UnpricedReason::NoMatchingRate)
        );
    }

    #[test]
    fn ambiguous_ranges_invalid_currencies_and_reversed_dates_are_rejected() {
        let base = row("2026-01-01T00:00:00Z", None, None, "2");
        assert!(PriceTable::new(vec![base.clone(), base.clone()]).is_err());
        let mut invalid = base.clone();
        invalid.currency = "usd".to_owned();
        assert!(PriceTable::new(vec![invalid]).is_err());
        let mut invalid = base;
        invalid.effective_until = Some(invalid.effective_from);
        assert!(PriceTable::new(vec![invalid]).is_err());
    }

    #[test]
    fn highest_band_wins_and_no_dimension_is_silently_substituted() {
        let base = row("2026-01-01T00:00:00Z", None, None, "1");
        let key = base.key;
        let date = Some("2026-01-01T00:00:00Z".parse().unwrap());
        let table = PriceTable::new(vec![
            row("2026-01-01T00:00:00Z", None, Some(400_000), "3"),
            base,
            row("2026-01-01T00:00:00Z", None, Some(200_000), "2"),
        ])
        .unwrap();
        assert_eq!(
            table
                .lookup(key, date, Some(400_001))
                .unwrap()
                .rates
                .uncached_input
                .unwrap()
                .to_string(),
            "3"
        );
        let unknown = Name::new("unknown");
        for different in [
            PriceKey { provider: unknown, ..key },
            PriceKey { channel: unknown, ..key },
            PriceKey { model: unknown, ..key },
            PriceKey { tier: unknown, ..key },
            PriceKey { speed: unknown, ..key },
            PriceKey { geography: unknown, ..key },
        ] {
            assert_eq!(table.lookup(different, date, Some(1)), Err(UnpricedReason::NoMatchingRate));
        }
        assert_eq!(
            table.lookup(key, Some("2025-12-31T23:59:59Z".parse().unwrap()), Some(1)),
            Err(UnpricedReason::NoMatchingRate)
        );
    }
}
