//! RFC 8785 (JSON Canonicalization Scheme) serialization for the value types identity keys
//! use: `null`, integers, strings and arrays of them (design §3.6).
//!
//! Only this subset is implemented, because an analytical ID digests a canonical JSON array
//! of key components and no key holds an object, a boolean or a fractional number. The
//! subset still follows RFC 8785 exactly:
//!
//! - **Whitespace:** none is emitted between tokens (§3.2.1).
//! - **Strings:** U+0008, U+0009, U+000A, U+000C and U+000D are written as `\b`, `\t`,
//!   `\n`, `\f` and `\r`; every other code point below U+0020 as `\u00hh` with lowercase
//!   hex; `"` and `\` as `\"` and `\\`; everything else, including U+007F, U+2028 and
//!   non-ASCII text, as its UTF-8 bytes (§3.2.2.2). Rust strings cannot hold lone
//!   surrogates, so the RFC's error case for them cannot arise.
//! - **Integers:** RFC 8785 serializes numbers as ECMAScript doubles (§3.2.2.3), so only
//!   integers whose magnitude is at most 2^53 − 1 keep an exact, portable decimal form.
//!   Larger magnitudes are rejected with [`CanonicalJsonError::IntegerOutOfRange`] rather
//!   than rounded; keys must carry such values as strings, as RFC 8785 Appendix D advises.

use std::fmt::Write as _;

/// The largest integer magnitude RFC 8785 serializes exactly: 2^53 − 1.
pub const MAX_SAFE_INTEGER: i64 = 9_007_199_254_740_991;

/// A JSON value in the subset that identity keys use.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum CanonicalValue {
    /// JSON `null`, used for an unknown key component.
    Null,
    /// A JSON integer; serialization fails outside ±[`MAX_SAFE_INTEGER`].
    Integer(i64),
    /// A JSON string, serialized with RFC 8785 escaping.
    String(String),
    /// A JSON array, serialized in element order.
    Array(Vec<CanonicalValue>),
}

/// Why a value has no RFC 8785 canonical form.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum CanonicalJsonError {
    /// An integer is outside the range an IEEE 754 double represents exactly.
    #[error(
        "integer {0} is outside ±(2^53 − 1) and has no exact RFC 8785 form; carry it as a string"
    )]
    IntegerOutOfRange(i64),
}

/// Serializes `value` as RFC 8785 canonical JSON text, whose UTF-8 bytes are the
/// canonical encoding.
pub fn to_canonical_json(value: &CanonicalValue) -> Result<String, CanonicalJsonError> {
    let mut out = String::new();
    write_value(&mut out, value)?;
    Ok(out)
}

fn write_value(out: &mut String, value: &CanonicalValue) -> Result<(), CanonicalJsonError> {
    match value {
        CanonicalValue::Null => out.push_str("null"),
        CanonicalValue::Integer(number) => {
            if number.unsigned_abs() > MAX_SAFE_INTEGER.unsigned_abs() {
                return Err(CanonicalJsonError::IntegerOutOfRange(*number));
            }
            // Writing to a String cannot fail.
            write!(out, "{number}").ok();
        }
        CanonicalValue::String(text) => write_string(out, text),
        CanonicalValue::Array(items) => {
            out.push('[');
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                write_value(out, item)?;
            }
            out.push(']');
        }
    }
    Ok(())
}

fn write_string(out: &mut String, text: &str) {
    out.push('"');
    for ch in text.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{08}' => out.push_str("\\b"),
            '\t' => out.push_str("\\t"),
            '\n' => out.push_str("\\n"),
            '\u{0c}' => out.push_str("\\f"),
            '\r' => out.push_str("\\r"),
            control if u32::from(control) < 0x20 => {
                // Writing to a String cannot fail.
                write!(out, "\\u{:04x}", u32::from(control)).ok();
            }
            other => out.push(other),
        }
    }
    out.push('"');
}

#[cfg(test)]
mod tests {
    use super::{CanonicalJsonError, CanonicalValue, MAX_SAFE_INTEGER, to_canonical_json};

    fn string(text: &str) -> CanonicalValue {
        CanonicalValue::String(text.to_owned())
    }

    #[test]
    fn serializes_the_rfc_8785_string_sample_byte_for_byte() {
        // RFC 8785 §3.2.2 parses the JSON string sample
        // "\u20ac$\u000F\u000aA'\u0042\u0022\u005c\\\"\/" and
        // §3.2.4 gives the canonical bytes of the resulting string value.
        let parsed = "\u{20ac}$\u{000f}\nA'B\"\\\\\"/";
        let expected: &[u8] = &[
            0x22, 0xe2, 0x82, 0xac, 0x24, 0x5c, 0x75, 0x30, 0x30, 0x30, 0x66, 0x5c, 0x6e, 0x41,
            0x27, 0x42, 0x5c, 0x22, 0x5c, 0x5c, 0x5c, 0x5c, 0x5c, 0x22, 0x2f, 0x22,
        ];
        let canonical = to_canonical_json(&string(parsed)).unwrap();
        assert_eq!(canonical.as_bytes(), expected);
        assert_eq!(canonical, r#""€$\u000f\nA'B\"\\\\\"/""#);
    }

    #[test]
    fn uses_short_escapes_only_for_the_five_predefined_controls() {
        let all_controls: String = (0u8..0x20).map(char::from).collect();
        let canonical = to_canonical_json(&string(&all_controls)).unwrap();
        assert_eq!(
            canonical,
            concat!(
                r#""\u0000\u0001\u0002\u0003\u0004\u0005\u0006\u0007"#,
                r#"\b\t\n\u000b\f\r\u000e\u000f"#,
                r#"\u0010\u0011\u0012\u0013\u0014\u0015\u0016\u0017"#,
                r#"\u0018\u0019\u001a\u001b\u001c\u001d\u001e\u001f""#,
            )
        );
    }

    #[test]
    fn writes_delete_line_separators_and_astral_text_as_is() {
        let text = "\u{7f}\u{2028}\u{2029}\u{1f600}/";
        assert_eq!(to_canonical_json(&string(text)).unwrap(), format!("\"{text}\""));
    }

    #[test]
    fn serializes_integers_like_ecmascript_within_the_safe_range() {
        // RFC 8785 Appendix B: zero, and the integer range its note (1) recommends.
        let cases = [
            (0, "0"),
            (7, "7"),
            (-42, "-42"),
            (MAX_SAFE_INTEGER, "9007199254740991"),
            (-MAX_SAFE_INTEGER, "-9007199254740991"),
        ];
        for (number, expected) in cases {
            assert_eq!(to_canonical_json(&CanonicalValue::Integer(number)).unwrap(), expected);
        }
    }

    #[test]
    fn rejects_integers_a_double_cannot_hold_exactly() {
        for number in [MAX_SAFE_INTEGER + 1, -MAX_SAFE_INTEGER - 1, i64::MAX, i64::MIN] {
            assert_eq!(
                to_canonical_json(&CanonicalValue::Integer(number)),
                Err(CanonicalJsonError::IntegerOutOfRange(number))
            );
        }
    }

    #[test]
    fn serializes_arrays_without_whitespace() {
        let value = CanonicalValue::Array(vec![
            string("req"),
            CanonicalValue::Integer(1),
            CanonicalValue::Null,
            CanonicalValue::Array(vec![]),
            CanonicalValue::Array(vec![CanonicalValue::Null]),
        ]);
        assert_eq!(to_canonical_json(&value).unwrap(), r#"["req",1,null,[],[null]]"#);
    }

    #[test]
    fn an_out_of_range_integer_inside_an_array_fails_the_whole_value() {
        let value = CanonicalValue::Array(vec![string("a"), CanonicalValue::Integer(i64::MAX)]);
        assert!(to_canonical_json(&value).is_err());
    }
}
