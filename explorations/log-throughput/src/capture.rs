//! Survey composition and write content-stripped, zstd-compressed captured records.
//!
//! Each source file becomes `<capture-dir>/<path hash>.jsonl.zst`, holding the
//! records the capture policy keeps with content replaced by `{"$b": bytes, "$d": digest}`
//! stubs. File names carry no source path. The digest here is an unkeyed, deterministic
//! stand-in for the architecture doc's keyed HMAC-SHA-256: `DefaultHasher::new()` uses
//! fixed SipHash keys, so identical content gets the same digest on every machine. It
//! matches only the 64-hex length and incompressibility, and is not cryptographic.

use std::collections::hash_map::DefaultHasher;
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::hash::Hasher;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

use rayon::prelude::*;
use serde_json::{Map, Value, json};

use crate::Result;
use crate::args::Args;
use crate::lines::{for_each_line, open_entry};
use crate::manifest::{Agent, Entry, Kind, Manifest};
use crate::metrics::{MIB, iso_day, rusage};

/// Record classes, from most to least relevant to usage accounting.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Class {
    /// Carries token usage or provider limits.
    Usage,
    /// Identity, model, timing, linkage or tool-call structure.
    Structure,
    /// Conversation records whose value is mostly content (kept stripped).
    Conversation,
    /// Display-only or bookkeeping records (dropped by the capture policy).
    Display,
}

pub const CLASSES: [Class; 4] =
    [Class::Usage, Class::Structure, Class::Conversation, Class::Display];

impl Class {
    fn name(self) -> &'static str {
        match self {
            Class::Usage => "usage",
            Class::Structure => "structure",
            Class::Conversation => "conversation",
            Class::Display => "display",
        }
    }
    fn index(self) -> usize {
        self as usize
    }
    pub fn kept(self) -> bool {
        self != Class::Display
    }
}

fn str_at<'a>(v: &'a Value, keys: &[&str]) -> &'a str {
    let mut cur = v;
    for k in keys {
        match cur.get(*k) {
            Some(next) => cur = next,
            None => return "",
        }
    }
    cur.as_str().unwrap_or("")
}

/// Sanitized record type label, safe to print in aggregates.
fn type_label(agent: Agent, v: &Value) -> String {
    let clean = |s: &str| -> String {
        if !s.is_empty()
            && s.len() <= 48
            && s.bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'-')
        {
            s.to_string()
        } else if s.is_empty() {
            "-".into()
        } else {
            "?".into()
        }
    };
    match agent {
        Agent::Claude => {
            let ty = str_at(v, &["type"]);
            let sub = str_at(v, &["subtype"]);
            if sub.is_empty() { clean(ty) } else { format!("{}/{}", clean(ty), clean(sub)) }
        }
        Agent::Codex => {
            let ty = str_at(v, &["type"]);
            let pty = str_at(v, &["payload", "type"]);
            if v.get("type").is_none() && v.get("record_type").is_some() {
                return "legacy-record".into();
            }
            if pty.is_empty() { clean(ty) } else { format!("{}/{}", clean(ty), clean(pty)) }
        }
    }
}

pub fn classify(agent: Agent, v: &Value) -> Class {
    match agent {
        Agent::Claude => match str_at(v, &["type"]) {
            "assistant" => {
                let has_usage =
                    v.get("message").and_then(|m| m.get("usage")).is_some_and(Value::is_object);
                let has_limits = v.get("quotaLimits").is_some_and(|q| !q.is_null());
                if has_usage || has_limits { Class::Usage } else { Class::Structure }
            }
            "user" | "system" | "summary" => Class::Structure,
            "attachment" => Class::Conversation,
            _ => Class::Display,
        },
        Agent::Codex => match (str_at(v, &["type"]), str_at(v, &["payload", "type"])) {
            ("token_usage_record", _) | ("event_msg", "token_count") => Class::Usage,
            ("session_meta", _) | ("turn_context", _) | ("compacted", _) => Class::Structure,
            ("inter_agent_communication_metadata", _) => Class::Structure,
            (
                "event_msg",
                "task_started"
                | "task_complete"
                | "turn_aborted"
                | "thread_settings_applied"
                | "context_compacted"
                | "thread_rolled_back"
                | "sub_agent_activity"
                | "entered_review_mode"
                | "exited_review_mode"
                | "error"
                | "stream_error",
            ) => Class::Structure,
            ("response_item", "message" | "reasoning" | "agent_message") => Class::Conversation,
            ("response_item", _) => Class::Structure,
            _ => Class::Display,
        },
    }
}

const CLAUDE_CONTENT: &[&str] = &[
    "content",
    "text",
    "thinking",
    "signature",
    "input",
    "wireToolInputs",
    "attachment",
    "snapshot",
    "trackedFileBackups",
    "originalFile",
    "oldString",
    "newString",
    "stdout",
    "stderr",
    "prompt",
    "lastPrompt",
    "summary",
    "data",
    "normalizedMessages",
    "structuredPatch",
    "result",
    "output",
    "query",
    "results",
    "questions",
    "answers",
    "todos",
    "oldTodos",
    "newTodos",
    "filenames",
    "matches",
    "plan",
];

const CODEX_CONTENT: &[&str] = &[
    "content",
    "text",
    "summary",
    "encrypted_content",
    "arguments",
    "input",
    "output",
    "message",
    "replacement_history",
    "base_instructions",
    "instructions",
    "user_instructions",
    "developer_instructions",
    "last_agent_message",
    "patch",
    "stdout",
    "stderr",
    "aggregated_output",
    "formatted_output",
    "changes",
    "unified_diff",
    "action",
    "results",
    "query",
];

/// Keys whose values are never stubbed as a whole (they are recursed instead).
const NEVER_STUB: &[&str] =
    &["usage", "info", "rate_limits", "message_usage", "iterations", "payload"];

const SHORT: usize = 16;
const LONG: usize = 256;

/// Hex digits kept in each stub digest (64 matches a full HMAC-SHA-256).
static DIGEST_HEX: AtomicUsize = AtomicUsize::new(64);

#[derive(Default, Clone, Copy)]
pub struct StripStats {
    pub stubs: u64,
    pub stubbed_bytes: u64,
}

struct HashCount {
    hasher: DefaultHasher,
    bytes: u64,
}

impl Write for HashCount {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.hasher.write(buf);
        self.bytes += buf.len() as u64;
        Ok(buf.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn stub_for(value: &Value) -> (Value, u64) {
    let mut hc = HashCount { hasher: DefaultHasher::new(), bytes: 0 };
    match value {
        Value::String(s) => {
            hc.write_all(s.as_bytes()).ok();
        }
        other => {
            serde_json::to_writer(&mut hc, other).ok();
        }
    }
    // Expand the unkeyed 64-bit hash to 256 pseudo-random-looking bits (splitmix64),
    // matching the size of a hex HMAC-SHA-256 digest; it holds only 64 bits of entropy.
    let mut x = hc.hasher.finish();
    let mut hex = String::with_capacity(64);
    for _ in 0..4 {
        x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = x;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        hex.push_str(&format!("{z:016x}"));
    }
    hex.truncate(DIGEST_HEX.load(Ordering::Relaxed));
    (json!({"$b": hc.bytes, "$d": hex}), hc.bytes)
}

fn is_long_string_array(a: &[Value]) -> bool {
    !a.is_empty()
        && a.iter().all(Value::is_string)
        && a.iter().map(|s| s.as_str().map_or(0, str::len)).sum::<usize>() > LONG
}

fn strip_value(v: &mut Value, content_keys: &[&str], stats: &mut StripStats) {
    match v {
        Value::Object(map) => {
            for (k, child) in map.iter_mut() {
                let stub = if NEVER_STUB.contains(&k.as_str()) {
                    false
                } else if content_keys.contains(&k.as_str()) {
                    // Arrays of objects (content blocks) are recursed so block types, tool
                    // names and tool-use IDs stay verbatim; other content is stubbed.
                    match child {
                        Value::String(s) => s.len() > SHORT,
                        Value::Array(a) => !a.is_empty() && !a.iter().all(Value::is_object),
                        Value::Object(o) => !o.is_empty(),
                        _ => false,
                    }
                } else {
                    match child {
                        Value::String(s) => s.len() > LONG,
                        Value::Array(a) => is_long_string_array(a),
                        _ => false,
                    }
                };
                if stub {
                    let (s, bytes) = stub_for(child);
                    stats.stubs += 1;
                    stats.stubbed_bytes += bytes;
                    *child = s;
                } else {
                    strip_value(child, content_keys, stats);
                }
            }
        }
        Value::Array(items) => {
            for item in items {
                let stub = match item {
                    Value::String(s) => s.len() > LONG,
                    _ => false,
                };
                if stub {
                    let (s, bytes) = stub_for(item);
                    stats.stubs += 1;
                    stats.stubbed_bytes += bytes;
                    *item = s;
                } else {
                    strip_value(item, content_keys, stats);
                }
            }
        }
        _ => {}
    }
}

pub fn strip(agent: Agent, v: &mut Value, stats: &mut StripStats) {
    let keys = match agent {
        Agent::Claude => CLAUDE_CONTENT,
        Agent::Codex => CODEX_CONTENT,
    };
    strip_value(v, keys, stats);
}

struct CountWriter(u64);

impl Write for CountWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0 += buf.len() as u64;
        Ok(buf.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[derive(Default, Clone, Copy)]
struct ClassAcc {
    records: u64,
    bytes: u64,
    stripped_bytes: u64,
    window_records: u64,
    window_bytes: u64,
}

#[derive(Default)]
struct FileSurvey {
    agent: Option<Agent>,
    in_mtime_window: bool,
    logical_bytes: u64,
    lines: u64,
    pending_bytes: u64,
    max_line: u64,
    parse_errors: u64,
    io_errors: u64,
    no_timestamp: u64,
    classes: [ClassAcc; 4],
    types: HashMap<String, (u64, u64)>,
    days: BTreeMap<u32, (u64, u64)>,
    strip: StripStats,
    captured_records: u64,
    captured_raw: u64,
    captured_zst3: u64,
    captured_zst19: u64,
    usage_only_zst3: u64,
    window_captured_raw: u64,
}

fn survey_file(
    entry: &Entry,
    out_dir: &Path,
    level19: bool,
    cutoff_day: u32,
    cutoff: i64,
) -> Result<FileSurvey> {
    let mut fs_acc = FileSurvey {
        agent: Some(entry.agent),
        in_mtime_window: entry.mtime >= cutoff,
        ..Default::default()
    };
    let out_path = out_dir.join(entry.capture_name());
    let file = fs::File::create(&out_path)?;
    let mut enc3 = zstd::stream::write::Encoder::new(io::BufWriter::new(file), 3)?;
    let mut enc19 =
        if level19 { Some(zstd::stream::write::Encoder::new(CountWriter(0), 19)?) } else { None };
    let mut enc_usage = zstd::stream::write::Encoder::new(CountWriter(0), 3)?;
    let mut buf = Vec::with_capacity(4096);
    let mut write_err: Option<io::Error> = None;

    let reader = match open_entry(entry) {
        Ok(r) => r,
        Err(_) => {
            fs_acc.io_errors += 1;
            return Ok(fs_acc);
        }
    };
    let stats = for_each_line(reader, |line| {
        if line.iter().all(u8::is_ascii_whitespace) {
            return;
        }
        let mut v: Value = match serde_json::from_slice(line) {
            Ok(v) => v,
            Err(_) => {
                fs_acc.parse_errors += 1;
                return;
            }
        };
        let class = classify(entry.agent, &v);
        let label = type_label(entry.agent, &v);
        let bytes = line.len() as u64 + 1;
        let day =
            v.get("timestamp").and_then(Value::as_str).map(|s| iso_day(s.as_bytes())).unwrap_or(0);
        let in_window = if day == 0 {
            fs_acc.no_timestamp += 1;
            fs_acc.in_mtime_window
        } else {
            day >= cutoff_day
        };
        let t = fs_acc.types.entry(label).or_default();
        t.0 += 1;
        t.1 += bytes;
        let d = fs_acc.days.entry(day).or_default();
        d.0 += 1;
        d.1 += bytes;

        strip(entry.agent, &mut v, &mut fs_acc.strip);
        buf.clear();
        if serde_json::to_writer(&mut buf, &v).is_err() {
            return;
        }
        buf.push(b'\n');
        let c = &mut fs_acc.classes[class.index()];
        c.records += 1;
        c.bytes += bytes;
        c.stripped_bytes += buf.len() as u64;
        if in_window {
            c.window_records += 1;
            c.window_bytes += bytes;
        }
        if class.kept() {
            fs_acc.captured_records += 1;
            fs_acc.captured_raw += buf.len() as u64;
            if in_window {
                fs_acc.window_captured_raw += buf.len() as u64;
            }
            let mut res = enc3.write_all(&buf);
            if let Some(e) = enc19.as_mut() {
                res = res.and_then(|_| e.write_all(&buf));
            }
            if class == Class::Usage {
                res = res.and_then(|_| enc_usage.write_all(&buf));
            }
            if let Err(e) = res {
                write_err.get_or_insert(e);
            }
        }
    });
    if let Some(e) = write_err {
        return Err(e.into());
    }
    match stats {
        Ok(s) => {
            fs_acc.logical_bytes = s.bytes;
            fs_acc.lines = s.lines;
            fs_acc.pending_bytes = s.pending_bytes;
            fs_acc.max_line = s.max_line;
        }
        Err(_) => fs_acc.io_errors += 1,
    }
    enc3.finish()?.flush()?;
    fs_acc.captured_zst3 = fs::metadata(&out_path)?.len();
    if let Some(e) = enc19 {
        fs_acc.captured_zst19 = e.finish()?.0;
    }
    fs_acc.usage_only_zst3 = enc_usage.finish()?.0;
    Ok(fs_acc)
}

/// `capture --manifest FILE --out DIR [--threads N] [--level19] [--digest-hex 64]`
pub fn cmd(args: &Args) -> Result<()> {
    let digest_hex: usize = args.value("digest-hex").unwrap_or("64").parse()?;
    DIGEST_HEX.store(digest_hex.clamp(0, 64), Ordering::Relaxed);
    let manifest = Manifest::load(Path::new(args.required("manifest")?))?;
    let out_dir = PathBuf::from(args.required("out")?);
    let threads: usize = args.value("threads").unwrap_or("10").parse()?;
    let level19 = args.flag("level19");
    fs::create_dir_all(&out_dir)?;
    let cutoff_day = manifest.cutoff_day();

    let t0 = Instant::now();
    let pool = rayon::ThreadPoolBuilder::new().num_threads(threads).build()?;
    let entries: Vec<&Entry> =
        manifest.entries.iter().filter(|e| e.kind != Kind::LegacyJson).collect();
    let results: Vec<(String, Result<FileSurvey>)> = pool.install(|| {
        entries
            .par_iter()
            .map(|e| {
                (e.capture_name(), survey_file(e, &out_dir, level19, cutoff_day, manifest.cutoff))
            })
            .collect()
    });
    let wall_s = t0.elapsed().as_secs_f64();
    let ru = rusage();

    let mut index = io::BufWriter::new(fs::File::create(out_dir.join("index.tsv"))?);
    let mut report = Map::new();
    for agent in [Agent::Claude, Agent::Codex] {
        let mut total = FileSurvey::default();
        for (idx, r) in &results {
            let s = match r {
                Ok(s) if s.agent == Some(agent) => s,
                Ok(_) => continue,
                Err(e) => return Err(format!("capture failed for entry {idx}: {e}").into()),
            };
            writeln!(
                index,
                "{idx}\t{}\t{}\t{}\t{}",
                s.logical_bytes, s.lines, s.captured_records, s.captured_zst3
            )?;
            total.logical_bytes += s.logical_bytes;
            total.lines += s.lines;
            total.pending_bytes += s.pending_bytes;
            total.max_line = total.max_line.max(s.max_line);
            total.parse_errors += s.parse_errors;
            total.io_errors += s.io_errors;
            total.no_timestamp += s.no_timestamp;
            for (i, c) in s.classes.iter().enumerate() {
                let t = &mut total.classes[i];
                t.records += c.records;
                t.bytes += c.bytes;
                t.stripped_bytes += c.stripped_bytes;
                t.window_records += c.window_records;
                t.window_bytes += c.window_bytes;
            }
            for (k, (n, b)) in &s.types {
                let t = total.types.entry(k.clone()).or_default();
                t.0 += n;
                t.1 += b;
            }
            for (d, (n, b)) in &s.days {
                let t = total.days.entry(*d).or_default();
                t.0 += n;
                t.1 += b;
            }
            total.strip.stubs += s.strip.stubs;
            total.strip.stubbed_bytes += s.strip.stubbed_bytes;
            total.captured_records += s.captured_records;
            total.captured_raw += s.captured_raw;
            total.captured_zst3 += s.captured_zst3;
            total.captured_zst19 += s.captured_zst19;
            total.usage_only_zst3 += s.usage_only_zst3;
            total.window_captured_raw += s.window_captured_raw;
        }
        let mut types: Vec<_> = total.types.iter().collect();
        types.sort_by_key(|a| std::cmp::Reverse(a.1.1));
        let types: Vec<Value> = types
            .iter()
            .take(40)
            .map(|(k, (n, b))| json!({"type": k, "records": n, "bytes": b}))
            .collect();
        let active_days = total.days.keys().filter(|d| **d != 0).count();
        let window_days = total.days.keys().filter(|d| **d >= cutoff_day).count();
        let window_bytes: u64 = total.classes.iter().map(|c| c.window_bytes).sum();
        let window_records: u64 = total.classes.iter().map(|c| c.window_records).sum();
        let mut classes = Map::new();
        for c in CLASSES {
            let a = total.classes[c.index()];
            classes.insert(
                c.name().into(),
                json!({"records": a.records, "bytes": a.bytes, "stripped_bytes": a.stripped_bytes,
                       "window_records": a.window_records, "window_bytes": a.window_bytes}),
            );
        }
        // Per-month bytes by record timestamp, for growth context.
        let mut months: BTreeMap<u32, (u64, u64)> = BTreeMap::new();
        for (d, (n, b)) in &total.days {
            let m = months.entry(d / 100).or_default();
            m.0 += n;
            m.1 += b;
        }
        report.insert(
            match agent {
                Agent::Claude => "claude",
                Agent::Codex => "codex",
            }
            .into(),
            json!({
                "logical_bytes": total.logical_bytes,
                "lines": total.lines,
                "pending_bytes": total.pending_bytes,
                "max_line_bytes": total.max_line,
                "parse_errors": total.parse_errors,
                "io_errors": total.io_errors,
                "records_without_timestamp": total.no_timestamp,
                "active_days": active_days,
                "bytes_per_active_day": total.logical_bytes as f64 / active_days.max(1) as f64,
                "window": {"records": window_records, "bytes": window_bytes, "active_days": window_days,
                           "bytes_per_active_day": window_bytes as f64 / window_days.max(1) as f64,
                           "captured_raw_bytes": total.window_captured_raw},
                "classes": classes,
                "stubs": total.strip.stubs,
                "stubbed_bytes": total.strip.stubbed_bytes,
                "captured": {"records": total.captured_records, "raw_bytes": total.captured_raw,
                             "zst3_bytes": total.captured_zst3, "zst19_bytes": total.captured_zst19,
                             "usage_only_zst3_bytes": total.usage_only_zst3},
                "months": months.iter().map(|(m, (n, b))| json!({"month": m, "records": n, "bytes": b})).collect::<Vec<_>>(),
                "top_types": types,
            }),
        );
    }
    index.flush()?;
    let out = json!({
        "wall_s": wall_s,
        "cpu_s": ru.user_s + ru.sys_s,
        "max_rss_mib": ru.max_rss_bytes as f64 / MIB,
        "threads": threads,
        "level19": level19,
        "digest_hex": digest_hex,
        "cutoff_day": cutoff_day,
        "agents": report,
    });
    println!("{}", serde_json::to_string_pretty(&out)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extract::tests::{CLAUDE, CODEX, run};
    use crate::extract::{Decoder, FileAcc, process_line};

    fn stripped_lines(agent: Agent, lines: &[&str]) -> Vec<String> {
        let mut stats = StripStats::default();
        lines
            .iter()
            .filter_map(|l| serde_json::from_str::<Value>(l).ok())
            .filter(|v| classify(agent, v).kept())
            .map(|mut v| {
                strip(agent, &mut v, &mut stats);
                serde_json::to_string(&v).unwrap()
            })
            .collect()
    }

    #[test]
    fn stripping_keeps_usage_and_stubs_content() {
        let long = "secret ".repeat(40);
        let line = format!(
            r#"{{"type":"assistant","requestId":"req_9","cwd":"/w","message":{{"id":"msg_9","model":"m","content":[{{"type":"text","text":"{long}"}},{{"type":"tool_use","id":"t1","name":"Bash","input":{{"command":"{long}"}}}}],"usage":{{"input_tokens":3,"output_tokens":4}}}}}}"#
        );
        let mut v: Value = serde_json::from_str(&line).unwrap();
        assert_eq!(classify(Agent::Claude, &v), Class::Usage);
        let mut stats = StripStats::default();
        strip(Agent::Claude, &mut v, &mut stats);
        let out = serde_json::to_string(&v).unwrap();
        assert!(!out.contains("secret"));
        assert_eq!(v["message"]["usage"]["output_tokens"], 4);
        assert_eq!(v["requestId"], "req_9");
        assert_eq!(v["cwd"], "/w");
        assert_eq!(v["message"]["content"][1]["name"], "Bash");
        assert_eq!(v["message"]["content"][0]["text"]["$b"].as_u64().unwrap(), long.len() as u64);
        assert_eq!(stats.stubs, 2);
    }

    #[test]
    fn captured_records_reproduce_source_totals() {
        for (agent, lines) in [(Agent::Claude, CLAUDE), (Agent::Codex, CODEX)] {
            let source = run(agent, lines, Decoder::Typed, false);
            let captured = stripped_lines(agent, lines);
            let mut acc = FileAcc::default();
            for l in &captured {
                process_line(agent, Decoder::Typed, false, l.as_bytes(), &mut acc);
            }
            acc.finish();
            let keys = |a: &FileAcc| {
                a.claude.iter().map(|o| (o.key.to_string(), o.usage)).collect::<Vec<_>>()
            };
            assert_eq!(keys(&source), keys(&acc));
            assert_eq!(
                serde_json::to_string(&source.tc).unwrap(),
                serde_json::to_string(&acc.tc).unwrap()
            );
            assert_eq!(source.codex_records.len(), acc.codex_records.len());
            assert_eq!(source.limits, acc.limits);
        }
    }

    #[test]
    fn identical_content_gets_identical_stub() {
        let (a, _) = stub_for(&Value::String("x".repeat(500)));
        let (b, _) = stub_for(&Value::String("x".repeat(500)));
        let (c, _) = stub_for(&Value::String("y".repeat(500)));
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_eq!(a["$d"].as_str().unwrap().len(), 64);
    }
}
