//! Lenient record decoding (design §2.2).
//!
//! Decoding is lenient where records allow it, because a strict reader loses usage that a
//! source really recorded:
//!
//! - A record is never rejected because a nested field is null. ccusage drops a Claude
//!   record whose line contains `"id":null` anywhere, including inside tool input, and
//!   loses its usage; the helpers here treat null like a missing field at the path asked
//!   for and ignore it everywhere else.
//! - Timestamps accept any RFC 3339 fractional precision. [`parse_timestamp`] keeps the
//!   first nine fractional digits, which is the resolution a [`Timestamp`] holds, and
//!   ignores the rest instead of failing.
//! - A record that is not valid JSON is [`MalformedRecord`], counted per source as
//!   interior corruption rather than skipped silently.
//!
//! Values are read by path with [`field`], [`text`] and [`unsigned`], which return `None`
//! for a missing field, a null, or a value of another type, so a dialect's shape change
//! turns into an unparsable record with counters rather than a panic or a zero.

use jiff::Timestamp;
use std::fmt;

use serde::de::{DeserializeSeed, Error, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer};
use serde_json::Value;

/// A record that is not valid JSON.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[error("record is not valid JSON: {message}")]
pub struct MalformedRecord {
    /// The parser's message, without the record's bytes.
    pub message: String,
}

/// Parses one record's bytes as JSON for differential test oracles.
#[cfg(test)]
pub(crate) fn parse_record(bytes: &[u8]) -> Result<Value, MalformedRecord> {
    serde_json::from_slice(bytes).map_err(|error| MalformedRecord { message: error.to_string() })
}

/// Checks that one record's bytes are valid JSON without building a document.
///
/// It fails exactly when parsing a `serde_json::Value` would: every value is read through
/// `deserialize_any`, the path a document takes, so invalid UTF-8 or escapes in any string,
/// a number out of range, nesting past the recursion limit and trailing characters are all
/// rejected. `serde::de::IgnoredAny` would not do, because `serde_json` skips it without
/// those checks.
pub fn validate_record(bytes: &[u8]) -> Result<(), MalformedRecord> {
    let invalid = |error: serde_json::Error| MalformedRecord { message: error.to_string() };
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    Skip.deserialize(&mut deserializer).map_err(invalid)?;
    deserializer.end().map_err(invalid)
}

/// Consumes any JSON value with the checks a document gets, keeping nothing.
#[derive(Clone, Copy)]
pub(crate) struct Skip;

impl<'de> DeserializeSeed<'de> for Skip {
    type Value = ();

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<(), D::Error> {
        deserializer.deserialize_any(self)
    }
}

impl<'de> Visitor<'de> for Skip {
    type Value = ();

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("any JSON value")
    }

    fn visit_bool<E: Error>(self, _: bool) -> Result<(), E> {
        Ok(())
    }

    fn visit_i64<E: Error>(self, _: i64) -> Result<(), E> {
        Ok(())
    }

    fn visit_u64<E: Error>(self, _: u64) -> Result<(), E> {
        Ok(())
    }

    fn visit_f64<E: Error>(self, _: f64) -> Result<(), E> {
        Ok(())
    }

    fn visit_str<E: Error>(self, _: &str) -> Result<(), E> {
        Ok(())
    }

    fn visit_unit<E: Error>(self) -> Result<(), E> {
        Ok(())
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<(), A::Error> {
        while seq.next_element_seed(self)?.is_some() {}
        Ok(())
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<(), A::Error> {
        while map.next_entry_seed(self, self)?.is_some() {}
        Ok(())
    }
}

/// The value at `path`, or `None` when any step is missing, null or not an object.
pub fn field<'a>(value: &'a Value, path: &[&str]) -> Option<&'a Value> {
    let mut current = value;
    for step in path {
        current = current.as_object()?.get(*step)?;
    }
    if current.is_null() { None } else { Some(current) }
}

/// The string at `path`, or `None` when it is missing, null or not a string.
pub fn text<'a>(value: &'a Value, path: &[&str]) -> Option<&'a str> {
    field(value, path)?.as_str()
}

/// The non-negative integer at `path`.
///
/// A JSON number written as a whole float, which several writers emit for counters, is
/// accepted; a fractional or negative number is not, because a token count that is
/// neither is a dialect change worth reporting rather than rounding.
pub fn unsigned(value: &Value, path: &[&str]) -> Option<u64> {
    let number = field(value, path)?.as_number()?;
    if let Some(unsigned) = number.as_u64() {
        return Some(unsigned);
    }
    let float = number.as_f64()?;
    if float.is_finite() && float >= 0.0 && float.fract() == 0.0 && float <= 9_007_199_254_740_992.0
    {
        // Exact: the value is a whole number inside the range a double represents exactly.
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "checked above"
        )]
        return Some(float as u64);
    }
    None
}

/// A timestamp that is not RFC 3339, apart from extra fractional digits.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[error("not an RFC 3339 timestamp: {text:?} ({message})")]
pub struct TimestampError {
    /// The rejected text.
    pub text: String,
    /// The parser's message.
    pub message: String,
}

/// Parses an RFC 3339 timestamp at any fractional precision.
///
/// RFC 3339 allows any number of fractional digits; a [`Timestamp`] holds nanoseconds, so
/// digits past the ninth are dropped (the value is truncated, never rounded up past the
/// instant recorded).
pub fn parse_timestamp(value: &str) -> Result<Timestamp, TimestampError> {
    let trimmed = truncate_fraction(value);
    trimmed
        .parse::<Timestamp>()
        .map_err(|error| TimestampError { text: value.to_owned(), message: error.to_string() })
}

/// Drops fractional-second digits past the ninth, leaving everything else untouched.
fn truncate_fraction(value: &str) -> std::borrow::Cow<'_, str> {
    let Some(start) = value.find(['.', ',']) else {
        return std::borrow::Cow::Borrowed(value);
    };
    let digits_at = start.saturating_add(1);
    let Some(rest) = value.get(digits_at..) else {
        return std::borrow::Cow::Borrowed(value);
    };
    let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
    if digits <= 9 {
        return std::borrow::Cow::Borrowed(value);
    }
    let keep = digits_at.saturating_add(9);
    let tail_at = digits_at.saturating_add(digits);
    match (value.get(..keep), value.get(tail_at..)) {
        (Some(head), Some(tail)) => std::borrow::Cow::Owned(format!("{head}{tail}")),
        _ => std::borrow::Cow::Borrowed(value),
    }
}

/// A `deserialize_with` helper that reads an explicit null as the type's default, so a
/// typed record keeps its other fields instead of failing to deserialize.
pub fn null_as_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Default + Deserialize<'de>,
{
    Ok(Option::deserialize(deserializer)?.unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::{
        field, null_as_default, parse_record, parse_timestamp, text, unsigned, validate_record,
    };

    #[derive(Debug, Deserialize, Eq, PartialEq)]
    struct Record {
        #[serde(default, deserialize_with = "null_as_default")]
        id: String,
        #[serde(default, deserialize_with = "null_as_default")]
        output_tokens: u64,
    }

    #[test]
    fn a_nested_null_never_costs_a_record_its_usage() {
        // ccusage rejects this line because it contains `"id":null` inside tool input.
        let line = br#"{"id":"msg_1","tool":{"input":{"id":null,"cwd":null}},
            "message":{"usage":{"output_tokens":7,"cache_creation":{"ephemeral_1h_input_tokens":null}}}}"#;
        let record = parse_record(line).unwrap();
        assert_eq!(unsigned(&record, &["message", "usage", "output_tokens"]), Some(7));
        assert_eq!(text(&record, &["id"]), Some("msg_1"));
        // A null nested field reads as absent, never as zero.
        assert_eq!(
            unsigned(&record, &["message", "usage", "cache_creation", "ephemeral_1h_input_tokens"]),
            None
        );
        assert!(field(&record, &["tool", "input", "cwd"]).is_none());
        assert!(field(&record, &["message", "missing"]).is_none());
    }

    #[test]
    fn typed_records_take_an_explicit_null_as_the_default() {
        let record: Record = serde_json::from_slice(br#"{"id":null,"output_tokens":3}"#).unwrap();
        assert_eq!(record, Record { id: String::new(), output_tokens: 3 });
    }

    #[test]
    fn counters_written_as_whole_floats_are_read() {
        let record = parse_record(br#"{"a":12.0,"b":12.5,"c":-1,"d":"12"}"#).unwrap();
        assert_eq!(unsigned(&record, &["a"]), Some(12));
        assert_eq!(unsigned(&record, &["b"]), None);
        assert_eq!(unsigned(&record, &["c"]), None);
        assert_eq!(unsigned(&record, &["d"]), None);
    }

    #[test]
    fn malformed_records_are_errors_not_panics() {
        let error = parse_record(b"{\"a\":").unwrap_err();
        assert!(error.message.contains("EOF") || error.message.contains("eof"), "{error}");
        assert!(parse_record(&[0xff, 0xfe]).is_err());
    }

    #[test]
    fn timestamps_accept_any_rfc_3339_fractional_precision() {
        let expected = "2026-09-15T12:34:56Z".parse::<jiff::Timestamp>().unwrap();
        for value in [
            "2026-09-15T12:34:56Z",
            "2026-09-15T12:34:56.0Z",
            "2026-09-15T12:34:56.000Z",
            "2026-09-15T12:34:56.000000Z",
            "2026-09-15T12:34:56.000000000Z",
            "2026-09-15T12:34:56.0000000000000Z",
            "2026-09-15t12:34:56.00z",
        ] {
            assert_eq!(parse_timestamp(value).unwrap(), expected, "{value}");
        }
        // Digits past the ninth are dropped, and offsets survive the truncation.
        let precise = parse_timestamp("2026-09-15T12:34:56.123456789987654Z").unwrap();
        assert_eq!(precise, "2026-09-15T12:34:56.123456789Z".parse::<jiff::Timestamp>().unwrap());
        let offset = parse_timestamp("2026-09-15T08:34:56.1234567891234-04:00").unwrap();
        assert_eq!(offset, parse_timestamp("2026-09-15T12:34:56.123456789Z").unwrap());
        assert!(parse_timestamp("2026-09-15 12:34:56").is_err());
        assert!(parse_timestamp("not a time").is_err());
    }

    #[test]
    fn validation_rejects_exactly_what_parsing_rejects() {
        let deep = format!("{}{}", "[".repeat(200), "]".repeat(200));
        let lines: Vec<Vec<u8>> = [
            r#"{"type":"response_item","payload":{"content":"text"}}"#,
            r#"{"content":"\ud800"}"#,
            r#"{"content":"\ud83d\ude00"}"#,
            r#"{"size":1e400}"#,
            r#"{"size":-12.5e-3}"#,
            r#"{"content":"\q"}"#,
            r#"{"a":1} trailing"#,
            r#"{"a":1}   "#,
            r#"{"a":"#,
            r#"{"a":1,"a":[true,null,{}]}"#,
            deep.as_str(),
        ]
        .iter()
        .map(|line| line.as_bytes().to_vec())
        .chain([b"{\"content\":\"\xff\"}".to_vec()])
        .collect();
        for line in &lines {
            assert_eq!(
                validate_record(line).is_ok(),
                parse_record(line).is_ok(),
                "{}",
                String::from_utf8_lossy(line)
            );
        }
        assert!(validate_record(br#"{"content":"\ud800"}"#).is_err());
        assert!(validate_record(br#"{"size":1e400}"#).is_err());
        assert!(validate_record(deep.as_bytes()).is_err());
    }
}
