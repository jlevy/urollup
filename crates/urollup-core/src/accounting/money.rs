//! Exact nonnegative decimal arithmetic for rates and monetary estimates.

use std::fmt;
use std::str::FromStr;

use serde::de::{Error, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// A canonical decimal, with at most 38 fractional digits and a `u128` coefficient.
///
/// All operations fail on overflow rather than rounding. Currency is deliberately not
/// part of this number; monetary aggregators must partition by currency before adding.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct Decimal {
    coefficient: u128,
    scale: u32,
}

/// Invalid decimal syntax or arithmetic outside the exact representable range.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum DecimalError {
    /// Rates must be unsigned decimal strings, not signs, exponents or floats.
    #[error("expected a nonnegative exact decimal string")]
    Invalid,
    /// No rounded or saturated amount is returned.
    #[error("exact decimal arithmetic overflow")]
    Overflow,
}

impl Decimal {
    fn canonical(mut coefficient: u128, mut scale: u32) -> Result<Self, DecimalError> {
        if coefficient == 0 {
            return Ok(Self::default());
        }
        while scale > 0 && coefficient % 10 == 0 {
            coefficient /= 10;
            scale = scale.checked_sub(1).ok_or(DecimalError::Overflow)?;
        }
        if scale > 38 {
            return Err(DecimalError::Overflow);
        }
        Ok(Self { coefficient, scale })
    }

    /// Add exact values of the same currency, without intermediate floating point.
    pub fn checked_add(self, other: Self) -> Result<Self, DecimalError> {
        let scale = self.scale.max(other.scale);
        let align = |value: Self| {
            let exponent = scale.checked_sub(value.scale).ok_or(DecimalError::Overflow)?;
            let power = 10_u128.checked_pow(exponent).ok_or(DecimalError::Overflow)?;
            value.coefficient.checked_mul(power).ok_or(DecimalError::Overflow)
        };
        Self::canonical(
            align(self)?.checked_add(align(other)?).ok_or(DecimalError::Overflow)?,
            scale,
        )
    }

    /// Apply an exact decimal multiplier such as a documented geography modifier.
    pub fn checked_mul(self, other: Self) -> Result<Self, DecimalError> {
        Self::canonical(
            self.coefficient.checked_mul(other.coefficient).ok_or(DecimalError::Overflow)?,
            self.scale.checked_add(other.scale).ok_or(DecimalError::Overflow)?,
        )
    }

    /// Price a token count at this rate per million tokens.
    pub fn per_million(self, tokens: u64) -> Result<Self, DecimalError> {
        Self::canonical(
            self.coefficient.checked_mul(u128::from(tokens)).ok_or(DecimalError::Overflow)?,
            self.scale.checked_add(6).ok_or(DecimalError::Overflow)?,
        )
    }
}

impl FromStr for Decimal {
    type Err = DecimalError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let (whole, fractional) =
            value.split_once('.').map_or((value, None), |(a, b)| (a, Some(b)));
        let digits =
            |text: &str| !text.is_empty() && text.bytes().all(|byte| byte.is_ascii_digit());
        if !digits(whole) || fractional.is_some_and(|part| !digits(part)) {
            return Err(DecimalError::Invalid);
        }
        let scale =
            u32::try_from(fractional.map_or(0, str::len)).map_err(|_| DecimalError::Overflow)?;
        let coefficient = whole.bytes().chain(fractional.unwrap_or("").bytes()).try_fold(
            0_u128,
            |sum, byte| {
                sum.checked_mul(10)
                    .and_then(|sum| sum.checked_add(u128::from(byte - b'0')))
                    .ok_or(DecimalError::Overflow)
            },
        )?;
        Self::canonical(coefficient, scale)
    }
}

impl fmt::Display for Decimal {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let digits = self.coefficient.to_string();
        let scale = usize::try_from(self.scale).map_err(|_| fmt::Error)?;
        if scale == 0 {
            return formatter.write_str(&digits);
        }
        if digits.len() <= scale {
            write!(formatter, "0.{}{digits}", "0".repeat(scale.saturating_sub(digits.len())))
        } else {
            let (whole, fractional) = digits.split_at(digits.len().saturating_sub(scale));
            write!(formatter, "{whole}.{fractional}")
        }
    }
}

impl Serialize for Decimal {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for Decimal {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct DecimalVisitor;

        impl Visitor<'_> for DecimalVisitor {
            type Value = Decimal;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a nonnegative exact decimal string")
            }

            fn visit_str<E: Error>(self, value: &str) -> Result<Decimal, E> {
                value.parse().map_err(E::custom)
            }
        }

        deserializer.deserialize_str(DecimalVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::Decimal;

    #[test]
    fn decimal_sums_and_per_token_rates_never_pass_through_float() {
        let tenth: Decimal = "0.1".parse().unwrap();
        let fifth: Decimal = "0.2".parse().unwrap();
        assert_eq!(tenth.checked_add(fifth).unwrap().to_string(), "0.3");
        let rate: Decimal = "3.7500".parse().unwrap();
        assert_eq!(rate.per_million(12).unwrap().to_string(), "0.000045");
        assert_eq!(
            "0.0000001".parse::<Decimal>().unwrap().per_million(1).unwrap().to_string(),
            "0.0000000000001"
        );
        assert_eq!(rate.per_million(0).unwrap().to_string(), "0");
        assert_eq!(serde_json::to_string(&rate).unwrap(), r#""3.75""#);
    }

    #[test]
    fn invalid_or_unrepresentable_values_fail_without_rounding() {
        for value in [
            "",
            "-1",
            "+1",
            "NaN",
            "inf",
            "1e-3",
            ".1",
            "1.",
            " 1",
            "1.2.3",
            "0.000000000000000000000000000000000000001",
        ] {
            assert!(value.parse::<Decimal>().is_err(), "{value}");
        }
        let maximum: Decimal = u128::MAX.to_string().parse().unwrap();
        assert!(maximum.checked_add("1".parse().unwrap()).is_err());
        assert!(maximum.per_million(2).is_err());
    }

    #[test]
    fn modifiers_are_exact_and_canonical_values_compare_equal() {
        assert_eq!("01.2000".parse::<Decimal>().unwrap(), "1.2".parse::<Decimal>().unwrap());
        assert_eq!("0.000".parse::<Decimal>().unwrap(), Decimal::default());
        let rate: Decimal = "2.5".parse().unwrap();
        assert_eq!(rate.checked_mul("1.10".parse().unwrap()).unwrap().to_string(), "2.75");
    }

    #[test]
    fn rate_documents_accept_decimal_strings_but_reject_json_numbers() {
        let value: Decimal = serde_json::from_str(r#""0.0125""#).unwrap();
        assert_eq!(value.to_string(), "0.0125");
        for invalid in ["0.0125", "1", "null", "true", r#""1e-3""#] {
            assert!(serde_json::from_str::<Decimal>(invalid).is_err());
        }
    }
}
