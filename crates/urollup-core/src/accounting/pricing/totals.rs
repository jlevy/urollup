//! Additive cost rollups with separate currencies and disjoint request coverage.

use std::collections::BTreeMap;

use serde::Serialize;

use super::PricingError;
use super::request::RequestPrice;
use crate::accounting::money::Decimal;

/// Known list-price subtotals and coverage. Unknown amounts are never replaced by zero.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
pub struct CostTotals {
    /// Known amounts, keyed by currency; no exchange-rate conversion is performed.
    pub known_amounts: BTreeMap<String, Decimal>,
    /// Requests with complete known costs and no default assumptions.
    pub priced_requests: u64,
    /// Requests with complete known costs that used explicit pricing-policy defaults.
    pub default_assumed_requests: u64,
    /// Requests with a known priced token population and missing pricing/usage elsewhere.
    pub partially_priced_requests: u64,
    /// Requests for which no complete cost or priced token population is available.
    pub unpriced_requests: u64,
    /// Known tokens assigned rates, including tokens at an explicitly zero rate.
    pub priced_tokens: u64,
    /// Known tokens without rates.
    pub unpriced_tokens: u64,
    /// Requests with unknown native usage quantities, overlapping request coverage above.
    pub unknown_usage_requests: u64,
}

impl CostTotals {
    /// Add one counted/selected request exactly once. Ownership selection stays upstream.
    /// On arithmetic failure, this accumulator is left unchanged.
    pub fn add(&mut self, request: &RequestPrice) -> Result<(), PricingError> {
        let mut next = self.clone();
        next.add_inner(request)?;
        *self = next;
        Ok(())
    }

    fn add_inner(&mut self, request: &RequestPrice) -> Result<(), PricingError> {
        let unknown_usage = request.missing_usage
            || request.models.iter().any(|model| !model.price.tokens.unknown_usage.is_empty());
        let incomplete = unknown_usage
            || request.models.is_empty()
            || request.models.iter().any(|model| {
                model.price.unmatched.is_some() || model.price.tokens.unpriced_tokens > 0
            });
        let mut has_priced_tokens = false;
        let mut assumed = false;
        for model in &request.models {
            let priced = &model.price;
            if let Some(currency) = &priced.currency {
                let amount = self.known_amounts.entry(currency.clone()).or_default();
                *amount = amount.checked_add(priced.tokens.known_amount)?;
            }
            self.priced_tokens = add(self.priced_tokens, priced.tokens.priced_tokens)?;
            self.unpriced_tokens = add(self.unpriced_tokens, priced.tokens.unpriced_tokens)?;
            has_priced_tokens |= priced.tokens.priced_tokens > 0;
            assumed |= !model.assumed.is_empty();
        }
        let population = if incomplete {
            if has_priced_tokens {
                &mut self.partially_priced_requests
            } else {
                &mut self.unpriced_requests
            }
        } else if assumed {
            &mut self.default_assumed_requests
        } else {
            &mut self.priced_requests
        };
        *population = add(*population, 1)?;
        if unknown_usage {
            self.unknown_usage_requests = add(self.unknown_usage_requests, 1)?;
        }
        Ok(())
    }
}

fn add(left: u64, right: u64) -> Result<u64, PricingError> {
    left.checked_add(right).ok_or(PricingError::TokenOverflow)
}

#[cfg(test)]
mod tests {
    use super::CostTotals;
    use crate::accounting::pricing::request::{ModelPrice, RequestPrice};
    use crate::accounting::pricing::{ComponentPrice, TokenPrice};

    fn model(currency: &str) -> ModelPrice {
        ModelPrice {
            model: None,
            assumed: Vec::new(),
            price: ComponentPrice {
                currency: Some(currency.to_owned()),
                unmatched: None,
                tokens: TokenPrice {
                    known_amount: "0.25".parse().expect("synthetic amount"),
                    priced_tokens: 100,
                    ..TokenPrice::default()
                },
            },
        }
    }

    #[test]
    fn currencies_are_separate_and_model_components_do_not_inflate_request_counts() {
        let mut total = CostTotals::default();
        total
            .add(&RequestPrice { missing_usage: false, models: vec![model("USD"), model("EUR")] })
            .unwrap();
        assert_eq!(total.priced_requests, 1);
        assert_eq!(total.priced_tokens, 200);
        assert_eq!(total.known_amounts["USD"].to_string(), "0.25");
        assert_eq!(total.known_amounts["EUR"].to_string(), "0.25");
        let mut assumed = model("USD");
        assumed.assumed.push("service_tier");
        total.add(&RequestPrice { missing_usage: false, models: vec![assumed] }).unwrap();
        assert_eq!(total.default_assumed_requests, 1);
        let mut partial = model("USD");
        partial.price.tokens.unpriced_tokens = 10;
        partial.price.tokens.unknown_usage.push("cache_read");
        total.add(&RequestPrice { missing_usage: false, models: vec![partial] }).unwrap();
        total.add(&RequestPrice { missing_usage: true, models: Vec::new() }).unwrap();
        assert_eq!(total.partially_priced_requests, 1);
        assert_eq!(total.unpriced_requests, 1);
        assert_eq!(total.unknown_usage_requests, 2);
        assert_eq!(total.unpriced_tokens, 10);
        assert_eq!(total.known_amounts["USD"].to_string(), "0.75");
    }

    #[test]
    fn failed_accumulation_does_not_partially_change_totals() {
        let mut total = CostTotals { priced_requests: u64::MAX, ..CostTotals::default() };
        let before = total.clone();
        assert!(
            total.add(&RequestPrice { missing_usage: false, models: vec![model("USD")] }).is_err()
        );
        assert_eq!(total, before);
    }
}
