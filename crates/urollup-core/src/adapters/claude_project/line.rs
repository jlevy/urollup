//! The first pass over a Claude Code transcript line.
//!
//! Every line contributes its record type, its sidechain flag and, until a source has
//! them, its CLI version and working directory; only a line that can bear a request
//! needs the rest. [`LineHead::read`] reads those fields, and whether the line bears
//! usage, without building a JSON document or allocating for the fields it ignores.
//!
//! The pass is exact with respect to [`parse_record`](crate::sources::decode::parse_record)
//! followed by the adapter's path helpers:
//!
//! - Every value, kept or ignored, is read through `deserialize_any`, the path a
//!   `serde_json::Value` takes, so a line fails here exactly when it fails to parse as a
//!   document: invalid UTF-8 or escapes in any string, a number out of range, nesting past
//!   the recursion limit, or trailing characters. `serde::de::IgnoredAny` would not do:
//!   `serde_json` skips it without those checks.
//! - A field that repeats takes its last value, as a document's object does.
//! - A field that is null or of another type reads as missing, as
//!   [`text`](crate::sources::decode::text) and [`unsigned`] treat it.

use std::borrow::Cow;
use std::fmt;

use serde::Deserialize;
use serde::de::{DeserializeSeed, Deserializer, Error, MapAccess, SeqAccess, Visitor};
use serde_json::{Map, Value};

use crate::sources::decode::{Skip, unsigned};

/// The path, below a record's `message`, of the output count every usage record has.
const MESSAGE_OUTPUT_TOKENS: &[&str] = &["usage", "output_tokens"];

/// The path, below a progress record's `data`, of its nested record's output count.
const NESTED_OUTPUT_TOKENS: &[&str] = &["message", "message", "usage", "output_tokens"];

/// A record's `type`, as far as the adapter distinguishes it.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) enum LineType {
    /// `assistant`: a response, which is a request record when it reports usage.
    Assistant,
    /// `progress`: which nests a subagent's response under `data.message`.
    Progress,
    /// Any other type, a missing type, or a type that is not a string.
    #[default]
    Other,
}

/// The fields of one line that the adapter reads before deciding whether to decode it.
#[derive(Debug, Default, Eq, PartialEq)]
pub(super) struct LineHead<'a> {
    /// The record type.
    pub(super) line_type: LineType,
    /// Whether `isSidechain` is `true`.
    pub(super) sidechain: bool,
    /// The `version` string, when it was asked for.
    pub(super) version: Option<Cow<'a, str>>,
    /// The `cwd` string, when it was asked for.
    pub(super) cwd: Option<Cow<'a, str>>,
    /// Whether `message.usage.output_tokens` is an unsigned integer.
    message_usage: bool,
    /// Whether `data.message.message.usage.output_tokens` is an unsigned integer.
    nested_usage: bool,
}

impl<'a> LineHead<'a> {
    /// Reads the head of one line, keeping `version` and `cwd` only when asked for.
    ///
    /// Fails exactly when the line is not a valid JSON document.
    pub(super) fn read(
        bytes: &'a [u8],
        want_version: bool,
        want_cwd: bool,
    ) -> Result<Self, serde_json::Error> {
        let mut deserializer = serde_json::Deserializer::from_slice(bytes);
        let head = Head { want_version, want_cwd }.deserialize(&mut deserializer)?;
        deserializer.end()?;
        Ok(head)
    }

    /// Whether the line can bear a request: an assistant record with an output count, or a
    /// progress record whose nested record has one.
    pub(super) fn bears_usage(&self) -> bool {
        match self.line_type {
            LineType::Assistant => self.message_usage,
            LineType::Progress => self.nested_usage,
            LineType::Other => false,
        }
    }
}

/// Implements the visits of a lenient reader that yields `$default` for every value it
/// does not interpret, consuming containers with [`Skip`].
macro_rules! lenient_visits {
    ($default:expr, [$($skip:ident),*]) => {
        $(lenient_visits!(@one $skip, $default);)*

        fn visit_seq<A: SeqAccess<'de>>(self, seq: A) -> Result<Self::Value, A::Error> {
            Skip.visit_seq(seq)?;
            Ok($default)
        }
    };
    (@one bool, $default:expr) => {
        fn visit_bool<E: Error>(self, _: bool) -> Result<Self::Value, E> {
            Ok($default)
        }
    };
    (@one numbers, $default:expr) => {
        fn visit_i64<E: Error>(self, _: i64) -> Result<Self::Value, E> {
            Ok($default)
        }

        fn visit_u64<E: Error>(self, _: u64) -> Result<Self::Value, E> {
            Ok($default)
        }

        fn visit_f64<E: Error>(self, _: f64) -> Result<Self::Value, E> {
            Ok($default)
        }
    };
    (@one str, $default:expr) => {
        fn visit_str<E: Error>(self, _: &str) -> Result<Self::Value, E> {
            Ok($default)
        }
    };
    (@one unit, $default:expr) => {
        fn visit_unit<E: Error>(self) -> Result<Self::Value, E> {
            Ok($default)
        }
    };
    (@one map, $default:expr) => {
        fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<Self::Value, A::Error> {
            Skip.visit_map(map)?;
            Ok($default)
        }
    };
}

/// The top-level fields the head reads.
#[derive(Clone, Copy)]
enum Field {
    Type,
    IsSidechain,
    Version,
    Cwd,
    Message,
    Data,
    Other,
}

struct FieldName;

impl<'de> DeserializeSeed<'de> for FieldName {
    type Value = Field;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Field, D::Error> {
        deserializer.deserialize_str(self)
    }
}

impl Visitor<'_> for FieldName {
    type Value = Field;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("an object key")
    }

    fn visit_str<E: Error>(self, name: &str) -> Result<Field, E> {
        Ok(match name {
            "type" => Field::Type,
            "isSidechain" => Field::IsSidechain,
            "version" => Field::Version,
            "cwd" => Field::Cwd,
            "message" => Field::Message,
            "data" => Field::Data,
            _ => Field::Other,
        })
    }
}

/// Reads the head from a whole line; a line that is not an object has no fields.
struct Head {
    want_version: bool,
    want_cwd: bool,
}

impl<'de> DeserializeSeed<'de> for Head {
    type Value = LineHead<'de>;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_any(self)
    }
}

impl<'de> Visitor<'de> for Head {
    type Value = LineHead<'de>;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("any JSON value")
    }

    lenient_visits!(LineHead::default(), [bool, numbers, str, unit]);

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let mut head = LineHead::default();
        while let Some(field) = map.next_key_seed(FieldName)? {
            match field {
                Field::Type => head.line_type = map.next_value_seed(TypeName)?,
                Field::IsSidechain => head.sidechain = map.next_value_seed(IsTrue)?,
                Field::Version if self.want_version => head.version = map.next_value_seed(Text)?,
                Field::Cwd if self.want_cwd => head.cwd = map.next_value_seed(Text)?,
                Field::Message => {
                    head.message_usage = map.next_value_seed(UnsignedAt(MESSAGE_OUTPUT_TOKENS))?;
                }
                Field::Data => {
                    head.nested_usage = map.next_value_seed(UnsignedAt(NESTED_OUTPUT_TOKENS))?;
                }
                Field::Version | Field::Cwd | Field::Other => map.next_value_seed(Skip)?,
            }
        }
        Ok(head)
    }
}

/// Reads a `type` value.
struct TypeName;

impl<'de> DeserializeSeed<'de> for TypeName {
    type Value = LineType;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<LineType, D::Error> {
        deserializer.deserialize_any(self)
    }
}

impl<'de> Visitor<'de> for TypeName {
    type Value = LineType;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("any JSON value")
    }

    lenient_visits!(LineType::Other, [bool, numbers, unit, map]);

    fn visit_str<E: Error>(self, name: &str) -> Result<LineType, E> {
        Ok(match name {
            "assistant" => LineType::Assistant,
            "progress" => LineType::Progress,
            _ => LineType::Other,
        })
    }
}

/// Reads whether a value is JSON `true`.
struct IsTrue;

impl<'de> DeserializeSeed<'de> for IsTrue {
    type Value = bool;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<bool, D::Error> {
        deserializer.deserialize_any(self)
    }
}

impl<'de> Visitor<'de> for IsTrue {
    type Value = bool;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("any JSON value")
    }

    lenient_visits!(false, [numbers, str, unit, map]);

    fn visit_bool<E: Error>(self, value: bool) -> Result<bool, E> {
        Ok(value)
    }
}

/// Reads a string value, borrowing it from the line unless it has escapes.
struct Text;

impl<'de> DeserializeSeed<'de> for Text {
    type Value = Option<Cow<'de, str>>;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_any(self)
    }
}

impl<'de> Visitor<'de> for Text {
    type Value = Option<Cow<'de, str>>;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("any JSON value")
    }

    lenient_visits!(None, [bool, numbers, unit, map]);

    fn visit_borrowed_str<E: Error>(self, value: &'de str) -> Result<Self::Value, E> {
        Ok(Some(Cow::Borrowed(value)))
    }

    fn visit_str<E: Error>(self, value: &str) -> Result<Self::Value, E> {
        Ok(Some(Cow::Owned(value.to_owned())))
    }
}

/// Reads whether the value at a path below this one is an unsigned integer, as
/// [`unsigned`] reads it from a document.
#[derive(Clone, Copy)]
struct UnsignedAt(&'static [&'static str]);

impl UnsignedAt {
    fn number(self, number: &Value) -> bool {
        self.0.is_empty() && unsigned(number, &[]).is_some()
    }
}

impl<'de> DeserializeSeed<'de> for UnsignedAt {
    type Value = bool;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<bool, D::Error> {
        deserializer.deserialize_any(self)
    }
}

impl<'de> Visitor<'de> for UnsignedAt {
    type Value = bool;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("any JSON value")
    }

    lenient_visits!(false, [bool, str, unit]);

    // Each number becomes the document value a parse would build, so the check is the
    // one `unsigned` applies.
    fn visit_i64<E: Error>(self, value: i64) -> Result<bool, E> {
        Ok(self.number(&Value::from(value)))
    }

    fn visit_u64<E: Error>(self, value: u64) -> Result<bool, E> {
        Ok(self.number(&Value::from(value)))
    }

    fn visit_f64<E: Error>(self, value: f64) -> Result<bool, E> {
        Ok(self.number(&Value::from(value)))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<bool, A::Error> {
        let Some((step, rest)) = self.0.split_first() else {
            Skip.visit_map(map)?;
            return Ok(false);
        };
        let mut found = false;
        while let Some(matches) = map.next_key_seed(KeyIs(step))? {
            if matches {
                found = map.next_value_seed(UnsignedAt(rest))?;
            } else {
                map.next_value_seed(Skip)?;
            }
        }
        Ok(found)
    }
}

/// Reads whether an object key is a given name.
struct KeyIs<'a>(&'a str);

impl<'de> DeserializeSeed<'de> for KeyIs<'_> {
    type Value = bool;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<bool, D::Error> {
        deserializer.deserialize_str(self)
    }
}

impl Visitor<'_> for KeyIs<'_> {
    type Value = bool;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("an object key")
    }

    fn visit_str<E: Error>(self, name: &str) -> Result<bool, E> {
        Ok(name == self.0)
    }
}

/// Implements `DeserializeSeed` for a visitor seed that reads any JSON value.
macro_rules! any_value_seed {
    ($seed:ty) => {
        impl<'de> DeserializeSeed<'de> for $seed {
            type Value = <Self as Visitor<'de>>::Value;

            fn deserialize<D: Deserializer<'de>>(
                self,
                deserializer: D,
            ) -> Result<Self::Value, D::Error> {
                deserializer.deserialize_any(self)
            }
        }
    };
}

/// The accounting fields of one usage-bearing line, without a parent JSON document.
#[derive(Debug, Default, PartialEq)]
pub(super) struct UsageBody<'a> {
    /// Top-level record fields.
    pub top: RecordFields<'a>,
    /// `data.message`, the nested transcript record a progress line carries.
    pub nested: Option<RecordFields<'a>>,
}

/// The fields [`super::SourceDecoder::record`] reads from an assistant line or a nested
/// progress record.
#[derive(Debug, Default, PartialEq)]
pub(super) struct RecordFields<'a> {
    pub session: Option<Cow<'a, str>>,
    pub uuid: Option<Cow<'a, str>>,
    pub request_id: Option<Cow<'a, str>>,
    pub timestamp: Option<Cow<'a, str>>,
    pub effort: Option<Cow<'a, str>>,
    pub cwd: Option<Cow<'a, str>>,
    pub is_api_error: bool,
    /// `quotaLimits` when it is an object; the parent document is never kept.
    pub quota: Option<Map<String, Value>>,
    pub api_block_index: Option<u64>,
    pub message: MessageFields<'a>,
}

/// The fields below a record's `message`.
#[derive(Debug, Default, PartialEq)]
pub(super) struct MessageFields<'a> {
    pub id: Option<Cow<'a, str>>,
    pub model: Option<Cow<'a, str>>,
    pub usage: UsageCounts,
    pub tool_use_ids: Vec<Cow<'a, str>>,
    /// `message.content[0].text`, used only for the usage-limit error string.
    pub first_text: Option<Cow<'a, str>>,
    pub advisors: Vec<AdvisorFields<'a>>,
}

/// Token counts a usage object may report.
#[derive(Debug, Default, PartialEq)]
pub(super) struct UsageCounts {
    pub input: Option<u64>,
    pub cache_read: Option<u64>,
    pub cache_write: Option<u64>,
    pub cache_write_5m: Option<u64>,
    pub cache_write_1h: Option<u64>,
    pub output: Option<u64>,
    pub reasoning: Option<u64>,
}

/// One `message.usage.iterations` entry whose `type` is `advisor_message`.
#[derive(Debug, Default, PartialEq)]
pub(super) struct AdvisorFields<'a> {
    pub model: Option<Cow<'a, str>>,
    pub input: Option<u64>,
    pub cache_read: Option<u64>,
    pub cache_write: Option<u64>,
    pub output: Option<u64>,
    pub reasoning: Option<u64>,
}

impl<'a> UsageBody<'a> {
    /// Reads one line; fails exactly when the line is not a valid JSON document.
    pub(super) fn read(bytes: &'a [u8]) -> Result<Self, serde_json::Error> {
        let mut deserializer = serde_json::Deserializer::from_slice(bytes);
        let body = BodySeed.deserialize(&mut deserializer)?;
        deserializer.end()?;
        Ok(body)
    }
}

/// Reads an unsigned integer as [`unsigned`] reads it from a document.
struct Unsigned;

any_value_seed!(Unsigned);

impl<'de> Visitor<'de> for Unsigned {
    type Value = Option<u64>;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("any JSON value")
    }

    lenient_visits!(None, [bool, str, unit, map]);

    fn visit_i64<E: Error>(self, value: i64) -> Result<Self::Value, E> {
        Ok(unsigned(&Value::from(value), &[]))
    }

    fn visit_u64<E: Error>(self, value: u64) -> Result<Self::Value, E> {
        Ok(unsigned(&Value::from(value), &[]))
    }

    fn visit_f64<E: Error>(self, value: f64) -> Result<Self::Value, E> {
        Ok(unsigned(&Value::from(value), &[]))
    }
}

struct BodySeed;

any_value_seed!(BodySeed);

impl<'de> Visitor<'de> for BodySeed {
    type Value = UsageBody<'de>;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("any JSON value")
    }

    lenient_visits!(UsageBody::default(), [bool, numbers, str, unit]);

    fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<Self::Value, A::Error> {
        let mut top = RecordFields::default();
        let nested = read_record_map(map, &mut top)?;
        Ok(UsageBody { top, nested })
    }
}

/// Top-level keys of an assistant or nested progress record.
#[derive(Clone, Copy)]
enum RecordField {
    SessionId,
    Uuid,
    RequestId,
    Timestamp,
    Effort,
    Cwd,
    IsApiErrorMessage,
    QuotaLimits,
    ApiBlockIndex,
    Message,
    Data,
    Other,
}

impl RecordField {
    fn named(name: &str) -> Self {
        match name {
            "sessionId" => Self::SessionId,
            "uuid" => Self::Uuid,
            "requestId" => Self::RequestId,
            "timestamp" => Self::Timestamp,
            "effort" => Self::Effort,
            "cwd" => Self::Cwd,
            "isApiErrorMessage" => Self::IsApiErrorMessage,
            "quotaLimits" => Self::QuotaLimits,
            "apiBlockIndex" => Self::ApiBlockIndex,
            "message" => Self::Message,
            "data" => Self::Data,
            _ => Self::Other,
        }
    }
}

struct RecordKey;

impl<'de> DeserializeSeed<'de> for RecordKey {
    type Value = RecordField;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<RecordField, D::Error> {
        deserializer.deserialize_str(self)
    }
}

impl Visitor<'_> for RecordKey {
    type Value = RecordField;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("an object key")
    }

    fn visit_str<E: Error>(self, name: &str) -> Result<RecordField, E> {
        Ok(RecordField::named(name))
    }
}

fn read_record_map<'de, A: MapAccess<'de>>(
    mut map: A,
    fields: &mut RecordFields<'de>,
) -> Result<Option<RecordFields<'de>>, A::Error> {
    let mut nested = None;
    while let Some(field) = map.next_key_seed(RecordKey)? {
        match field {
            RecordField::SessionId => fields.session = map.next_value_seed(Text)?,
            RecordField::Uuid => fields.uuid = map.next_value_seed(Text)?,
            RecordField::RequestId => fields.request_id = map.next_value_seed(Text)?,
            RecordField::Timestamp => fields.timestamp = map.next_value_seed(Text)?,
            RecordField::Effort => fields.effort = map.next_value_seed(Text)?,
            RecordField::Cwd => fields.cwd = map.next_value_seed(Text)?,
            RecordField::IsApiErrorMessage => fields.is_api_error = map.next_value_seed(IsTrue)?,
            RecordField::QuotaLimits => fields.quota = map.next_value_seed(ObjectSeed)?,
            RecordField::ApiBlockIndex => fields.api_block_index = map.next_value_seed(Unsigned)?,
            RecordField::Message => fields.message = map.next_value_seed(MessageSeed)?,
            RecordField::Data => nested = map.next_value_seed(DataSeed)?,
            RecordField::Other => map.next_value_seed(Skip)?,
        }
    }
    Ok(nested)
}

struct NestedRecordSeed;

any_value_seed!(NestedRecordSeed);

impl<'de> Visitor<'de> for NestedRecordSeed {
    type Value = Option<RecordFields<'de>>;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("any JSON value")
    }

    lenient_visits!(None, [bool, numbers, str, unit]);

    fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<Self::Value, A::Error> {
        let mut fields = RecordFields::default();
        let _nested = read_record_map(map, &mut fields)?;
        Ok(Some(fields))
    }
}

/// Reads `data`; only an object with a `message` object yields a nested record.
struct DataSeed;

any_value_seed!(DataSeed);

impl<'de> Visitor<'de> for DataSeed {
    type Value = Option<RecordFields<'de>>;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("any JSON value")
    }

    lenient_visits!(None, [bool, numbers, str, unit]);

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let mut nested = None;
        while let Some(matches) = map.next_key_seed(KeyIs("message"))? {
            if matches {
                nested = map.next_value_seed(NestedRecordSeed)?;
            } else {
                map.next_value_seed(Skip)?;
            }
        }
        Ok(nested)
    }
}

/// Reads an object as a document, keeping it only when it is an object.
struct ObjectSeed;

impl<'de> DeserializeSeed<'de> for ObjectSeed {
    type Value = Option<Map<String, Value>>;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        Ok(match Value::deserialize(deserializer)? {
            Value::Object(object) => Some(object),
            Value::Null
            | Value::Bool(_)
            | Value::Number(_)
            | Value::String(_)
            | Value::Array(_) => None,
        })
    }
}

struct MessageSeed;

any_value_seed!(MessageSeed);

impl<'de> Visitor<'de> for MessageSeed {
    type Value = MessageFields<'de>;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("any JSON value")
    }

    lenient_visits!(MessageFields::default(), [bool, numbers, str, unit]);

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let mut message = MessageFields::default();
        while let Some(field) = map.next_key_seed(MessageKey)? {
            match field {
                MessageField::Id => message.id = map.next_value_seed(Text)?,
                MessageField::Model => message.model = map.next_value_seed(Text)?,
                MessageField::Usage => {
                    let extra = map.next_value_seed(UsageSeed)?;
                    message.usage = extra.counts;
                    message.advisors = extra.advisors;
                }
                MessageField::Content => {
                    let content = map.next_value_seed(ContentSeed)?;
                    message.tool_use_ids = content.tool_use_ids;
                    message.first_text = content.first_text;
                }
                MessageField::Other => map.next_value_seed(Skip)?,
            }
        }
        Ok(message)
    }
}

#[derive(Clone, Copy)]
enum MessageField {
    Id,
    Model,
    Usage,
    Content,
    Other,
}

struct MessageKey;

impl<'de> DeserializeSeed<'de> for MessageKey {
    type Value = MessageField;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<MessageField, D::Error> {
        deserializer.deserialize_str(self)
    }
}

impl Visitor<'_> for MessageKey {
    type Value = MessageField;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("an object key")
    }

    fn visit_str<E: Error>(self, name: &str) -> Result<MessageField, E> {
        Ok(match name {
            "id" => MessageField::Id,
            "model" => MessageField::Model,
            "usage" => MessageField::Usage,
            "content" => MessageField::Content,
            _ => MessageField::Other,
        })
    }
}

#[derive(Default)]
struct UsageExtra<'a> {
    counts: UsageCounts,
    advisors: Vec<AdvisorFields<'a>>,
}

struct UsageSeed;

any_value_seed!(UsageSeed);

impl<'de> Visitor<'de> for UsageSeed {
    type Value = UsageExtra<'de>;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("any JSON value")
    }

    lenient_visits!(UsageExtra::default(), [bool, numbers, str, unit]);

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let mut extra = UsageExtra::default();
        while let Some(field) = map.next_key_seed(UsageKey)? {
            match field {
                UsageField::Input => extra.counts.input = map.next_value_seed(Unsigned)?,
                UsageField::CacheRead => extra.counts.cache_read = map.next_value_seed(Unsigned)?,
                UsageField::CacheWrite => {
                    extra.counts.cache_write = map.next_value_seed(Unsigned)?;
                }
                UsageField::Output => extra.counts.output = map.next_value_seed(Unsigned)?,
                UsageField::CacheCreation => {
                    let (five, hour) = map.next_value_seed(CacheCreationSeed)?;
                    extra.counts.cache_write_5m = five;
                    extra.counts.cache_write_1h = hour;
                }
                UsageField::OutputDetails => {
                    extra.counts.reasoning = map.next_value_seed(ThinkingSeed)?;
                }
                UsageField::Iterations => extra.advisors = map.next_value_seed(IterationsSeed)?,
                UsageField::Other => map.next_value_seed(Skip)?,
            }
        }
        Ok(extra)
    }
}

#[derive(Clone, Copy)]
enum UsageField {
    Input,
    CacheRead,
    CacheWrite,
    Output,
    CacheCreation,
    OutputDetails,
    Iterations,
    Other,
}

struct UsageKey;

impl<'de> DeserializeSeed<'de> for UsageKey {
    type Value = UsageField;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<UsageField, D::Error> {
        deserializer.deserialize_str(self)
    }
}

impl Visitor<'_> for UsageKey {
    type Value = UsageField;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("an object key")
    }

    fn visit_str<E: Error>(self, name: &str) -> Result<UsageField, E> {
        Ok(match name {
            "input_tokens" => UsageField::Input,
            "cache_read_input_tokens" => UsageField::CacheRead,
            "cache_creation_input_tokens" => UsageField::CacheWrite,
            "output_tokens" => UsageField::Output,
            "cache_creation" => UsageField::CacheCreation,
            "output_tokens_details" => UsageField::OutputDetails,
            "iterations" => UsageField::Iterations,
            _ => UsageField::Other,
        })
    }
}

struct CacheCreationSeed;

any_value_seed!(CacheCreationSeed);

impl<'de> Visitor<'de> for CacheCreationSeed {
    type Value = (Option<u64>, Option<u64>);

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("any JSON value")
    }

    lenient_visits!((None, None), [bool, numbers, str, unit]);

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let (mut five, mut hour) = (None, None);
        while let Some(field) = map.next_key_seed(CacheCreationKey)? {
            match field {
                CacheCreationField::Five => five = map.next_value_seed(Unsigned)?,
                CacheCreationField::Hour => hour = map.next_value_seed(Unsigned)?,
                CacheCreationField::Other => map.next_value_seed(Skip)?,
            }
        }
        Ok((five, hour))
    }
}

#[derive(Clone, Copy)]
enum CacheCreationField {
    Five,
    Hour,
    Other,
}

struct CacheCreationKey;

impl<'de> DeserializeSeed<'de> for CacheCreationKey {
    type Value = CacheCreationField;

    fn deserialize<D: Deserializer<'de>>(
        self,
        deserializer: D,
    ) -> Result<CacheCreationField, D::Error> {
        deserializer.deserialize_str(self)
    }
}

impl Visitor<'_> for CacheCreationKey {
    type Value = CacheCreationField;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("an object key")
    }

    fn visit_str<E: Error>(self, name: &str) -> Result<CacheCreationField, E> {
        Ok(match name {
            "ephemeral_5m_input_tokens" => CacheCreationField::Five,
            "ephemeral_1h_input_tokens" => CacheCreationField::Hour,
            _ => CacheCreationField::Other,
        })
    }
}

struct ThinkingSeed;

any_value_seed!(ThinkingSeed);

impl<'de> Visitor<'de> for ThinkingSeed {
    type Value = Option<u64>;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("any JSON value")
    }

    lenient_visits!(None, [bool, numbers, str, unit]);

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let mut thinking = None;
        while let Some(matches) = map.next_key_seed(KeyIs("thinking_tokens"))? {
            if matches {
                thinking = map.next_value_seed(Unsigned)?;
            } else {
                map.next_value_seed(Skip)?;
            }
        }
        Ok(thinking)
    }
}

struct IterationsSeed;

any_value_seed!(IterationsSeed);

impl<'de> Visitor<'de> for IterationsSeed {
    type Value = Vec<AdvisorFields<'de>>;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("any JSON value")
    }

    fn visit_bool<E: Error>(self, _: bool) -> Result<Self::Value, E> {
        Ok(Vec::new())
    }

    fn visit_i64<E: Error>(self, _: i64) -> Result<Self::Value, E> {
        Ok(Vec::new())
    }

    fn visit_u64<E: Error>(self, _: u64) -> Result<Self::Value, E> {
        Ok(Vec::new())
    }

    fn visit_f64<E: Error>(self, _: f64) -> Result<Self::Value, E> {
        Ok(Vec::new())
    }

    fn visit_str<E: Error>(self, _: &str) -> Result<Self::Value, E> {
        Ok(Vec::new())
    }

    fn visit_unit<E: Error>(self) -> Result<Self::Value, E> {
        Ok(Vec::new())
    }

    fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<Self::Value, A::Error> {
        Skip.visit_map(map)?;
        Ok(Vec::new())
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let mut advisors = Vec::new();
        while let Some(advisor) = seq.next_element_seed(AdvisorSeed)? {
            if let Some(advisor) = advisor {
                advisors.push(advisor);
            }
        }
        Ok(advisors)
    }
}

struct AdvisorSeed;

any_value_seed!(AdvisorSeed);

impl<'de> Visitor<'de> for AdvisorSeed {
    type Value = Option<AdvisorFields<'de>>;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("any JSON value")
    }

    lenient_visits!(None, [bool, numbers, str, unit]);

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let mut advisor = AdvisorFields::default();
        let mut is_advisor = false;
        while let Some(field) = map.next_key_seed(AdvisorKey)? {
            match field {
                AdvisorField::Type => {
                    is_advisor = map.next_value_seed(Text)?.as_deref() == Some("advisor_message");
                }
                AdvisorField::Model => advisor.model = map.next_value_seed(Text)?,
                AdvisorField::Input => advisor.input = map.next_value_seed(Unsigned)?,
                AdvisorField::CacheRead => advisor.cache_read = map.next_value_seed(Unsigned)?,
                AdvisorField::CacheWrite => advisor.cache_write = map.next_value_seed(Unsigned)?,
                AdvisorField::Output => advisor.output = map.next_value_seed(Unsigned)?,
                AdvisorField::Reasoning => advisor.reasoning = map.next_value_seed(Unsigned)?,
                AdvisorField::Other => map.next_value_seed(Skip)?,
            }
        }
        Ok(is_advisor.then_some(advisor))
    }
}

#[derive(Clone, Copy)]
enum AdvisorField {
    Type,
    Model,
    Input,
    CacheRead,
    CacheWrite,
    Output,
    Reasoning,
    Other,
}

struct AdvisorKey;

impl<'de> DeserializeSeed<'de> for AdvisorKey {
    type Value = AdvisorField;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<AdvisorField, D::Error> {
        deserializer.deserialize_str(self)
    }
}

impl Visitor<'_> for AdvisorKey {
    type Value = AdvisorField;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("an object key")
    }

    fn visit_str<E: Error>(self, name: &str) -> Result<AdvisorField, E> {
        Ok(match name {
            "type" => AdvisorField::Type,
            "model" => AdvisorField::Model,
            "input_tokens" => AdvisorField::Input,
            "cache_read_input_tokens" => AdvisorField::CacheRead,
            "cache_creation_input_tokens" => AdvisorField::CacheWrite,
            "output_tokens" => AdvisorField::Output,
            "reasoning" => AdvisorField::Reasoning,
            _ => AdvisorField::Other,
        })
    }
}

#[derive(Default)]
struct ContentBits<'a> {
    tool_use_ids: Vec<Cow<'a, str>>,
    first_text: Option<Cow<'a, str>>,
}

struct ContentSeed;

any_value_seed!(ContentSeed);

impl<'de> Visitor<'de> for ContentSeed {
    type Value = ContentBits<'de>;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("any JSON value")
    }

    fn visit_bool<E: Error>(self, _: bool) -> Result<Self::Value, E> {
        Ok(ContentBits::default())
    }

    fn visit_i64<E: Error>(self, _: i64) -> Result<Self::Value, E> {
        Ok(ContentBits::default())
    }

    fn visit_u64<E: Error>(self, _: u64) -> Result<Self::Value, E> {
        Ok(ContentBits::default())
    }

    fn visit_f64<E: Error>(self, _: f64) -> Result<Self::Value, E> {
        Ok(ContentBits::default())
    }

    fn visit_str<E: Error>(self, _: &str) -> Result<Self::Value, E> {
        Ok(ContentBits::default())
    }

    fn visit_unit<E: Error>(self) -> Result<Self::Value, E> {
        Ok(ContentBits::default())
    }

    fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<Self::Value, A::Error> {
        Skip.visit_map(map)?;
        Ok(ContentBits::default())
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let mut bits = ContentBits::default();
        let mut first = true;
        while let Some(block) = seq.next_element_seed(ContentBlockSeed)? {
            if first {
                bits.first_text = block.text;
                first = false;
            }
            if block.tool_use {
                if let Some(id) = block.id {
                    bits.tool_use_ids.push(id);
                }
            }
        }
        Ok(bits)
    }
}

#[derive(Default)]
struct ContentBlock<'a> {
    tool_use: bool,
    id: Option<Cow<'a, str>>,
    text: Option<Cow<'a, str>>,
}

struct ContentBlockSeed;

any_value_seed!(ContentBlockSeed);

impl<'de> Visitor<'de> for ContentBlockSeed {
    type Value = ContentBlock<'de>;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("any JSON value")
    }

    lenient_visits!(ContentBlock::default(), [bool, numbers, str, unit]);

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let mut block = ContentBlock::default();
        while let Some(field) = map.next_key_seed(BlockKey)? {
            match field {
                BlockField::Type => {
                    block.tool_use = map.next_value_seed(Text)?.as_deref() == Some("tool_use");
                }
                BlockField::Id => block.id = map.next_value_seed(Text)?,
                BlockField::Text => block.text = map.next_value_seed(Text)?,
                BlockField::Other => map.next_value_seed(Skip)?,
            }
        }
        Ok(block)
    }
}

#[derive(Clone, Copy)]
enum BlockField {
    Type,
    Id,
    Text,
    Other,
}

struct BlockKey;

impl<'de> DeserializeSeed<'de> for BlockKey {
    type Value = BlockField;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<BlockField, D::Error> {
        deserializer.deserialize_str(self)
    }
}

impl Visitor<'_> for BlockKey {
    type Value = BlockField;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("an object key")
    }

    fn visit_str<E: Error>(self, name: &str) -> Result<BlockField, E> {
        Ok(match name {
            "type" => BlockField::Type,
            "id" => BlockField::Id,
            "text" => BlockField::Text,
            _ => BlockField::Other,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use proptest::prelude::*;
    use serde_json::Value;

    use super::{
        AdvisorFields, LineHead, LineType, MessageFields, RecordFields, UsageBody, UsageCounts,
    };
    use crate::sources::decode::{parse_record, text, unsigned};

    /// What the adapter read from a line before this pass existed, from a document.
    fn from_document(bytes: &[u8]) -> Option<LineHead<'static>> {
        let value = parse_record(bytes).ok()?;
        let line_type = match text(&value, &["type"]) {
            Some("assistant") => LineType::Assistant,
            Some("progress") => LineType::Progress,
            Some(_) | None => LineType::Other,
        };
        let owned = |path: &str| text(&value, &[path]).map(|text| Cow::Owned(text.to_owned()));
        Some(LineHead {
            line_type,
            sidechain: value.get("isSidechain").and_then(Value::as_bool) == Some(true),
            version: owned("version"),
            cwd: owned("cwd"),
            message_usage: unsigned(&value, &["message", "usage", "output_tokens"]).is_some(),
            nested_usage: value.pointer("/data/message").is_some_and(|nested| {
                unsigned(nested, &["message", "usage", "output_tokens"]).is_some()
            }),
        })
    }

    fn assert_matches_document(bytes: &[u8]) {
        let expected = from_document(bytes);
        let head = LineHead::read(bytes, true, true).ok();
        assert_eq!(head, expected, "{}", String::from_utf8_lossy(bytes));
        let unwanted = LineHead::read(bytes, false, false).ok();
        assert_eq!(unwanted.is_some(), expected.is_some());
        if let Some(unwanted) = unwanted {
            assert_eq!((unwanted.version, unwanted.cwd), (None, None));
        }
    }

    #[test]
    fn the_head_fails_exactly_when_the_document_does() {
        let deep = |depth: usize| {
            format!(r#"{{"type":"user","x":{}{}}}"#, "[".repeat(depth), "]".repeat(depth))
        };
        let mut cases: Vec<Vec<u8>> = [
            r#"{"type":"assistant","message":{"usage":{"output_tokens":3}}}"#,
            r#"{"type":"user","content":"\ud800"}"#,
            r#"{"type":"user","content":"\ud800\udc00"}"#,
            r#"{"type":"user","content":"\x"}"#,
            r#"{"type":"user","n":1e400}"#,
            r#"{"type":"user","n":-1e400}"#,
            r#"{"type":"user","n":1e-400}"#,
            r#"{"type":"user","n":01}"#,
            r#"{"type":"user","n":1.}"#,
            r#"{"type":"user"} x"#,
            r#"{"type":"user"}   "#,
            r#"{"type":"user",}"#,
            r#"{"type":"user""#,
            r#"{"ty\u0070e":"assist\u0061nt","message":{"us\u0061ge":{"output_tokens":1}}}"#,
            r"{1:2}",
            r#"[1,2,{"type":"assistant"}]"#,
            r#""assistant""#,
            "3",
            "null",
            "",
            " ",
        ]
        .iter()
        .map(|line| line.as_bytes().to_vec())
        .collect();
        cases.push(deep(126).into_bytes());
        cases.push(deep(127).into_bytes());
        cases.push(deep(128).into_bytes());
        cases.push(deep(300).into_bytes());
        cases.push(b"{\"type\":\"user\",\"content\":\"\xff\"}".to_vec());
        cases.push(b"{\"type\":\"user\",\"\xfe\":1}".to_vec());
        for case in &cases {
            assert_matches_document(case);
        }
    }

    #[test]
    fn fields_read_as_a_document_reads_them() {
        for line in [
            r#"{"type":"assistant","message":{"usage":{"output_tokens":3}},"isSidechain":true,"version":"2.1.0","cwd":"/work/app"}"#,
            r#"{"type":"assistant","type":"user","message":{"usage":{"output_tokens":3}}}"#,
            r#"{"type":"user","type":"assistant","message":{"usage":{"output_tokens":3}}}"#,
            r#"{"type":null,"message":{"usage":{"output_tokens":3}}}"#,
            r#"{"type":["assistant"],"message":{"usage":{"output_tokens":3}}}"#,
            r#"{"type":"assistant","message":{"usage":{"output_tokens":3}},"message":{"usage":{}}}"#,
            r#"{"type":"assistant","message":{"usage":{"output_tokens":3},"usage":null}}"#,
            r#"{"type":"assistant","message":{"usage":{"output_tokens":null}}}"#,
            r#"{"type":"assistant","message":{"usage":{"output_tokens":"3"}}}"#,
            r#"{"type":"assistant","message":{"usage":{"output_tokens":3.0}}}"#,
            r#"{"type":"assistant","message":{"usage":{"output_tokens":3.5}}}"#,
            r#"{"type":"assistant","message":{"usage":{"output_tokens":-0}}}"#,
            r#"{"type":"assistant","message":{"usage":{"output_tokens":-0.0}}}"#,
            r#"{"type":"assistant","message":{"usage":{"output_tokens":-1}}}"#,
            r#"{"type":"assistant","message":{"usage":{"output_tokens":18446744073709551615}}}"#,
            r#"{"type":"assistant","message":{"usage":{"output_tokens":18446744073709551616}}}"#,
            r#"{"type":"assistant","message":{"usage":{"output_tokens":9007199254740992.0}}}"#,
            r#"{"type":"assistant","message":{"usage":{"output_tokens":9007199254740994.0}}}"#,
            r#"{"type":"assistant","message":[{"usage":{"output_tokens":3}}]}"#,
            r#"{"type":"assistant","message":{"usage":[3]}}"#,
            r#"{"type":"assistant","usage":{"output_tokens":3}}"#,
            r#"{"type":"progress","data":{"message":{"message":{"usage":{"output_tokens":4}}}}}"#,
            r#"{"type":"progress","data":{"message":null}}"#,
            r#"{"type":"progress","data":[{"message":{"message":{"usage":{"output_tokens":4}}}}]}"#,
            r#"{"type":"progress","data":{"message":{"usage":{"output_tokens":4}}}}"#,
            r#"{"type":"progress","message":{"usage":{"output_tokens":4}}}"#,
            r#"{"isSidechain":true,"isSidechain":"true"}"#,
            r#"{"isSidechain":false,"isSidechain":true}"#,
            r#"{"isSidechain":1}"#,
            r#"{"version":"1","version":2,"cwd":null}"#,
            r#"{"version":"a\"b","cwd":"C:\\work\\app"}"#,
            r#"{"cwd":{"path":"/x"},"version":["1"]}"#,
            r"{}",
        ] {
            assert_matches_document(line.as_bytes());
        }
    }

    fn owned_text(value: &Value, path: &[&str]) -> Option<Cow<'static, str>> {
        text(value, path).map(|text| Cow::Owned(text.to_owned()))
    }

    fn message_from_document(value: &Value) -> MessageFields<'static> {
        let content = value.pointer("/message/content").and_then(Value::as_array);
        let first = content.and_then(|blocks| blocks.first());
        MessageFields {
            id: owned_text(value, &["message", "id"]),
            model: owned_text(value, &["message", "model"]),
            usage: UsageCounts {
                input: unsigned(value, &["message", "usage", "input_tokens"]),
                cache_read: unsigned(value, &["message", "usage", "cache_read_input_tokens"]),
                cache_write: unsigned(value, &["message", "usage", "cache_creation_input_tokens"]),
                cache_write_5m: unsigned(
                    value,
                    &["message", "usage", "cache_creation", "ephemeral_5m_input_tokens"],
                ),
                cache_write_1h: unsigned(
                    value,
                    &["message", "usage", "cache_creation", "ephemeral_1h_input_tokens"],
                ),
                output: unsigned(value, &["message", "usage", "output_tokens"]),
                reasoning: unsigned(
                    value,
                    &["message", "usage", "output_tokens_details", "thinking_tokens"],
                ),
            },
            tool_use_ids: content
                .into_iter()
                .flatten()
                .filter(|block| text(block, &["type"]) == Some("tool_use"))
                .filter_map(|block| owned_text(block, &["id"]))
                .collect(),
            first_text: first.and_then(|block| owned_text(block, &["text"])),
            advisors: value
                .pointer("/message/usage/iterations")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter(|iteration| text(iteration, &["type"]) == Some("advisor_message"))
                .map(|iteration| AdvisorFields {
                    model: owned_text(iteration, &["model"]),
                    input: unsigned(iteration, &["input_tokens"]),
                    cache_read: unsigned(iteration, &["cache_read_input_tokens"]),
                    cache_write: unsigned(iteration, &["cache_creation_input_tokens"]),
                    output: unsigned(iteration, &["output_tokens"]),
                    reasoning: unsigned(iteration, &["reasoning"]),
                })
                .collect(),
        }
    }

    fn record_from_document(value: &Value) -> RecordFields<'static> {
        RecordFields {
            session: owned_text(value, &["sessionId"]),
            uuid: owned_text(value, &["uuid"]),
            request_id: owned_text(value, &["requestId"]),
            timestamp: owned_text(value, &["timestamp"]),
            effort: owned_text(value, &["effort"]),
            cwd: owned_text(value, &["cwd"]),
            is_api_error: value.get("isApiErrorMessage").and_then(Value::as_bool) == Some(true),
            quota: value.get("quotaLimits").and_then(Value::as_object).cloned(),
            api_block_index: unsigned(value, &["apiBlockIndex"]),
            message: message_from_document(value),
        }
    }

    fn body_from_document(bytes: &[u8]) -> Option<UsageBody<'static>> {
        let value = parse_record(bytes).ok()?;
        Some(UsageBody {
            top: record_from_document(&value),
            nested: value
                .get("data")
                .and_then(|data| data.get("message"))
                .and_then(|nested| nested.as_object().map(|_| record_from_document(nested))),
        })
    }

    fn assert_body_matches_document(bytes: &[u8]) {
        let expected = body_from_document(bytes);
        let body = UsageBody::read(bytes).ok();
        assert_eq!(body, expected, "{}", String::from_utf8_lossy(bytes));
    }

    #[test]
    fn the_body_reads_usage_fields_as_a_document_does() {
        for line in [
            r#"{"type":"assistant","sessionId":"s1","message":{"id":"m1","model":"claude","usage":{"input_tokens":2,"output_tokens":3}}}"#,
            r#"{"type":"assistant","message":{"usage":{"output_tokens":3,"cache_creation":{"ephemeral_5m_input_tokens":1,"ephemeral_1h_input_tokens":4}}}}"#,
            r#"{"type":"assistant","apiBlockIndex":7,"message":{"usage":{"output_tokens":1}}}"#,
            r#"{"type":"assistant","isApiErrorMessage":true,"message":{"content":[{"type":"text","text":"Claude AI usage limit reached tonight"}],"usage":{"output_tokens":0}}}"#,
            r#"{"type":"assistant","message":{"content":[{"type":"tool_use","id":"t1","input":{"huge":true}},{"type":"tool_use","id":"t2"}],"usage":{"output_tokens":1}}}"#,
            r#"{"type":"assistant","quotaLimits":{"rateLimitType":"five_hour","used":1},"message":{"usage":{"output_tokens":1}}}"#,
            r#"{"type":"assistant","message":{"usage":{"output_tokens":1,"iterations":[{"type":"advisor_message","model":"a","input_tokens":2},{"type":"other"}]}}}"#,
            r#"{"type":"progress","sessionId":"outer","data":{"message":{"sessionId":"inner","message":{"id":"m2","usage":{"output_tokens":4}}}}}"#,
            r#"{"type":"assistant","message":{"usage":{"output_tokens":3}},"message":{"usage":{}}}"#,
            r#"{"type":"assistant","quotaLimits":"none","message":{"usage":{"output_tokens":1}}}"#,
        ] {
            assert_body_matches_document(line.as_bytes());
        }
    }

    #[test]
    fn unescaped_strings_are_borrowed_from_the_line() {
        let head = LineHead::read(br#"{"version":"2.1.0","cwd":"a\/b"}"#, true, true).unwrap();
        assert!(matches!(head.version, Some(Cow::Borrowed("2.1.0"))));
        assert!(matches!(head.cwd, Some(Cow::Owned(ref cwd)) if cwd == "a/b"));
    }

    /// A JSON object text from generated entries, which may repeat a key: a document keeps
    /// the last value, and a derived struct would reject the line.
    fn object(
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
    fn leaf() -> BoxedStrategy<String> {
        prop_oneof![
            12 => prop_oneof![
                Just("null"),
                Just("true"),
                Just("false"),
                Just("\"assistant\""),
                Just("\"progress\""),
                Just("\"assist\\u0061nt\""),
                Just("\"user\""),
                Just("\"/work/app\""),
                Just("\"C:\\\\work\\\\app\""),
                Just("\"2.1.0\""),
                Just("2.0"),
                Just("2.5"),
                Just("-0"),
                Just("-1"),
                Just("18446744073709551616"),
                Just("[]"),
                Just("[1,{\"output_tokens\":2}]"),
                Just("{}"),
            ]
            .prop_map(str::to_owned),
            6 => (0_u64..1_000).prop_map(|number| number.to_string()),
            1 => prop_oneof![Just("\"\\ud800\""), Just("1e400"), Just("01"), Just("\"\\q\"")]
                .prop_map(str::to_owned),
        ]
        .boxed()
    }

    /// A token count: usually a number, sometimes anything else.
    fn count() -> BoxedStrategy<String> {
        prop_oneof![5 => (0_u64..1_000).prop_map(|number| number.to_string()), 2 => leaf()].boxed()
    }

    fn usage() -> BoxedStrategy<String> {
        let entries = prop_oneof![
            6 => count().prop_map(|value| ("output_tokens", value)),
            1 => count().prop_map(|value| ("output_t\\u006fkens", value)),
            1 => count().prop_map(|value| ("input_tokens", value)),
        ];
        prop_oneof![1 => leaf(), 6 => object(entries.boxed(), 0..3)].boxed()
    }

    /// A `message`, which may nest another one as a progress record's `data.message` does.
    fn message(depth: u32) -> BoxedStrategy<String> {
        let mut entries = vec![
            (6, usage().prop_map(|value| ("usage", value)).boxed()),
            (1, leaf().prop_map(|value| ("id", value)).boxed()),
        ];
        if depth > 0 {
            entries.push((6, message(depth - 1).prop_map(|value| ("message", value)).boxed()));
        }
        let entries = prop::strategy::Union::new_weighted(entries).boxed();
        prop_oneof![1 => leaf(), 8 => object(entries, 0..3)].boxed()
    }

    /// The entries of a record that bears usage, as an assistant or a progress record.
    fn usage_record() -> BoxedStrategy<Vec<(&'static str, String)>> {
        prop_oneof![
            count().prop_map(|output| vec![
                ("type", "\"assistant\"".to_owned()),
                ("message", format!(r#"{{"usage":{{"output_tokens":{output}}}}}"#)),
            ]),
            count().prop_map(|output| vec![
                (
                    "data",
                    format!(
                        r#"{{"message":{{"message":{{"usage":{{"output_tokens":{output}}}}}}}}}"#
                    )
                ),
                ("type", "\"progress\"".to_owned()),
            ]),
            Just(Vec::new()),
        ]
        .boxed()
    }

    /// A transcript line over the fields the head reads: often a usage record, with other
    /// fields before and after it that may repeat and so override its fields.
    fn line() -> BoxedStrategy<String> {
        let entries = prop_oneof![
            2 => prop_oneof![Just("\"assistant\""), Just("\"progress\"")]
                .prop_map(|value| ("type", value.to_owned())),
            1 => leaf().prop_map(|value| ("type", value)),
            1 => leaf().prop_map(|value| ("ty\\u0070e", value)),
            3 => prop_oneof![Just("true".to_owned()), leaf()]
                .prop_map(|value| ("isSidechain", value)),
            2 => leaf().prop_map(|value| ("version", value)),
            2 => leaf().prop_map(|value| ("cwd", value)),
            2 => message(1).prop_map(|value| ("message", value)),
            2 => message(2).prop_map(|value| ("data", value)),
            3 => leaf().prop_map(|value| ("content", value)),
        ]
        .boxed();
        let fields = (
            prop::collection::vec(entries.clone(), 0..4),
            usage_record(),
            prop::collection::vec(entries, 0..3),
        )
            .prop_map(|(before, record, after)| {
                let fields: Vec<String> = before
                    .into_iter()
                    .chain(record)
                    .chain(after)
                    .map(|(key, value)| format!("\"{key}\":{value}"))
                    .collect();
                format!("{{{}}}", fields.join(","))
            });
        prop_oneof![1 => leaf(), 20 => fields].boxed()
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(2_000))]

        #[test]
        fn generated_lines_read_as_documents_read_them(
            line in line(),
            cut in any::<prop::sample::Index>(),
            truncate in prop::bool::weighted(0.1),
        ) {
            let bytes = line.as_bytes();
            let bytes = if truncate { &bytes[..cut.index(bytes.len() + 1)] } else { bytes };
            assert_matches_document(bytes);
        }
    }
}
