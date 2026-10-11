//! Apply explicit default policies to recorded request context without changing evidence.

use jiff::Timestamp;

use super::table::{PriceKey, PriceTable, UnpricedReason};
use super::{ComponentPrice, PricingError, TokenRates, price_component, price_tokens};
use crate::ledger::entities::{CompactTimestamp, PricingContext, Request};
use crate::ledger::names::Name;
use crate::ledger::tokens::TokenMeasures;

/// Caller-selected documented defaults. Their use is recorded on every estimate.
/// These describe a list-price valuation, never proof of the user's billing channel.
#[derive(Clone, Copy, Debug)]
pub struct PricingDefaults {
    /// Provider used when the log records none.
    pub provider: Name,
    /// Valuation channel used when the log records none.
    pub channel: Name,
    /// Default service tier.
    pub tier: Name,
    /// Default inference speed.
    pub speed: Name,
    /// Default inference geography.
    pub geography: Name,
}

/// One request's model-specific estimate. Model components are never charged again
/// through the parent request total.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelPrice {
    /// Exact recorded model; no placeholder substitution or prefix matching.
    pub model: Option<Name>,
    /// Dimensions resolved from the explicit valuation policy, not native evidence.
    pub assumed: Vec<&'static str>,
    /// Matched cost and coverage for this model's disjoint usage.
    pub price: ComponentPrice,
}

/// Request valuation retains missing usage and supports an explicit counterfactual date.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RequestPrice {
    /// True when no original usage revision was recorded; not a zero-cost request.
    pub missing_usage: bool,
    /// Individual model estimates, with separate currencies and coverage.
    pub models: Vec<ModelPrice>,
}

/// Price one request, using its date unless the caller explicitly selects a price date.
/// The caller remains responsible for selection/ownership and labeling the date policy.
pub fn price_request(
    request: &Request,
    table: &PriceTable,
    defaults: PricingDefaults,
    price_date: Option<Timestamp>,
) -> Result<RequestPrice, PricingError> {
    let Some(selected) = &request.usage else {
        return Ok(RequestPrice { missing_usage: true, models: Vec::new() });
    };
    let date =
        price_date.or_else(|| request.last_seen.or(request.first_seen).map(CompactTimestamp::get));
    let context = request.pricing.as_deref().cloned().unwrap_or_default();
    let price = |model, usage| price_model(table, defaults, &context, date, model, usage);
    let models = if selected.revision.model_usage.is_empty() {
        vec![price(request.model.as_ref().map(|model| model.name), selected.revision.usage.into())?]
    } else {
        selected
            .revision
            .model_usage
            .iter()
            .map(|component| {
                price(component.model.as_ref().map(|model| model.name), component.usage.into())
            })
            .collect::<Result<Vec<_>, _>>()?
    };
    Ok(RequestPrice { missing_usage: false, models })
}

fn price_model(
    table: &PriceTable,
    defaults: PricingDefaults,
    context: &PricingContext,
    date: Option<Timestamp>,
    model: Option<Name>,
    usage: TokenMeasures,
) -> Result<ModelPrice, PricingError> {
    let model_name = match (context.conflicted, model) {
        (false, Some(name)) => name,
        (conflicted, _) => {
            return Ok(ModelPrice {
                model,
                assumed: Vec::new(),
                price: ComponentPrice {
                    currency: None,
                    tokens: price_tokens(&usage, &TokenRates::default())?,
                    unmatched: Some(if conflicted {
                        UnpricedReason::ConflictingContext
                    } else {
                        UnpricedReason::MissingModel
                    }),
                },
            });
        }
    };
    let mut assumed = Vec::new();
    let mut resolve = |recorded: Option<Name>, default, dimension| {
        recorded.unwrap_or_else(|| {
            assumed.push(dimension);
            default
        })
    };
    let key = PriceKey {
        provider: resolve(context.provider, defaults.provider, "provider"),
        channel: resolve(context.billing_channel, defaults.channel, "billing_channel"),
        tier: resolve(context.service_tier, defaults.tier, "service_tier"),
        speed: resolve(context.speed, defaults.speed, "speed"),
        geography: resolve(context.inference_geo, defaults.geography, "inference_geo"),
        model: model_name,
    };
    Ok(ModelPrice { model, assumed, price: price_component(table, key, date, &usage)? })
}
