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

/// The most paths [`strings_at`] reads at once.
pub const MAX_PATHS: usize = 64;

/// The strings at `paths` in one record's bytes, read without building a document.
///
/// It is exact with respect to parsing a `serde_json::Value` and asking each path's
/// [`text`]: it fails exactly when that parse would, reading every value through
/// `deserialize_any` as [`validate_record`] does, and a path's result is `None` when any
/// step is missing or not an object, or the value is not a string. A key that repeats
/// takes its last value, so a repeated object replaces every string read from the earlier
/// one. Paths name object keys only, and no path may be a prefix of another.
///
/// # Panics
///
/// In debug builds, when `paths` holds more than [`MAX_PATHS`] paths, an empty path, or a
/// path that is a prefix of another.
pub fn strings_at(bytes: &[u8], paths: &[&[&str]]) -> Result<Vec<Option<String>>, MalformedRecord> {
    debug_assert!(paths.len() <= MAX_PATHS && paths.iter().all(|path| !path.is_empty()));
    debug_assert!(paths.iter().enumerate().all(|(index, path)| {
        paths.iter().enumerate().all(|(other, longer)| index == other || !longer.starts_with(path))
    }));
    let invalid = |error: serde_json::Error| MalformedRecord { message: error.to_string() };
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    let shift = u32::try_from(MAX_PATHS.saturating_sub(paths.len())).unwrap_or(u32::MAX);
    let all = u64::MAX.checked_shr(shift).unwrap_or(0);
    let found = PathStrings { paths, selected: all, depth: 0 }
        .deserialize(&mut deserializer)
        .map_err(invalid)?;
    deserializer.end().map_err(invalid)?;
    Ok(found)
}

/// Reads the strings at the `selected` paths, below their first `depth` keys, from one
/// value; see [`strings_at`]. Its result holds an entry per path, `None` for the paths it
/// does not select.
#[derive(Clone, Copy)]
struct PathStrings<'p> {
    paths: &'p [&'p [&'p str]],
    /// A bit per path index.
    selected: u64,
    depth: usize,
}

impl PathStrings<'_> {
    /// Whether every selected path ends at this depth, so the value is the string itself.
    fn at_leaf(&self) -> bool {
        selected(self.selected, self.paths.len())
            .all(|index| self.paths.get(index).is_some_and(|path| path.len() == self.depth))
    }

    fn none(&self) -> Vec<Option<String>> {
        vec![None; self.paths.len()]
    }
}

/// The path indexes set in `mask`.
fn selected(mask: u64, paths: usize) -> impl Iterator<Item = usize> {
    (0..paths.min(MAX_PATHS)).filter(move |index| mask & (1 << index) != 0)
}

impl<'de> DeserializeSeed<'de> for PathStrings<'_> {
    type Value = Vec<Option<String>>;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_any(self)
    }
}

impl<'de> Visitor<'de> for PathStrings<'_> {
    type Value = Vec<Option<String>>;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("any JSON value")
    }

    fn visit_bool<E: Error>(self, _: bool) -> Result<Self::Value, E> {
        Ok(self.none())
    }

    fn visit_i64<E: Error>(self, _: i64) -> Result<Self::Value, E> {
        Ok(self.none())
    }

    fn visit_u64<E: Error>(self, _: u64) -> Result<Self::Value, E> {
        Ok(self.none())
    }

    fn visit_f64<E: Error>(self, _: f64) -> Result<Self::Value, E> {
        Ok(self.none())
    }

    fn visit_str<E: Error>(self, text: &str) -> Result<Self::Value, E> {
        let mut found = self.none();
        if self.at_leaf() {
            for index in selected(self.selected, self.paths.len()) {
                if let Some(slot) = found.get_mut(index) {
                    *slot = Some(text.to_owned());
                }
            }
        }
        Ok(found)
    }

    fn visit_unit<E: Error>(self) -> Result<Self::Value, E> {
        Ok(self.none())
    }

    fn visit_seq<A: SeqAccess<'de>>(self, seq: A) -> Result<Self::Value, A::Error> {
        Skip.visit_seq(seq)?;
        Ok(self.none())
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let mut found = self.none();
        if self.at_leaf() {
            Skip.visit_map(map)?;
            return Ok(found);
        }
        while let Some(under) = map.next_key_seed(KeyMatch(self))? {
            if under == 0 {
                map.next_value_seed(Skip)?;
                continue;
            }
            let child = PathStrings { selected: under, depth: self.depth + 1, ..self };
            let values = map.next_value_seed(child)?;
            // A repeated key replaces what the earlier value gave every path below it.
            for index in selected(under, self.paths.len()) {
                if let (Some(slot), Some(value)) = (found.get_mut(index), values.get(index)) {
                    slot.clone_from(value);
                }
            }
        }
        Ok(found)
    }
}

/// Reads an object key and gives the selected paths whose next step it is.
struct KeyMatch<'p>(PathStrings<'p>);

impl<'de> DeserializeSeed<'de> for KeyMatch<'_> {
    type Value = u64;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<u64, D::Error> {
        deserializer.deserialize_str(self)
    }
}

impl Visitor<'_> for KeyMatch<'_> {
    type Value = u64;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("an object key")
    }

    fn visit_str<E: Error>(self, key: &str) -> Result<u64, E> {
        let PathStrings { paths, selected: mask, depth } = self.0;
        Ok(selected(mask, paths.len())
            .filter(|index| paths.get(*index).and_then(|path| path.get(depth)) == Some(&key))
            .fold(0, |under, index| under | (1 << index)))
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

    use proptest::prelude::*;

    use super::{
        field, null_as_default, parse_record, parse_timestamp, strings_at, text, unsigned,
        validate_record,
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

    /// The paths a Codex catalog peek reads.
    const LINK_PATHS: [&[&str]; 5] = [
        &["type"],
        &["payload", "session_id"],
        &["payload", "parent_thread_id"],
        &["payload", "forked_from_id"],
        &["payload", "source", "subagent", "thread_spawn", "parent_thread_id"],
    ];

    /// Asserts that [`strings_at`] reads `bytes` as a parsed document and its paths do.
    fn assert_strings_match_document(bytes: &[u8]) {
        let read = strings_at(bytes, &LINK_PATHS);
        match parse_record(bytes) {
            Err(_) => assert!(read.is_err(), "{}", String::from_utf8_lossy(bytes)),
            Ok(document) => {
                let expected: Vec<Option<String>> = LINK_PATHS
                    .iter()
                    .map(|path| text(&document, path).map(str::to_owned))
                    .collect();
                assert_eq!(read.unwrap(), expected, "{}", String::from_utf8_lossy(bytes));
            }
        }
    }

    #[test]
    fn strings_are_read_as_a_document_reads_them() {
        for line in [
            r#"{"type":"session_meta","payload":{"session_id":"s","parent_thread_id":"p"}}"#,
            r#"{"type":"session_meta","payload":{"source":{"subagent":{"thread_spawn":{"parent_thread_id":"q"}}}}}"#,
            // A repeated key takes its last value; a repeated object replaces the earlier.
            r#"{"type":"turn_context","type":"session_meta","payload":{"session_id":"a"},"payload":{"forked_from_id":"f"}}"#,
            r#"{"payload":{"session_id":"a","session_id":7},"type":"session_meta"}"#,
            r#"{"payload":{"source":"cli","source":{"subagent":"review"}}}"#,
            r#"{"ty\u0070e":"session_meta","payload":{"session_id":"\u0073"}}"#,
            r#"["type","session_meta"]"#,
            r#""session_meta""#,
            r#"{"type":null,"payload":[{"session_id":"s"}]}"#,
            r#"{"type":"session_meta","payload":{"session_id":"\ud800"}}"#,
            r#"{"type":"session_meta","x":1e400}"#,
            r#"{"type":"session_meta"} trailing"#,
            r#"{"type":"session_meta""#,
        ] {
            assert_strings_match_document(line.as_bytes());
        }
        let deep = format!("{{\"payload\":{}{}}}", "[".repeat(200), "]".repeat(200));
        assert_strings_match_document(deep.as_bytes());
        assert_eq!(strings_at(b"{\"a\":\"b\"}", &[]).unwrap(), Vec::<Option<String>>::new());
    }

    fn json_object(
        entries: BoxedStrategy<(&'static str, String)>,
        size: std::ops::Range<usize>,
    ) -> BoxedStrategy<String> {
        prop::collection::vec(entries, size)
            .prop_map(|entries| {
                let fields: Vec<String> =
                    entries.into_iter().map(|(key, value)| format!("\"{key}\":{value}")).collect();
                format!("{{{}}}", fields.join(","))
            })
            .boxed()
    }

    /// Any value a field might hold, mostly valid, sometimes one a document rejects.
    fn json_leaf() -> BoxedStrategy<String> {
        prop_oneof![
            8 => prop_oneof![
                Just("null"),
                Just("true"),
                Just("\"session_meta\""),
                Just("\"s\\u0065ssion_meta\""),
                Just("\"thread\""),
                Just("2.5"),
                Just("[]"),
                Just("[\"session_meta\"]"),
                Just("{}"),
            ]
            .prop_map(str::to_owned),
            1 => prop_oneof![Just("\"\\ud800\""), Just("1e400"), Just("01")].prop_map(str::to_owned),
        ]
        .boxed()
    }

    fn json_line() -> BoxedStrategy<String> {
        let spawn = json_object(
            prop_oneof![
                3 => json_leaf().prop_map(|value| ("parent_thread_id", value)),
                1 => json_leaf().prop_map(|value| ("depth", value)),
            ]
            .boxed(),
            0..3,
        );
        let subagent = json_object(
            prop_oneof![
                3 => spawn.prop_map(|value| ("thread_spawn", value)),
                1 => json_leaf().prop_map(|value| ("thread_spawn", value)),
            ]
            .boxed(),
            0..3,
        );
        let source = json_object(
            prop_oneof![
                3 => subagent.prop_map(|value| ("subagent", value)),
                1 => json_leaf().prop_map(|value| ("subagent", value)),
            ]
            .boxed(),
            0..3,
        );
        let payload = json_object(
            prop_oneof![
                2 => json_leaf().prop_map(|value| ("session_id", value)),
                2 => json_leaf().prop_map(|value| ("parent_thread_id", value)),
                2 => json_leaf().prop_map(|value| ("forked_from_id", value)),
                1 => json_leaf().prop_map(|value| ("type", value)),
                3 => source.prop_map(|value| ("source", value)),
                1 => json_leaf().prop_map(|value| ("source", value)),
            ]
            .boxed(),
            0..6,
        );
        let entries = prop_oneof![
            3 => json_leaf().prop_map(|value| ("type", value)),
            1 => json_leaf().prop_map(|value| ("ty\\u0070e", value)),
            4 => payload.prop_map(|value| ("payload", value)),
            1 => json_leaf().prop_map(|value| ("payload", value)),
            1 => json_leaf().prop_map(|value| ("other", value)),
        ];
        prop_oneof![1 => json_leaf(), 12 => json_object(entries.boxed(), 0..5)].boxed()
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(2_000))]

        #[test]
        fn generated_lines_read_their_strings_as_documents_read_them(
            line in json_line(),
            cut in any::<prop::sample::Index>(),
            truncate in prop::bool::weighted(0.1),
        ) {
            let bytes = line.as_bytes();
            let bytes = if truncate { &bytes[..cut.index(bytes.len() + 1)] } else { bytes };
            assert_strings_match_document(bytes);
        }
    }
}
