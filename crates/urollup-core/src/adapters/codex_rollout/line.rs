//! The typed first pass over a Codex rollout line that may be relevant.
//!
//! [`Line::read`] reads every field the adapter uses from any relevant record kind,
//! borrowing strings from the line and building no JSON document. A `rate_limits` object
//! is reduced to compact native JSON and window names while it is read.
//! A record's type is only known once its `type` field is read, which may follow its
//! payload, so the payload fields of every kind are read.
//!
//! The pass is exact with respect to parsing a `serde_json::Value` document followed by
//! the document path helpers:
//!
//! - Every value, kept or ignored, is read through `deserialize_any`, the path a
//!   `serde_json::Value` takes, so a line fails here exactly when it fails to parse as a
//!   document: invalid UTF-8 or escapes in any string, a number out of range, nesting past
//!   the recursion limit, or trailing characters.
//! - A field that repeats takes its last value, as a document's object does; a repeated
//!   object replaces every field read from the earlier one.
//! - A field that is null or of another type reads as missing, as
//!   [`text`](crate::sources::decode::text) and [`unsigned`] treat it, and a value that is
//!   not an object has no fields. Usage objects are the exception the document helpers
//!   made: a `usage`, `latest_token_usage_record`, `total_token_usage` or
//!   `last_token_usage` key is present whatever its value, with no counts when the value
//!   is not an object.

use std::borrow::Cow;
use std::collections::BTreeMap;
use std::fmt;

use serde::de::{DeserializeSeed, Deserializer, Error, MapAccess, SeqAccess, Visitor};
use serde_json::Value;

use super::CodexUsage;
use crate::sources::decode::{Skip, unsigned};

/// A record's `type`, as far as the adapter distinguishes it.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) enum RecordType {
    SessionMeta,
    TurnContext,
    TokenUsageRecord,
    Compacted,
    EventMsg,
    /// Any other type, a missing type, or a type that is not a string.
    #[default]
    Other,
}

impl RecordType {
    fn named(name: &str) -> Self {
        match name {
            "session_meta" => Self::SessionMeta,
            "turn_context" => Self::TurnContext,
            "token_usage_record" => Self::TokenUsageRecord,
            "compacted" => Self::Compacted,
            "event_msg" => Self::EventMsg,
            _ => Self::Other,
        }
    }
}

/// An `event_msg` payload's `type`, as far as the adapter distinguishes it.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) enum EventType {
    ThreadSettingsApplied,
    TokenCount,
    /// Any other type, a missing type, or a type that is not a string.
    #[default]
    Other,
}

impl EventType {
    fn named(name: &str) -> Self {
        match name {
            "thread_settings_applied" => Self::ThreadSettingsApplied,
            "token_count" => Self::TokenCount,
            _ => Self::Other,
        }
    }
}

/// The fields of one rollout line that any relevant record kind reads.
#[derive(Debug, Default, PartialEq)]
pub(super) struct Line<'a> {
    pub(super) record_type: RecordType,
    pub(super) timestamp: Option<Cow<'a, str>>,
    pub(super) ordinal: Option<u64>,
    pub(super) payload: Payload<'a>,
}

/// The fields of a record's `payload` that any relevant record kind reads.
#[derive(Debug, Default, PartialEq)]
pub(super) struct Payload<'a> {
    /// An `event_msg` payload's `type`.
    pub(super) event_type: EventType,
    pub(super) id: Option<Cow<'a, str>>,
    pub(super) cli_version: Option<Cow<'a, str>>,
    pub(super) source: Option<Cow<'a, str>>,
    pub(super) thread_source: Option<Cow<'a, str>>,
    pub(super) cwd: Option<Cow<'a, str>>,
    pub(super) parent_thread_id: Option<Cow<'a, str>>,
    pub(super) forked_from_id: Option<Cow<'a, str>>,
    pub(super) subagent_history_start_ordinal: Option<u64>,
    pub(super) turn_id: Option<Cow<'a, str>>,
    pub(super) model: Option<Cow<'a, str>>,
    pub(super) effort: Option<Cow<'a, str>>,
    /// The payload's own usage record fields, whose `thread_id` a
    /// `thread_settings_applied` event also names.
    pub(super) usage_record: UsageFields<'a>,
    pub(super) latest_token_usage_record: Option<UsageFields<'a>>,
    /// `info.total_token_usage`.
    pub(super) total_token_usage: Option<CodexUsage>,
    /// `info.last_token_usage`.
    pub(super) last_token_usage: Option<CodexUsage>,
    /// `rate_limits`, when it is an object, already reduced to native JSON and windows.
    pub(super) rate_limits: Option<DecodedLimits>,
}

/// A `rate_limits` object without a `serde_json::Value` document.
#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct DecodedLimits {
    pub limit_name: Option<String>,
    pub windows: Vec<&'static str>,
    pub native: String,
}

/// The fields of a usage record: a `token_usage_record` payload, or the latest one a
/// `compacted` payload copies.
#[derive(Debug, Default, PartialEq)]
pub(super) struct UsageFields<'a> {
    pub(super) thread_id: Option<Cow<'a, str>>,
    pub(super) response_id: Option<Cow<'a, str>>,
    pub(super) root_turn_id: Option<Cow<'a, str>>,
    pub(super) usage: Option<CodexUsage>,
}

impl<'a> Line<'a> {
    /// Reads one line; fails exactly when the line is not a valid JSON document.
    pub(super) fn read(bytes: &'a [u8]) -> Result<Self, serde_json::Error> {
        let mut deserializer = serde_json::Deserializer::from_slice(bytes);
        let line = LineSeed.deserialize(&mut deserializer)?;
        deserializer.end()?;
        Ok(line)
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

/// Reads an object key through a function from its unescaped name.
struct Key<F>(fn(&str) -> F);

impl<'de, F> DeserializeSeed<'de> for Key<F> {
    type Value = F;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<F, D::Error> {
        deserializer.deserialize_str(self)
    }
}

impl<F> Visitor<'_> for Key<F> {
    type Value = F;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("an object key")
    }

    fn visit_str<E: Error>(self, name: &str) -> Result<F, E> {
        Ok((self.0)(name))
    }
}

/// The top-level fields the pass reads.
#[derive(Clone, Copy)]
enum LineField {
    Type,
    Timestamp,
    Ordinal,
    Payload,
    Other,
}

impl LineField {
    fn named(name: &str) -> Self {
        match name {
            "type" => Self::Type,
            "timestamp" => Self::Timestamp,
            "ordinal" => Self::Ordinal,
            "payload" => Self::Payload,
            _ => Self::Other,
        }
    }
}

/// Reads a whole line; a line that is not an object has no fields.
struct LineSeed;

any_value_seed!(LineSeed);

impl<'de> Visitor<'de> for LineSeed {
    type Value = Line<'de>;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("any JSON value")
    }

    lenient_visits!(Line::default(), [bool, numbers, str, unit]);

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let mut line = Line::default();
        while let Some(field) = map.next_key_seed(Key(LineField::named))? {
            match field {
                LineField::Type => {
                    line.record_type = map.next_value_seed(Name(RecordType::named))?;
                }
                LineField::Timestamp => line.timestamp = map.next_value_seed(Text)?,
                LineField::Ordinal => line.ordinal = map.next_value_seed(Unsigned)?,
                LineField::Payload => line.payload = map.next_value_seed(PayloadSeed)?,
                LineField::Other => map.next_value_seed(Skip)?,
            }
        }
        Ok(line)
    }
}

/// The payload fields the pass reads.
#[derive(Clone, Copy)]
enum PayloadField {
    Type,
    Id,
    CliVersion,
    Source,
    ThreadSource,
    Cwd,
    ParentThreadId,
    ForkedFromId,
    SubagentHistoryStartOrdinal,
    TurnId,
    Model,
    Effort,
    Usage(UsageField),
    LatestTokenUsageRecord,
    Info,
    RateLimits,
    Other,
}

impl PayloadField {
    fn named(name: &str) -> Self {
        match name {
            "type" => Self::Type,
            "id" => Self::Id,
            "cli_version" => Self::CliVersion,
            "source" => Self::Source,
            "thread_source" => Self::ThreadSource,
            "cwd" => Self::Cwd,
            "parent_thread_id" => Self::ParentThreadId,
            "forked_from_id" => Self::ForkedFromId,
            "subagent_history_start_ordinal" => Self::SubagentHistoryStartOrdinal,
            "turn_id" => Self::TurnId,
            "model" => Self::Model,
            "effort" => Self::Effort,
            "latest_token_usage_record" => Self::LatestTokenUsageRecord,
            "info" => Self::Info,
            "rate_limits" => Self::RateLimits,
            "thread_id" => Self::Usage(UsageField::ThreadId),
            "response_id" => Self::Usage(UsageField::ResponseId),
            "root_turn_id" => Self::Usage(UsageField::RootTurnId),
            "usage" => Self::Usage(UsageField::Usage),
            _ => Self::Other,
        }
    }
}

/// Reads a payload; a payload that is not an object has no fields.
struct PayloadSeed;

any_value_seed!(PayloadSeed);

impl<'de> Visitor<'de> for PayloadSeed {
    type Value = Payload<'de>;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("any JSON value")
    }

    lenient_visits!(Payload::default(), [bool, numbers, str, unit]);

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let mut payload = Payload::default();
        while let Some(field) = map.next_key_seed(Key(PayloadField::named))? {
            match field {
                PayloadField::Type => {
                    payload.event_type = map.next_value_seed(Name(EventType::named))?;
                }
                PayloadField::Id => payload.id = map.next_value_seed(Text)?,
                PayloadField::CliVersion => payload.cli_version = map.next_value_seed(Text)?,
                PayloadField::Source => payload.source = map.next_value_seed(Text)?,
                PayloadField::ThreadSource => payload.thread_source = map.next_value_seed(Text)?,
                PayloadField::Cwd => payload.cwd = map.next_value_seed(Text)?,
                PayloadField::ParentThreadId => {
                    payload.parent_thread_id = map.next_value_seed(Text)?;
                }
                PayloadField::ForkedFromId => payload.forked_from_id = map.next_value_seed(Text)?,
                PayloadField::SubagentHistoryStartOrdinal => {
                    payload.subagent_history_start_ordinal = map.next_value_seed(Unsigned)?;
                }
                PayloadField::TurnId => payload.turn_id = map.next_value_seed(Text)?,
                PayloadField::Model => payload.model = map.next_value_seed(Text)?,
                PayloadField::Effort => payload.effort = map.next_value_seed(Text)?,
                PayloadField::Usage(field) => payload.usage_record.read(field, &mut map)?,
                PayloadField::LatestTokenUsageRecord => {
                    payload.latest_token_usage_record = Some(map.next_value_seed(UsageFieldsSeed)?);
                }
                PayloadField::Info => {
                    (payload.total_token_usage, payload.last_token_usage) =
                        map.next_value_seed(InfoSeed)?;
                }
                PayloadField::RateLimits => {
                    payload.rate_limits = map.next_value_seed(RateLimitsSeed)?;
                }
                PayloadField::Other => map.next_value_seed(Skip)?,
            }
        }
        Ok(payload)
    }
}

/// The fields of a usage record.
#[derive(Clone, Copy)]
enum UsageField {
    ThreadId,
    ResponseId,
    RootTurnId,
    Usage,
    Other,
}

impl UsageField {
    fn named(name: &str) -> Self {
        match name {
            "thread_id" => Self::ThreadId,
            "response_id" => Self::ResponseId,
            "root_turn_id" => Self::RootTurnId,
            "usage" => Self::Usage,
            _ => Self::Other,
        }
    }
}

impl<'de> UsageFields<'de> {
    /// Reads the value of one usage record field from `map`.
    fn read<A: MapAccess<'de>>(&mut self, field: UsageField, map: &mut A) -> Result<(), A::Error> {
        match field {
            UsageField::ThreadId => self.thread_id = map.next_value_seed(Text)?,
            UsageField::ResponseId => self.response_id = map.next_value_seed(Text)?,
            UsageField::RootTurnId => self.root_turn_id = map.next_value_seed(Text)?,
            UsageField::Usage => self.usage = Some(map.next_value_seed(UsageSeed)?),
            UsageField::Other => map.next_value_seed(Skip)?,
        }
        Ok(())
    }
}

/// Reads a usage record; one that is not an object has no fields.
struct UsageFieldsSeed;

any_value_seed!(UsageFieldsSeed);

impl<'de> Visitor<'de> for UsageFieldsSeed {
    type Value = UsageFields<'de>;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("any JSON value")
    }

    lenient_visits!(UsageFields::default(), [bool, numbers, str, unit]);

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let mut fields = UsageFields::default();
        while let Some(field) = map.next_key_seed(Key(UsageField::named))? {
            fields.read(field, &mut map)?;
        }
        Ok(fields)
    }
}

/// The counts of a usage object.
#[derive(Clone, Copy)]
enum CountField {
    Input,
    CachedInput,
    CacheWriteInput,
    Output,
    ReasoningOutput,
    Total,
    Other,
}

impl CountField {
    fn named(name: &str) -> Self {
        match name {
            "input_tokens" => Self::Input,
            "cached_input_tokens" => Self::CachedInput,
            "cache_write_input_tokens" => Self::CacheWriteInput,
            "output_tokens" => Self::Output,
            "reasoning_output_tokens" => Self::ReasoningOutput,
            "total_tokens" => Self::Total,
            _ => Self::Other,
        }
    }
}

/// Reads a usage object's counts; a value that is not an object has none.
struct UsageSeed;

any_value_seed!(UsageSeed);

impl<'de> Visitor<'de> for UsageSeed {
    type Value = CodexUsage;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("any JSON value")
    }

    lenient_visits!(CodexUsage::default(), [bool, numbers, str, unit]);

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let mut usage = CodexUsage::default();
        while let Some(field) = map.next_key_seed(Key(CountField::named))? {
            let count = match field {
                CountField::Input => &mut usage.input,
                CountField::CachedInput => &mut usage.cached_input,
                CountField::CacheWriteInput => &mut usage.cache_write_input,
                CountField::Output => &mut usage.output,
                CountField::ReasoningOutput => &mut usage.reasoning_output,
                CountField::Total => &mut usage.total,
                CountField::Other => {
                    map.next_value_seed(Skip)?;
                    continue;
                }
            };
            *count = map.next_value_seed(Unsigned)?;
        }
        Ok(usage)
    }
}

/// The fields of a token count's `info`.
#[derive(Clone, Copy)]
enum InfoField {
    Total,
    Last,
    Other,
}

impl InfoField {
    fn named(name: &str) -> Self {
        match name {
            "total_token_usage" => Self::Total,
            "last_token_usage" => Self::Last,
            _ => Self::Other,
        }
    }
}

/// Reads `info` into its total and last usage; a value that is not an object has neither.
struct InfoSeed;

any_value_seed!(InfoSeed);

impl<'de> Visitor<'de> for InfoSeed {
    type Value = (Option<CodexUsage>, Option<CodexUsage>);

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("any JSON value")
    }

    lenient_visits!((None, None), [bool, numbers, str, unit]);

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let (mut total, mut last) = (None, None);
        while let Some(field) = map.next_key_seed(Key(InfoField::named))? {
            match field {
                InfoField::Total => total = Some(map.next_value_seed(UsageSeed)?),
                InfoField::Last => last = Some(map.next_value_seed(UsageSeed)?),
                InfoField::Other => map.next_value_seed(Skip)?,
            }
        }
        Ok((total, last))
    }
}

/// Reads a `rate_limits` value, keeping it only when it is an object.
struct RateLimitsSeed;

any_value_seed!(RateLimitsSeed);

impl<'de> Visitor<'de> for RateLimitsSeed {
    type Value = Option<DecodedLimits>;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("any JSON value")
    }

    lenient_visits!(None, [bool, numbers, str, unit]);

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let mut fields = BTreeMap::new();
        while let Some(key) = map.next_key::<String>()? {
            fields.insert(key, map.next_value_seed(CompactJson)?);
        }
        Ok(Some(DecodedLimits::from_fields(&fields)))
    }
}

impl DecodedLimits {
    fn from_fields(fields: &BTreeMap<String, String>) -> Self {
        let native = compact_object(fields);
        let json_string = |key: &str| {
            fields.get(key).and_then(|value| serde_json::from_str::<String>(value).ok())
        };
        Self {
            limit_name: json_string("limit_id").or_else(|| json_string("limit_name")),
            windows: ["primary", "secondary"]
                .into_iter()
                .filter(|window| fields.get(*window).is_some_and(|value| value != "null"))
                .collect(),
            native,
        }
    }
}

/// Compact JSON text of one value, matching `serde_json::to_string` on a document.
struct CompactJson;

any_value_seed!(CompactJson);

impl<'de> Visitor<'de> for CompactJson {
    type Value = String;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("any JSON value")
    }

    fn visit_bool<E: Error>(self, value: bool) -> Result<String, E> {
        Ok(if value { "true".to_owned() } else { "false".to_owned() })
    }

    fn visit_i64<E: Error>(self, value: i64) -> Result<String, E> {
        Ok(serde_json::to_string(&Value::from(value)).unwrap_or_else(|_| value.to_string()))
    }

    fn visit_u64<E: Error>(self, value: u64) -> Result<String, E> {
        Ok(serde_json::to_string(&Value::from(value)).unwrap_or_else(|_| value.to_string()))
    }

    fn visit_f64<E: Error>(self, value: f64) -> Result<String, E> {
        Ok(serde_json::to_string(&Value::from(value)).unwrap_or_else(|_| value.to_string()))
    }

    fn visit_str<E: Error>(self, value: &str) -> Result<String, E> {
        Ok(serde_json::to_string(value).unwrap_or_else(|_| "\"\"".to_owned()))
    }

    fn visit_unit<E: Error>(self) -> Result<String, E> {
        Ok("null".to_owned())
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<String, A::Error> {
        let mut items = Vec::new();
        while let Some(item) = seq.next_element_seed(CompactJson)? {
            items.push(item);
        }
        Ok(format!("[{}]", items.join(",")))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<String, A::Error> {
        let mut fields = BTreeMap::new();
        while let Some(key) = map.next_key::<String>()? {
            fields.insert(key, map.next_value_seed(CompactJson)?);
        }
        Ok(compact_object(&fields))
    }
}

fn compact_object(fields: &BTreeMap<String, String>) -> String {
    let mut out = String::from("{");
    for (index, (key, value)) in fields.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str(&serde_json::to_string(key).unwrap_or_else(|_| "\"\"".to_owned()));
        out.push(':');
        out.push_str(value);
    }
    out.push('}');
    out
}

/// Reads a string value through a function from its text; any other value is the
/// function's default.
struct Name<T>(fn(&str) -> T);

impl<'de, T: Default> DeserializeSeed<'de> for Name<T> {
    type Value = T;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<T, D::Error> {
        deserializer.deserialize_any(self)
    }
}

impl<'de, T: Default> Visitor<'de> for Name<T> {
    type Value = T;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("any JSON value")
    }

    lenient_visits!(T::default(), [bool, numbers, unit, map]);

    fn visit_str<E: Error>(self, name: &str) -> Result<T, E> {
        Ok((self.0)(name))
    }
}

/// Reads a string value, borrowing it from the line unless it has escapes.
struct Text;

any_value_seed!(Text);

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

/// Reads an unsigned integer value as [`unsigned`] reads it from a document.
struct Unsigned;

any_value_seed!(Unsigned);

impl<'de> Visitor<'de> for Unsigned {
    type Value = Option<u64>;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("any JSON value")
    }

    lenient_visits!(None, [bool, str, unit, map]);

    // Each number becomes the document value a parse would build, so the check is the
    // one `unsigned` applies.
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

#[cfg(test)]
mod tests {
    use std::borrow::Cow;
    use std::collections::BTreeMap;

    use proptest::prelude::*;
    use serde_json::Value;

    use super::{DecodedLimits, EventType, Line, Payload, RecordType, UsageFields};
    use crate::adapters::codex_rollout::CodexUsage;
    use crate::sources::decode::{parse_record, text, unsigned};

    fn owned(value: &Value, key: &str) -> Option<Cow<'static, str>> {
        text(value, &[key]).map(|text| Cow::Owned(text.to_owned()))
    }

    /// A usage object's counts, as the adapter read them from a document.
    fn usage(value: &Value) -> CodexUsage {
        CodexUsage {
            input: unsigned(value, &["input_tokens"]),
            cached_input: unsigned(value, &["cached_input_tokens"]),
            cache_write_input: unsigned(value, &["cache_write_input_tokens"]),
            output: unsigned(value, &["output_tokens"]),
            reasoning_output: unsigned(value, &["reasoning_output_tokens"]),
            total: unsigned(value, &["total_tokens"]),
        }
    }

    /// A usage record's fields, as the adapter read them from a document.
    fn usage_fields(payload: &Value) -> UsageFields<'static> {
        UsageFields {
            thread_id: owned(payload, "thread_id"),
            response_id: owned(payload, "response_id"),
            root_turn_id: owned(payload, "root_turn_id"),
            usage: payload.get("usage").map(usage),
        }
    }

    /// What the adapter read from a line before this pass existed, from a document: every
    /// field through the path helper it used for that field's record kind.
    fn from_document(bytes: &[u8]) -> Option<Line<'static>> {
        let value = parse_record(bytes).ok()?;
        let payload = value.get("payload").unwrap_or(&Value::Null);
        let record_type = match text(&value, &["type"]) {
            Some("session_meta") => RecordType::SessionMeta,
            Some("turn_context") => RecordType::TurnContext,
            Some("token_usage_record") => RecordType::TokenUsageRecord,
            Some("compacted") => RecordType::Compacted,
            Some("event_msg") => RecordType::EventMsg,
            Some(_) | None => RecordType::Other,
        };
        let event_type = match text(payload, &["type"]) {
            Some("thread_settings_applied") => EventType::ThreadSettingsApplied,
            Some("token_count") => EventType::TokenCount,
            Some(_) | None => EventType::Other,
        };
        Some(Line {
            record_type,
            timestamp: owned(&value, "timestamp"),
            ordinal: unsigned(&value, &["ordinal"]),
            payload: Payload {
                event_type,
                id: owned(payload, "id"),
                cli_version: owned(payload, "cli_version"),
                source: owned(payload, "source"),
                thread_source: owned(payload, "thread_source"),
                cwd: owned(payload, "cwd"),
                parent_thread_id: owned(payload, "parent_thread_id"),
                forked_from_id: owned(payload, "forked_from_id"),
                subagent_history_start_ordinal: unsigned(
                    payload,
                    &["subagent_history_start_ordinal"],
                ),
                turn_id: owned(payload, "turn_id"),
                model: owned(payload, "model"),
                effort: owned(payload, "effort"),
                usage_record: usage_fields(payload),
                latest_token_usage_record: value
                    .pointer("/payload/latest_token_usage_record")
                    .map(usage_fields),
                total_token_usage: value.pointer("/payload/info/total_token_usage").map(usage),
                last_token_usage: value.pointer("/payload/info/last_token_usage").map(usage),
                rate_limits: payload.get("rate_limits").and_then(Value::as_object).map(|object| {
                    let sorted: BTreeMap<&String, &Value> = object.iter().collect();
                    let name =
                        |key: &str| object.get(key).and_then(Value::as_str).map(str::to_owned);
                    DecodedLimits {
                        limit_name: name("limit_id").or_else(|| name("limit_name")),
                        windows: ["primary", "secondary"]
                            .into_iter()
                            .filter(|window| {
                                object.get(*window).is_some_and(|value| !value.is_null())
                            })
                            .collect(),
                        native: serde_json::to_string(&sorted).unwrap_or_default(),
                    }
                }),
            },
        })
    }

    fn assert_matches_document(bytes: &[u8]) {
        let expected = from_document(bytes);
        let line = Line::read(bytes).ok();
        assert_eq!(line, expected, "{}", String::from_utf8_lossy(bytes));
    }

    #[test]
    fn the_pass_fails_exactly_when_the_document_does() {
        let deep = |depth: usize| {
            format!(
                r#"{{"type":"event_msg","payload":{{"x":{}{}}}}}"#,
                "[".repeat(depth),
                "]".repeat(depth)
            )
        };
        let mut cases: Vec<Vec<u8>> = [
            r#"{"type":"event_msg","payload":{"type":"token_count"}}"#,
            r#"{"type":"event_msg","payload":{"type":"token_count","x":"\ud800"}}"#,
            r#"{"type":"event_msg","payload":{"type":"token_count","x":"\ud800\udc00"}}"#,
            r#"{"type":"turn_context","payload":{"turn_id":"\x"}}"#,
            r#"{"type":"turn_context","payload":{"n":1e400}}"#,
            r#"{"type":"turn_context","payload":{"n":-1e400}}"#,
            r#"{"type":"turn_context","ordinal":1e-400}"#,
            r#"{"type":"turn_context","ordinal":01}"#,
            r#"{"type":"turn_context","ordinal":1.}"#,
            r#"{"type":"turn_context"} x"#,
            r#"{"type":"turn_context"}   "#,
            r#"{"type":"turn_context",}"#,
            r#"{"type":"turn_context""#,
            r#"{"type":"event_msg","payload":{"type":"token_count","rate_limits":{"a":1e400}}}"#,
            r#"{"type":"event_msg","payload":{"type":"token_count","rate_limits":{"a":"\q"}}}"#,
            r#"{"type":"event_msg","payload":{"info":{"total_token_usage":{"input_tokens":1e400}}}}"#,
            r#"{"ty\u0070e":"s\u0065ssion_meta","payload":{"\u0069d":"a\"b"}}"#,
            r"{1:2}",
            r#"[1,2,{"type":"turn_context"}]"#,
            r#""turn_context""#,
            "3",
            "null",
            "",
            " ",
        ]
        .iter()
        .map(|line| line.as_bytes().to_vec())
        .collect();
        for depth in [124, 125, 126, 127, 300] {
            cases.push(deep(depth).into_bytes());
        }
        cases.push(b"{\"type\":\"turn_context\",\"payload\":{\"model\":\"\xff\"}}".to_vec());
        cases.push(b"{\"type\":\"turn_context\",\"payload\":{\"\xfe\":1}}".to_vec());
        for case in &cases {
            assert_matches_document(case);
        }
    }

    #[test]
    fn fields_read_as_a_document_reads_them() {
        for line in [
            r#"{"timestamp":"2026-09-16T12:00:00Z","type":"session_meta","ordinal":4,"payload":{"id":"t","cli_version":"0.1","source":"cli","thread_source":"user","cwd":"/work/app","parent_thread_id":"p","forked_from_id":"f","subagent_history_start_ordinal":3}}"#,
            r#"{"type":"turn_context","payload":{"turn_id":"t1","model":"m","effort":"high"},"type":"compacted"}"#,
            r#"{"payload":{"turn_id":"t1"},"type":"turn_context","payload":{"model":"m"}}"#,
            r#"{"type":"turn_context","payload":{"turn_id":"t1","turn_id":null}}"#,
            r#"{"type":"turn_context","payload":{"turn_id":null,"turn_id":"t2"}}"#,
            r#"{"type":"turn_context","payload":{"turn_id":7,"model":["m"],"effort":{"e":1}}}"#,
            r#"{"type":"turn_context","payload":"text"}"#,
            r#"{"type":"turn_context","payload":[{"turn_id":"t1"}]}"#,
            r#"{"type":"token_usage_record","payload":{"thread_id":"t","response_id":"r","root_turn_id":"u","usage":{"input_tokens":3,"cached_input_tokens":1,"cache_write_input_tokens":0,"output_tokens":2,"reasoning_output_tokens":1,"total_tokens":5}}}"#,
            r#"{"type":"token_usage_record","payload":{"usage":null}}"#,
            r#"{"type":"token_usage_record","payload":{"usage":5}}"#,
            r#"{"type":"token_usage_record","payload":{"usage":{"input_tokens":3},"usage":{"output_tokens":4}}}"#,
            r#"{"type":"token_usage_record","payload":{"usage":{"input_tokens":3.0,"output_tokens":3.5,"total_tokens":-1,"cached_input_tokens":"3"}}}"#,
            r#"{"type":"token_usage_record","payload":{"usage":{"input_tokens":-0,"output_tokens":-0.0,"total_tokens":18446744073709551615,"reasoning_output_tokens":18446744073709551616}}}"#,
            r#"{"type":"token_usage_record","payload":{"usage":{"input_tokens":9007199254740992.0,"output_tokens":9007199254740994.0}}}"#,
            r#"{"type":"compacted","payload":{"latest_token_usage_record":null}}"#,
            r#"{"type":"compacted","payload":{"latest_token_usage_record":{"response_id":"r","usage":{}},"latest_token_usage_record":{"thread_id":"t"}}}"#,
            r#"{"type":"compacted","payload":{"latest_token_usage_record":[{"response_id":"r"}]}}"#,
            r#"{"type":"compacted","payload":{"response_id":"outer","latest_token_usage_record":{"latest_token_usage_record":{}}}}"#,
            r#"{"type":"event_msg","payload":{"type":"thread_settings_applied","thread_id":"t"}}"#,
            r#"{"type":"event_msg","payload":{"type":"token_count","type":"other"}}"#,
            r#"{"type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":1},"last_token_usage":null}}}"#,
            r#"{"type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":1}},"info":{"last_token_usage":{"output_tokens":2}}}}"#,
            r#"{"type":"event_msg","payload":{"type":"token_count","info":null}}"#,
            r#"{"type":"event_msg","payload":{"type":"token_count","info":[{"total_token_usage":{}}]}}"#,
            r#"{"type":"event_msg","payload":{"type":"token_count","rate_limits":{"limit_id":"codex","primary":{"used_percent":1.5,"resets_at":12},"secondary":null,"b":[1,{"z":null,"a":-0}]}}}"#,
            r#"{"type":"event_msg","payload":{"type":"token_count","rate_limits":{"primary":1},"rate_limits":null}}"#,
            r#"{"type":"event_msg","payload":{"type":"token_count","rate_limits":{"a":1,"a":2}}}"#,
            r#"{"type":"event_msg","payload":{"type":"token_count","rate_limits":"none"}}"#,
            r#"{"type":["event_msg"],"timestamp":12,"ordinal":"3"}"#,
            r#"{"type":"event_msg","timestamp":"a\"b","ordinal":3.0,"ordinal":null}"#,
            r"{}",
        ] {
            assert_matches_document(line.as_bytes());
        }
    }

    #[test]
    fn unescaped_strings_are_borrowed_from_the_line() {
        let line =
            Line::read(br#"{"type":"turn_context","payload":{"turn_id":"t1","model":"a\/b"}}"#)
                .unwrap();
        assert!(matches!(line.payload.turn_id, Some(Cow::Borrowed("t1"))));
        assert!(matches!(line.payload.model, Some(Cow::Owned(ref model)) if model == "a/b"));
    }

    /// A JSON object text from generated entries, which may repeat a key.
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
                Just("\"session_meta\""),
                Just("\"turn_context\""),
                Just("\"token_usage_record\""),
                Just("\"compacted\""),
                Just("\"event_msg\""),
                Just("\"token_count\""),
                Just("\"thread_settings_applied\""),
                Just("\"t\\u006fken_count\""),
                Just("\"t1\""),
                Just("\"/work/app\""),
                Just("\"C:\\\\work\\\\app\""),
                Just("\"2026-09-16T12:00:00.123Z\""),
                Just("2.0"),
                Just("2.5"),
                Just("-0"),
                Just("-1"),
                Just("1e2"),
                Just("18446744073709551616"),
                Just("[]"),
                Just("[1,{\"input_tokens\":2}]"),
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
        prop_oneof![5 => (0_u64..1_000_000).prop_map(|number| number.to_string()), 2 => leaf()]
            .boxed()
    }

    fn usage_object() -> BoxedStrategy<String> {
        let entries = prop_oneof![
            count().prop_map(|value| ("input_tokens", value)),
            count().prop_map(|value| ("cached_input_tokens", value)),
            count().prop_map(|value| ("cache_write_input_tokens", value)),
            count().prop_map(|value| ("output_tokens", value)),
            count().prop_map(|value| ("reasoning_output_tokens", value)),
            count().prop_map(|value| ("total_tokens", value)),
            count().prop_map(|value| ("t\\u006ftal_tokens", value)),
            count().prop_map(|value| ("other_tokens", value)),
        ];
        prop_oneof![1 => leaf(), 6 => object(entries.boxed(), 0..8)].boxed()
    }

    fn usage_record_object() -> BoxedStrategy<String> {
        let entries = prop_oneof![
            1 => leaf().prop_map(|value| ("thread_id", value)),
            1 => leaf().prop_map(|value| ("response_id", value)),
            1 => leaf().prop_map(|value| ("root_turn_id", value)),
            3 => usage_object().prop_map(|value| ("usage", value)),
        ];
        prop_oneof![1 => leaf(), 6 => object(entries.boxed(), 0..5)].boxed()
    }

    fn info_object() -> BoxedStrategy<String> {
        let entries = prop_oneof![
            usage_object().prop_map(|value| ("total_token_usage", value)),
            usage_object().prop_map(|value| ("last_token_usage", value)),
            leaf().prop_map(|value| ("model_context_window", value)),
        ];
        prop_oneof![1 => leaf(), 6 => object(entries.boxed(), 0..4)].boxed()
    }

    fn rate_limits_object() -> BoxedStrategy<String> {
        let window_entries = prop_oneof![
            leaf().prop_map(|value| ("used_percent", value)),
            leaf().prop_map(|value| ("resets_at", value)),
        ];
        let window = prop_oneof![1 => leaf(), 3 => object(window_entries.boxed(), 0..3)];
        let entries = prop_oneof![
            leaf().prop_map(|value| ("limit_id", value)),
            leaf().prop_map(|value| ("limit_name", value)),
            window.clone().prop_map(|value| ("primary", value)),
            window.prop_map(|value| ("secondary", value)),
            leaf().prop_map(|value| ("credits", value)),
        ];
        prop_oneof![1 => leaf(), 6 => object(entries.boxed(), 0..5)].boxed()
    }

    fn payload_object() -> BoxedStrategy<String> {
        let text_key = prop_oneof![
            Just("id"),
            Just("cli_version"),
            Just("source"),
            Just("thread_source"),
            Just("cwd"),
            Just("parent_thread_id"),
            Just("forked_from_id"),
            Just("turn_id"),
            Just("model"),
            Just("effort"),
            Just("thread_id"),
            Just("response_id"),
            Just("root_turn_id"),
            Just("type"),
            Just("content"),
        ];
        let entries = prop_oneof![
            6 => (text_key, leaf()),
            1 => count().prop_map(|value| ("subagent_history_start_ordinal", value)),
            2 => usage_object().prop_map(|value| ("usage", value)),
            2 => usage_record_object().prop_map(|value| ("latest_token_usage_record", value)),
            2 => info_object().prop_map(|value| ("info", value)),
            2 => rate_limits_object().prop_map(|value| ("rate_limits", value)),
        ];
        prop_oneof![1 => leaf(), 8 => object(entries.boxed(), 0..8)].boxed()
    }

    /// A rollout line over the fields the pass reads, whose keys may repeat.
    fn line() -> BoxedStrategy<String> {
        let entries = prop_oneof![
            3 => leaf().prop_map(|value| ("type", value)),
            1 => leaf().prop_map(|value| ("ty\\u0070e", value)),
            1 => leaf().prop_map(|value| ("timestamp", value)),
            1 => count().prop_map(|value| ("ordinal", value)),
            4 => payload_object().prop_map(|value| ("payload", value)),
            1 => leaf().prop_map(|value| ("content", value)),
        ];
        prop_oneof![1 => leaf(), 20 => object(entries.boxed(), 0..6)].boxed()
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
