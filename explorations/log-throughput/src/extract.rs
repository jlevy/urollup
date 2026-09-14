//! Usage extraction from `claude-project` and `codex-rollout` records.
//!
//! Two independent decoders produce the same per-file observations: a typed borrowed
//! decoder (the realistic adapter shape) and a `serde_json::Value` decoder (a cross-check
//! and a slower baseline). An optional byte prefilter skips lines that cannot carry the
//! fields either decoder uses.

use std::borrow::Cow;
use std::collections::BTreeMap;
use std::sync::LazyLock;

use memchr::memmem::Finder;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_json::value::RawValue;

use crate::manifest::Agent;
use crate::metrics::{fnv64, iso_day};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct ClaudeUsage {
    pub input: u64,
    pub output: u64,
    pub cache_create: u64,
    pub cache_read: u64,
}

impl ClaudeUsage {
    pub fn sum(&self) -> u64 {
        self.input + self.output + self.cache_create + self.cache_read
    }
    pub fn add(&mut self, o: &ClaudeUsage) {
        self.input += o.input;
        self.output += o.output;
        self.cache_create += o.cache_create;
        self.cache_read += o.cache_read;
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct CodexUsage {
    pub input: u64,
    pub cached: u64,
    pub cache_write: u64,
    pub output: u64,
    pub reasoning: u64,
    pub total: u64,
}

impl CodexUsage {
    pub fn add(&mut self, o: &CodexUsage) {
        self.input += o.input;
        self.cached += o.cached;
        self.cache_write += o.cache_write;
        self.output += o.output;
        self.reasoning += o.reasoning;
        self.total += o.total;
    }
}

pub struct ClaudeObs {
    /// `message.id` and `requestId` joined by NUL; the dedupe key.
    pub key: Box<str>,
    pub has_request_id: bool,
    pub usage: ClaudeUsage,
    pub day: u32,
}

pub struct CodexRecordObs {
    pub response_id: Box<str>,
    pub usage: CodexUsage,
    pub day: u32,
}

/// Per-file walk of Codex cumulative `token_count` snapshots.
#[derive(Default, Serialize)]
pub struct TokenCountAcc {
    pub events: u64,
    pub null_info: u64,
    /// Snapshots whose cumulative total equals the previous one (skipped).
    pub identical: u64,
    /// Cumulative total decreased: a new counter epoch.
    pub resets: u64,
    /// Cumulative delta disagreed with `last_token_usage.total_tokens`.
    pub delta_mismatch: u64,
    /// Sum of `last_token_usage` over non-identical snapshots.
    pub sum_last: CodexUsage,
    /// Sum over epochs of each epoch's final cumulative `total_tokens`.
    pub epoch_final_sum: u64,
    #[serde(skip)]
    prev_total: Option<CodexUsage>,
    #[serde(skip)]
    epoch_max: u64,
}

impl TokenCountAcc {
    /// Returns false when the snapshot repeats the previous one and was skipped.
    fn observe(&mut self, total: CodexUsage, last: CodexUsage, day: u32, days: &mut BTreeMap<u32, CodexUsage>) -> bool {
        match self.prev_total {
            Some(prev) if prev == total => {
                self.identical += 1;
                return false;
            }
            Some(prev) if total.total < prev.total => {
                self.resets += 1;
                self.epoch_final_sum += self.epoch_max;
                self.epoch_max = 0;
            }
            Some(prev) => {
                if total.total - prev.total != last.total {
                    self.delta_mismatch += 1;
                }
            }
            None => {}
        }
        self.prev_total = Some(total);
        self.epoch_max = self.epoch_max.max(total.total);
        self.sum_last.add(&last);
        days.entry(day).or_default().add(&last);
        true
    }

    fn finish(&mut self) {
        self.epoch_final_sum += self.epoch_max;
        self.epoch_max = 0;
    }
}

#[derive(Default)]
pub struct FileAcc {
    pub lines: u64,
    pub parsed: u64,
    pub parse_errors: u64,
    pub shape_errors: u64,
    pub claude: Vec<ClaudeObs>,
    pub claude_no_message_id: u64,
    pub claude_no_id_usage: ClaudeUsage,
    pub codex_records: Vec<CodexRecordObs>,
    pub tc: TokenCountAcc,
    pub tc_days: BTreeMap<u32, CodexUsage>,
    /// Counted `token_count` snapshots as (timestamp and cumulative-total key, delta),
    /// to detect snapshots replayed verbatim into another rollout.
    pub tc_snapshots: Vec<(u64, u64)>,
    pub limits: u64,
    pub session_meta: u64,
    pub forked: u64,
    pub subagent: u64,
    pub turn_contexts: u64,
}

impl FileAcc {
    pub fn finish(&mut self) {
        self.tc.finish();
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Decoder {
    Typed,
    Value,
}

static F_USAGE: LazyLock<Finder<'static>> = LazyLock::new(|| Finder::new(b"\"usage\""));
static F_QUOTA: LazyLock<Finder<'static>> = LazyLock::new(|| Finder::new(b"\"quotaLimits\""));
static F_TOKEN: LazyLock<Finder<'static>> = LazyLock::new(|| Finder::new(b"\"token_"));
static F_META: LazyLock<Finder<'static>> = LazyLock::new(|| Finder::new(b"\"session_meta\""));
static F_TURN: LazyLock<Finder<'static>> = LazyLock::new(|| Finder::new(b"\"turn_context\""));

/// True when a line may carry a field the extractors use. False positives only cost a
/// parse; a false negative would silently drop usage, so markers are key or type names
/// that every relevant record must contain verbatim.
pub fn prefilter(agent: Agent, line: &[u8]) -> bool {
    match agent {
        Agent::Claude => F_USAGE.find(line).is_some() || F_QUOTA.find(line).is_some(),
        Agent::Codex => {
            F_TOKEN.find(line).is_some() || F_META.find(line).is_some() || F_TURN.find(line).is_some()
        }
    }
}

pub fn process_line(agent: Agent, decoder: Decoder, use_prefilter: bool, line: &[u8], acc: &mut FileAcc) {
    acc.lines += 1;
    if line.iter().all(u8::is_ascii_whitespace) {
        return;
    }
    if use_prefilter && !prefilter(agent, line) {
        return;
    }
    acc.parsed += 1;
    match (agent, decoder) {
        (Agent::Claude, Decoder::Typed) => claude_typed(line, acc),
        (Agent::Claude, Decoder::Value) => claude_value(line, acc),
        (Agent::Codex, Decoder::Typed) => codex_typed(line, acc),
        (Agent::Codex, Decoder::Value) => codex_value(line, acc),
    }
}

// ---------- shared helpers ----------

fn snapshot_key(timestamp: &[u8], total: &CodexUsage) -> u64 {
    let mut bytes = timestamp.to_vec();
    for n in [total.input, total.cached, total.cache_write, total.output, total.reasoning, total.total] {
        bytes.extend_from_slice(&n.to_le_bytes());
    }
    fnv64(&bytes)
}

fn observe_snapshot(acc: &mut FileAcc, timestamp: &[u8], total: CodexUsage, last: CodexUsage, day: u32) {
    if acc.tc.observe(total, last, day, &mut acc.tc_days) {
        acc.tc_snapshots.push((snapshot_key(timestamp, &total), last.total));
    }
}

fn raw_is_null(r: &RawValue) -> bool {
    r.get() == "null"
}

fn raw_string(r: &RawValue) -> Option<Cow<'_, str>> {
    serde_json::from_str::<Cow<'_, str>>(r.get()).ok()
}

/// Day from a raw JSON string value such as `"2026-09-14T..."`.
fn raw_day(r: Option<&RawValue>) -> u32 {
    r.map(|r| r.get().as_bytes()).filter(|b| b.first() == Some(&b'"')).map(|b| iso_day(&b[1..])).unwrap_or(0)
}

fn value_day(v: &Value) -> u32 {
    v.get("timestamp").and_then(Value::as_str).map(|s| iso_day(s.as_bytes())).unwrap_or(0)
}

fn push_claude(acc: &mut FileAcc, message_id: Option<&str>, request_id: Option<&str>, usage: ClaudeUsage, day: u32) {
    match message_id {
        Some(mid) => {
            let rid = request_id.unwrap_or("");
            let mut key = String::with_capacity(mid.len() + 1 + rid.len());
            key.push_str(mid);
            key.push('\0');
            key.push_str(rid);
            acc.claude.push(ClaudeObs {
                key: key.into_boxed_str(),
                has_request_id: request_id.is_some(),
                usage,
                day,
            });
        }
        None => {
            acc.claude_no_message_id += 1;
            acc.claude_no_id_usage.add(&usage);
        }
    }
}

// ---------- claude-project: typed ----------

#[derive(Deserialize)]
struct ClaudeLine<'a> {
    #[serde(rename = "type", borrow, default)]
    ty: Option<&'a RawValue>,
    #[serde(borrow, default)]
    timestamp: Option<&'a RawValue>,
    #[serde(rename = "requestId", borrow, default)]
    request_id: Option<&'a RawValue>,
    #[serde(borrow, default)]
    message: Option<&'a RawValue>,
    #[serde(rename = "quotaLimits", borrow, default)]
    quota_limits: Option<&'a RawValue>,
}

#[derive(Deserialize)]
struct ClaudeMessage<'a> {
    #[serde(borrow, default)]
    id: Option<&'a RawValue>,
    #[serde(default)]
    usage: Option<ClaudeUsageJson>,
}

#[derive(Deserialize)]
struct ClaudeUsageJson {
    #[serde(default)]
    input_tokens: Option<u64>,
    #[serde(default)]
    output_tokens: Option<u64>,
    #[serde(default)]
    cache_creation_input_tokens: Option<u64>,
    #[serde(default)]
    cache_read_input_tokens: Option<u64>,
}

fn claude_typed(line: &[u8], acc: &mut FileAcc) {
    let rec: ClaudeLine<'_> = match serde_json::from_slice(line) {
        Ok(r) => r,
        Err(e) => {
            if e.is_data() { acc.shape_errors += 1 } else { acc.parse_errors += 1 }
            return;
        }
    };
    if rec.quota_limits.is_some_and(|q| !raw_is_null(q)) {
        acc.limits += 1;
    }
    if rec.ty.map(RawValue::get) != Some("\"assistant\"") {
        return;
    }
    let Some(msg) = rec.message else { return };
    let msg: ClaudeMessage<'_> = match serde_json::from_str(msg.get()) {
        Ok(m) => m,
        Err(_) => {
            acc.shape_errors += 1;
            return;
        }
    };
    let Some(u) = msg.usage else { return };
    let usage = ClaudeUsage {
        input: u.input_tokens.unwrap_or(0),
        output: u.output_tokens.unwrap_or(0),
        cache_create: u.cache_creation_input_tokens.unwrap_or(0),
        cache_read: u.cache_read_input_tokens.unwrap_or(0),
    };
    let mid = msg.id.and_then(raw_string);
    let rid = rec.request_id.and_then(raw_string);
    push_claude(acc, mid.as_deref(), rid.as_deref(), usage, raw_day(rec.timestamp));
}

// ---------- claude-project: Value ----------

fn claude_value(line: &[u8], acc: &mut FileAcc) {
    let v: Value = match serde_json::from_slice(line) {
        Ok(v) => v,
        Err(_) => {
            acc.parse_errors += 1;
            return;
        }
    };
    if v.get("quotaLimits").is_some_and(|q| !q.is_null()) {
        acc.limits += 1;
    }
    if v.get("type").and_then(Value::as_str) != Some("assistant") {
        return;
    }
    let Some(u) = v.get("message").and_then(|m| m.get("usage")).filter(|u| u.is_object()) else {
        return;
    };
    let n = |k: &str| u.get(k).and_then(Value::as_u64).unwrap_or(0);
    let usage = ClaudeUsage {
        input: n("input_tokens"),
        output: n("output_tokens"),
        cache_create: n("cache_creation_input_tokens"),
        cache_read: n("cache_read_input_tokens"),
    };
    let mid = v.get("message").and_then(|m| m.get("id")).and_then(Value::as_str);
    let rid = v.get("requestId").and_then(Value::as_str);
    push_claude(acc, mid, rid, usage, value_day(&v));
}

// ---------- codex-rollout: typed ----------

#[derive(Deserialize)]
struct CodexLine<'a> {
    #[serde(rename = "type", borrow, default)]
    ty: Option<&'a RawValue>,
    #[serde(borrow, default)]
    timestamp: Option<&'a RawValue>,
    #[serde(borrow, default)]
    payload: Option<CodexPayload<'a>>,
}

/// Union of the payload fields used across record types, all left raw so one pass
/// never fails on another record type's field shapes.
#[derive(Deserialize)]
struct CodexPayload<'a> {
    #[serde(rename = "type", borrow, default)]
    ty: Option<&'a RawValue>,
    #[serde(borrow, default)]
    info: Option<&'a RawValue>,
    #[serde(borrow, default)]
    rate_limits: Option<&'a RawValue>,
    #[serde(borrow, default)]
    response_id: Option<&'a RawValue>,
    #[serde(borrow, default)]
    usage: Option<&'a RawValue>,
    #[serde(borrow, default)]
    forked_from_id: Option<&'a RawValue>,
    #[serde(borrow, default)]
    source: Option<&'a RawValue>,
}

#[derive(Deserialize)]
struct CodexInfo {
    #[serde(default)]
    total_token_usage: Option<CodexUsageJson>,
    #[serde(default)]
    last_token_usage: Option<CodexUsageJson>,
}

#[derive(Deserialize)]
struct CodexUsageJson {
    #[serde(default)]
    input_tokens: Option<u64>,
    #[serde(default)]
    cached_input_tokens: Option<u64>,
    #[serde(default)]
    cache_write_input_tokens: Option<u64>,
    #[serde(default)]
    output_tokens: Option<u64>,
    #[serde(default)]
    reasoning_output_tokens: Option<u64>,
    #[serde(default)]
    total_tokens: Option<u64>,
}

impl From<CodexUsageJson> for CodexUsage {
    fn from(u: CodexUsageJson) -> Self {
        CodexUsage {
            input: u.input_tokens.unwrap_or(0),
            cached: u.cached_input_tokens.unwrap_or(0),
            cache_write: u.cache_write_input_tokens.unwrap_or(0),
            output: u.output_tokens.unwrap_or(0),
            reasoning: u.reasoning_output_tokens.unwrap_or(0),
            total: u.total_tokens.unwrap_or(0),
        }
    }
}

fn source_is_subagent(raw: &str) -> bool {
    serde_json::from_str::<Value>(raw).ok().is_some_and(|v| v.get("subagent").is_some())
}

fn codex_typed(line: &[u8], acc: &mut FileAcc) {
    let rec: CodexLine<'_> = match serde_json::from_slice(line) {
        Ok(r) => r,
        Err(e) => {
            if e.is_data() { acc.shape_errors += 1 } else { acc.parse_errors += 1 }
            return;
        }
    };
    let (Some(ty), Some(p)) = (rec.ty, rec.payload) else { return };
    let day = raw_day(rec.timestamp);
    match ty.get() {
        "\"event_msg\"" if p.ty.map(RawValue::get) == Some("\"token_count\"") => {
            if p.rate_limits.is_some_and(|r| !raw_is_null(r)) {
                acc.limits += 1;
            }
            acc.tc.events += 1;
            let info = p.info.filter(|i| !raw_is_null(i)).map(|i| serde_json::from_str::<CodexInfo>(i.get()));
            match info {
                None => acc.tc.null_info += 1,
                Some(Err(_)) => acc.shape_errors += 1,
                Some(Ok(info)) => match (info.total_token_usage, info.last_token_usage) {
                    (Some(t), Some(l)) => {
                        let ts = rec.timestamp.map(|r| r.get().trim_matches('"').as_bytes()).unwrap_or(b"");
                        observe_snapshot(acc, ts, t.into(), l.into(), day)
                    }
                    _ => acc.tc.null_info += 1,
                },
            }
        }
        "\"token_usage_record\"" => {
            let rid = p.response_id.and_then(raw_string);
            let usage = p.usage.filter(|u| !raw_is_null(u)).map(|u| serde_json::from_str::<CodexUsageJson>(u.get()));
            match (rid, usage) {
                (Some(rid), Some(Ok(u))) => acc.codex_records.push(CodexRecordObs {
                    response_id: rid.into_owned().into_boxed_str(),
                    usage: u.into(),
                    day,
                }),
                (_, Some(Err(_))) => acc.shape_errors += 1,
                _ => acc.shape_errors += 1,
            }
        }
        "\"session_meta\"" => {
            acc.session_meta += 1;
            if p.forked_from_id.is_some_and(|f| !raw_is_null(f)) {
                acc.forked += 1;
            }
            if p.source.is_some_and(|s| source_is_subagent(s.get())) {
                acc.subagent += 1;
            }
        }
        "\"turn_context\"" => acc.turn_contexts += 1,
        _ => {}
    }
}

// ---------- codex-rollout: Value ----------

fn codex_usage_value(u: &Value) -> CodexUsage {
    let n = |k: &str| u.get(k).and_then(Value::as_u64).unwrap_or(0);
    CodexUsage {
        input: n("input_tokens"),
        cached: n("cached_input_tokens"),
        cache_write: n("cache_write_input_tokens"),
        output: n("output_tokens"),
        reasoning: n("reasoning_output_tokens"),
        total: n("total_tokens"),
    }
}

fn codex_value(line: &[u8], acc: &mut FileAcc) {
    let v: Value = match serde_json::from_slice(line) {
        Ok(v) => v,
        Err(_) => {
            acc.parse_errors += 1;
            return;
        }
    };
    let (Some(ty), Some(p)) = (v.get("type").and_then(Value::as_str), v.get("payload")) else { return };
    if !p.is_object() {
        return;
    }
    let day = value_day(&v);
    let nonnull = |k: &str| p.get(k).is_some_and(|x| !x.is_null());
    match ty {
        "event_msg" if p.get("type").and_then(Value::as_str) == Some("token_count") => {
            if nonnull("rate_limits") {
                acc.limits += 1;
            }
            acc.tc.events += 1;
            let info = p.get("info").filter(|i| !i.is_null());
            match info.map(|i| (i.get("total_token_usage"), i.get("last_token_usage"))) {
                Some((Some(t), Some(l))) if t.is_object() && l.is_object() => {
                    let ts = v.get("timestamp").and_then(Value::as_str).unwrap_or("").as_bytes();
                    observe_snapshot(acc, ts, codex_usage_value(t), codex_usage_value(l), day)
                }
                _ => acc.tc.null_info += 1,
            }
        }
        "token_usage_record" => match (p.get("response_id").and_then(Value::as_str), p.get("usage")) {
            (Some(rid), Some(u)) if u.is_object() => acc.codex_records.push(CodexRecordObs {
                response_id: rid.into(),
                usage: codex_usage_value(u),
                day,
            }),
            _ => acc.shape_errors += 1,
        },
        "session_meta" => {
            acc.session_meta += 1;
            if nonnull("forked_from_id") {
                acc.forked += 1;
            }
            if p.get("source").is_some_and(|s| s.get("subagent").is_some()) {
                acc.subagent += 1;
            }
        }
        "turn_context" => acc.turn_contexts += 1,
        _ => {}
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub const CLAUDE: &[&str] = &[
        r#"{"type":"user","timestamp":"2026-09-01T00:00:00Z","message":{"role":"user","content":"prompt mentioning \"usage\" in text"}}"#,
        r#"{"type":"assistant","timestamp":"2026-09-01T00:00:01Z","requestId":"req_1","message":{"id":"msg_1","model":"m","content":[{"type":"text","text":"a"}],"usage":{"input_tokens":10,"output_tokens":5,"cache_creation_input_tokens":100,"cache_read_input_tokens":1000}}}"#,
        r#"{"type":"assistant","timestamp":"2026-09-01T00:00:01Z","requestId":"req_1","message":{"id":"msg_1","model":"m","content":[{"type":"tool_use","id":"t","name":"Bash","input":{}}],"usage":{"input_tokens":10,"output_tokens":5,"cache_creation_input_tokens":100,"cache_read_input_tokens":1000}}}"#,
        r#"{"type":"assistant","timestamp":"2026-09-02T00:00:01Z","message":{"id":"msg_2","usage":{"input_tokens":1,"output_tokens":2}},"quotaLimits":{"status":"allowed"}}"#,
        r#"{"type":"system","subtype":"x","content":"no usage here","message":"a string, not an object"}"#,
        r#"{"type":"assistant","message":{"id":"msg_3","usage":null}}"#,
        r#"not json"#,
    ];

    pub const CODEX: &[&str] = &[
        r#"{"timestamp":"2026-09-01T00:00:00Z","type":"session_meta","payload":{"id":"t1","forked_from_id":"t0","source":{"subagent":{"thread_spawn":{}}}}}"#,
        r#"{"timestamp":"2026-09-01T00:00:00Z","type":"turn_context","payload":{"model":"m","effort":"high"}}"#,
        r#"{"timestamp":"2026-09-01T00:00:01Z","type":"event_msg","payload":{"type":"token_count","info":null,"rate_limits":{"primary":{"used_percent":1.0}}}}"#,
        r#"{"timestamp":"2026-09-01T00:00:02Z","type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":100,"output_tokens":10,"total_tokens":110},"last_token_usage":{"input_tokens":100,"output_tokens":10,"total_tokens":110}}}}"#,
        r#"{"timestamp":"2026-09-01T00:00:03Z","type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":100,"output_tokens":10,"total_tokens":110},"last_token_usage":{"input_tokens":100,"output_tokens":10,"total_tokens":110}}}}"#,
        r#"{"timestamp":"2026-09-01T00:00:04Z","type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":300,"output_tokens":30,"total_tokens":330},"last_token_usage":{"input_tokens":200,"output_tokens":20,"total_tokens":220}}}}"#,
        r#"{"timestamp":"2026-09-02T00:00:05Z","type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":50,"output_tokens":5,"total_tokens":55},"last_token_usage":{"input_tokens":50,"output_tokens":5,"total_tokens":55}}}}"#,
        r#"{"timestamp":"2026-09-02T00:00:06Z","type":"token_usage_record","payload":{"response_id":"r1","usage":{"input_tokens":7,"total_tokens":8}}}"#,
        r#"{"timestamp":"2026-09-02T00:00:07Z","type":"event_msg","payload":{"type":"agent_message","message":"talks about token_count"}}"#,
        r#"{"timestamp":"2026-09-02T00:00:08Z","type":"response_item","payload":"unexpected string payload"}"#,
    ];

    pub fn run(agent: Agent, lines: &[&str], decoder: Decoder, pre: bool) -> FileAcc {
        let mut acc = FileAcc::default();
        for l in lines {
            process_line(agent, decoder, pre, l.as_bytes(), &mut acc);
        }
        acc.finish();
        acc
    }

    fn claude_summary(a: &FileAcc) -> Vec<(String, bool, ClaudeUsage, u32)> {
        a.claude.iter().map(|o| (o.key.to_string(), o.has_request_id, o.usage, o.day)).collect()
    }

    #[test]
    fn claude_decoders_and_prefilter_agree() {
        let typed = run(Agent::Claude, CLAUDE, Decoder::Typed, false);
        assert_eq!(typed.claude.len(), 3);
        assert_eq!(typed.claude[0].usage.sum(), 1115);
        assert!(!typed.claude[2].has_request_id);
        assert_eq!(typed.limits, 1);
        assert_eq!(typed.parse_errors, 1);
        for (dec, pre) in [(Decoder::Value, false), (Decoder::Typed, true), (Decoder::Value, true)] {
            let other = run(Agent::Claude, CLAUDE, dec, pre);
            assert_eq!(claude_summary(&typed), claude_summary(&other), "{dec:?} prefilter={pre}");
            assert_eq!(typed.limits, other.limits);
        }
    }

    #[test]
    fn codex_token_count_epochs_and_records() {
        let typed = run(Agent::Codex, CODEX, Decoder::Typed, false);
        assert_eq!(typed.tc.events, 5);
        assert_eq!(typed.tc.null_info, 1);
        assert_eq!(typed.tc.identical, 1);
        assert_eq!(typed.tc.resets, 1);
        assert_eq!(typed.tc.delta_mismatch, 0);
        assert_eq!(typed.tc.sum_last.total, 110 + 220 + 55);
        assert_eq!(typed.tc.epoch_final_sum, 330 + 55);
        assert_eq!((typed.session_meta, typed.forked, typed.subagent, typed.turn_contexts), (1, 1, 1, 1));
        assert_eq!(typed.codex_records.len(), 1);
        assert_eq!(typed.limits, 1);
        for (dec, pre) in [(Decoder::Value, false), (Decoder::Typed, true), (Decoder::Value, true)] {
            let o = run(Agent::Codex, CODEX, dec, pre);
            assert_eq!(serde_json::to_string(&typed.tc).unwrap(), serde_json::to_string(&o.tc).unwrap());
            assert_eq!(typed.tc_days, o.tc_days);
            assert_eq!(typed.codex_records.len(), o.codex_records.len());
            assert_eq!((o.session_meta, o.forked, o.subagent, o.limits), (1, 1, 1, 1));
        }
    }
}
