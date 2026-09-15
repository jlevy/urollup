//! One benchmark run: a workload over a manifest slice, printed as one JSON line.

use std::collections::hash_map::DefaultHasher;
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::hash::Hasher;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::Instant;

use rayon::prelude::*;
use serde::Serialize;
use serde_json::json;

use crate::Result;
use crate::args::Args;
use crate::extract::{ClaudeUsage, CodexUsage, Decoder, FileAcc, TokenCountAcc, process_line};
use crate::lines::{LineStats, for_each_line, open_entry, open_zstd, read_only};
use crate::manifest::{Agent, Entry, Kind, Manifest};
use crate::metrics::{MIB, epoch_to_day, fnv64, load1, now_epoch, rusage};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// (a) Read and decompress bytes, count newlines.
    Read,
    /// (b) `serde_json::Value` parse of every line.
    Value,
    /// (b) Typed borrowed parse of every line.
    Typed,
    /// (c) Byte prefilter, then typed parse.
    Prefilter,
    /// (d) Typed parse of captured records; sources are only stat'ed.
    Cache,
    /// (d) As `Cache`, plus reading each source's first record and final 64 KiB.
    CacheVerify,
}

impl Mode {
    fn parse(s: &str) -> Result<Mode> {
        Ok(match s {
            "read" => Mode::Read,
            "value" => Mode::Value,
            "typed" => Mode::Typed,
            "prefilter" => Mode::Prefilter,
            "cache" => Mode::Cache,
            "cache-verify" => Mode::CacheVerify,
            _ => return Err("unknown --mode".into()),
        })
    }
    fn name(self) -> &'static str {
        match self {
            Mode::Read => "read",
            Mode::Value => "value",
            Mode::Typed => "typed",
            Mode::Prefilter => "prefilter",
            Mode::Cache => "cache",
            Mode::CacheVerify => "cache-verify",
        }
    }
}

#[derive(Default)]
struct FileOut {
    agent: Option<Agent>,
    stats: LineStats,
    acc: FileAcc,
    io_errors: u64,
    stale_sources: u64,
    verify_bytes: u64,
}

const TAIL: u64 = 64 * 1024;

/// Reads what the capture cache's default prefix check needs: the first complete record
/// and the final 64 KiB of the captured extent, each hashed.
fn verify_source(entry: &Entry) -> std::io::Result<u64> {
    let mut f = fs::File::open(&entry.path)?;
    let mut read = 0u64;
    let mut head = Vec::new();
    let mut r = BufReader::with_capacity(TAIL as usize, (&mut f).take(entry.size));
    r.read_until(b'\n', &mut head)?;
    read += head.len() as u64;
    let mut h = DefaultHasher::new();
    h.write(&head);
    drop(r);
    let start = entry.size.saturating_sub(TAIL);
    f.seek(SeekFrom::Start(start))?;
    let mut tail = Vec::with_capacity(TAIL as usize);
    f.take(entry.size - start).read_to_end(&mut tail)?;
    read += tail.len() as u64;
    h.write(&tail);
    std::hint::black_box(h.finish());
    Ok(read)
}

fn process(entry: &Entry, mode: Mode, capture_dir: Option<&Path>) -> FileOut {
    let mut out = FileOut { agent: Some(entry.agent), ..Default::default() };
    // Every mode pays a stat per source, as discovery and cache validation would.
    match fs::metadata(&entry.path) {
        Ok(m) if m.len() != entry.size => out.stale_sources += 1,
        Ok(_) => {}
        Err(_) => out.io_errors += 1,
    }
    let result = match mode {
        Mode::Read => open_entry(entry).and_then(read_only),
        Mode::Value | Mode::Typed | Mode::Prefilter => {
            let decoder = if mode == Mode::Value { Decoder::Value } else { Decoder::Typed };
            let pre = mode == Mode::Prefilter;
            let acc = &mut out.acc;
            open_entry(entry)
                .and_then(|r| for_each_line(r, |l| process_line(entry.agent, decoder, pre, l, acc)))
        }
        Mode::Cache | Mode::CacheVerify => {
            if mode == Mode::CacheVerify {
                match verify_source(entry) {
                    Ok(n) => out.verify_bytes += n,
                    Err(_) => out.io_errors += 1,
                }
            }
            let path = capture_dir.expect("capture dir").join(entry.capture_name());
            let acc = &mut out.acc;
            open_zstd(&path).and_then(|r| {
                for_each_line(r, |l| process_line(entry.agent, Decoder::Typed, false, l, acc))
            })
        }
    };
    match result {
        Ok(s) => out.stats = s,
        Err(_) => out.io_errors += 1,
    }
    out.acc.finish();
    out
}

#[derive(Default, Serialize)]
struct ClaudeTotals {
    observations: u64,
    unique_requests: u64,
    dup_same_file: u64,
    dup_cross_file: u64,
    conflicting_usage: u64,
    missing_request_id: u64,
    no_message_id: u64,
    no_message_id_usage: ClaudeUsage,
    limits: u64,
    usage: ClaudeUsage,
}

#[derive(Default, Serialize)]
struct CodexTotals {
    record_observations: u64,
    record_unique: u64,
    record_dup_same_file: u64,
    record_dup_cross_file: u64,
    record_conflicting_usage: u64,
    record_usage: CodexUsage,
    token_count: TokenCountAcc,
    limits: u64,
    session_meta: u64,
    forked: u64,
    subagent: u64,
    turn_contexts: u64,
}

/// Dialect diagnostics, reported beside totals but excluded from the agreement digest.
#[derive(Default, Serialize)]
struct Diagnostics {
    claude_conflicts_same_file: u64,
    claude_conflicts_output_only: u64,
    claude_conflicts_later_larger: u64,
    /// Output tokens if the first observation per key were kept instead of the largest.
    claude_first_seen_output: u64,
    claude_max_output: u64,
    codex_tc_cross_file_replayed: u64,
    codex_tc_replayed_delta_total: u64,
    codex_files_with_records_and_tc: u64,
    codex_both_record_total: u64,
    codex_both_tc_delta_total: u64,
}

#[derive(Default, Serialize)]
struct Totals {
    claude: ClaudeTotals,
    codex: CodexTotals,
    active_days: usize,
    daily_digest: String,
}

fn add_tc(t: &mut TokenCountAcc, f: &TokenCountAcc) {
    t.events += f.events;
    t.null_info += f.null_info;
    t.identical += f.identical;
    t.resets += f.resets;
    t.delta_mismatch += f.delta_mismatch;
    t.sum_last.add(&f.sum_last);
    t.epoch_final_sum += f.epoch_final_sum;
}

/// Deterministic merge in manifest order: dedupe Claude requests by `message.id` plus
/// `requestId` and Codex `token_usage_record` by `response_id`, then bucket by UTC day.
fn merge(outs: &[(usize, FileOut)]) -> (Totals, Diagnostics) {
    let mut t = Totals::default();
    let mut g = Diagnostics::default();
    let mut first_output: HashMap<&str, u64> = HashMap::new();
    let mut snapshots: HashMap<u64, usize> = HashMap::new();
    let mut claude: HashMap<&str, (ClaudeUsage, usize, u32)> = HashMap::new();
    let mut codex: HashMap<&str, (CodexUsage, usize, u32)> = HashMap::new();
    let mut daily: BTreeMap<u32, (ClaudeUsage, CodexUsage, CodexUsage)> = BTreeMap::new();
    for (idx, o) in outs {
        let a = &o.acc;
        for obs in &a.claude {
            t.claude.observations += 1;
            if !obs.has_request_id {
                t.claude.missing_request_id += 1;
            }
            match claude.get_mut(&*obs.key) {
                None => {
                    claude.insert(&obs.key, (obs.usage, *idx, obs.day));
                    first_output.insert(&obs.key, obs.usage.output);
                }
                Some(prev) => {
                    if prev.1 == *idx {
                        t.claude.dup_same_file += 1
                    } else {
                        t.claude.dup_cross_file += 1
                    }
                    if prev.0 != obs.usage {
                        t.claude.conflicting_usage += 1;
                        if prev.1 == *idx {
                            g.claude_conflicts_same_file += 1;
                        }
                        let (p, u) = (prev.0, obs.usage);
                        if p.input == u.input
                            && p.cache_create == u.cache_create
                            && p.cache_read == u.cache_read
                        {
                            g.claude_conflicts_output_only += 1;
                        }
                        if obs.usage.sum() > prev.0.sum() {
                            g.claude_conflicts_later_larger += 1;
                            prev.0 = obs.usage;
                        }
                    }
                }
            }
        }
        t.claude.no_message_id += a.claude_no_message_id;
        t.claude.no_message_id_usage.add(&a.claude_no_id_usage);
        for obs in &a.codex_records {
            t.codex.record_observations += 1;
            match codex.get_mut(&*obs.response_id) {
                None => {
                    codex.insert(&obs.response_id, (obs.usage, *idx, obs.day));
                }
                Some(prev) => {
                    if prev.1 == *idx {
                        t.codex.record_dup_same_file += 1
                    } else {
                        t.codex.record_dup_cross_file += 1
                    }
                    if prev.0 != obs.usage {
                        t.codex.record_conflicting_usage += 1;
                    }
                }
            }
        }
        add_tc(&mut t.codex.token_count, &a.tc);
        for (key, delta) in &a.tc_snapshots {
            match snapshots.get(key) {
                Some(first) if first != idx => {
                    g.codex_tc_cross_file_replayed += 1;
                    g.codex_tc_replayed_delta_total += delta;
                }
                Some(_) => {}
                None => {
                    snapshots.insert(*key, *idx);
                }
            }
        }
        if !a.codex_records.is_empty() && a.tc.events > 0 {
            g.codex_files_with_records_and_tc += 1;
            g.codex_both_record_total += a.codex_records.iter().map(|r| r.usage.total).sum::<u64>();
            g.codex_both_tc_delta_total += a.tc.sum_last.total;
        }
        for (day, u) in &a.tc_days {
            daily.entry(*day).or_default().2.add(u);
        }
        match o.agent {
            Some(Agent::Claude) => t.claude.limits += a.limits,
            _ => t.codex.limits += a.limits,
        }
        t.codex.session_meta += a.session_meta;
        t.codex.forked += a.forked;
        t.codex.subagent += a.subagent;
        t.codex.turn_contexts += a.turn_contexts;
    }
    g.claude_first_seen_output = first_output.values().sum();
    for (u, _, day) in claude.values() {
        g.claude_max_output += u.output;
        t.claude.unique_requests += 1;
        t.claude.usage.add(u);
        daily.entry(*day).or_default().0.add(u);
    }
    for (u, _, day) in codex.values() {
        t.codex.record_unique += 1;
        t.codex.record_usage.add(u);
        daily.entry(*day).or_default().1.add(u);
    }
    t.active_days = daily.keys().filter(|d| **d != 0).count();
    let mut digest = Vec::new();
    for (d, (c, x, tc)) in &daily {
        digest.extend_from_slice(format!("{d}:{c:?}:{x:?}:{tc:?};").as_bytes());
    }
    t.daily_digest = format!("{:016x}", fnv64(&digest));
    (t, g)
}

/// Source lines and logical bytes recorded by `capture`, keyed by capture name.
fn load_index(dir: &Path) -> Result<HashMap<String, (u64, u64, u64)>> {
    let mut map = HashMap::new();
    for line in BufReader::new(fs::File::open(dir.join("index.tsv"))?).lines() {
        let line = line?;
        let f: Vec<&str> = line.split('\t').collect();
        if f.len() == 5 {
            map.insert(f[0].to_string(), (f[1].parse()?, f[2].parse()?, f[4].parse()?));
        }
    }
    Ok(map)
}

/// `run --manifest FILE --mode MODE --threads N --slice all|window [--capture DIR] [--rep N]`
pub fn cmd(args: &Args) -> Result<()> {
    let manifest_path = PathBuf::from(args.required("manifest")?);
    let mode = Mode::parse(args.required("mode")?)?;
    let threads: usize = args.value("threads").unwrap_or("1").parse()?;
    let slice = args.value("slice").unwrap_or("all").to_string();
    let rep: u64 = args.value("rep").unwrap_or("0").parse()?;
    let capture_dir = args.value("capture").map(PathBuf::from);
    if matches!(mode, Mode::Cache | Mode::CacheVerify) && capture_dir.is_none() {
        return Err("cache modes need --capture".into());
    }
    let load_before = load1();

    let t0 = Instant::now();
    let manifest = Manifest::load(&manifest_path)?;
    let entries: Vec<&Entry> = manifest
        .entries
        .iter()
        .filter(|e| e.kind != Kind::LegacyJson)
        .filter(|e| slice == "all" || e.mtime >= manifest.cutoff)
        .collect();
    let work = |e: &&Entry| (e.idx, process(e, mode, capture_dir.as_deref()));
    let outs: Vec<(usize, FileOut)> = if threads <= 1 {
        entries.iter().map(work).collect()
    } else {
        let pool = rayon::ThreadPoolBuilder::new().num_threads(threads).build()?;
        pool.install(|| entries.par_iter().map(work).collect())
    };
    let scan_s = t0.elapsed().as_secs_f64();
    let (totals, diagnostics) = match mode {
        Mode::Read => (None, None),
        _ => {
            let (t, g) = merge(&outs);
            (Some(t), Some(g))
        }
    };
    let wall_s = t0.elapsed().as_secs_f64();
    let ru = rusage();
    let load_after = load1();

    let sum = |f: &dyn Fn(&FileOut) -> u64| outs.iter().map(|(_, o)| f(o)).sum::<u64>();
    let source_bytes: u64 = entries.iter().map(|e| e.size).sum();
    let read_bytes = sum(&|o| o.stats.bytes);
    let (logical_bytes, source_lines, cache_bytes) = match &capture_dir {
        Some(dir) if matches!(mode, Mode::Cache | Mode::CacheVerify) => {
            let index = load_index(dir)?;
            let pick = |i: usize| {
                entries
                    .iter()
                    .map(|e| index.get(&e.capture_name()).map_or(0, |v| [v.0, v.1, v.2][i]))
                    .sum::<u64>()
            };
            (pick(0), pick(1), pick(2))
        }
        _ => (read_bytes, sum(&|o| o.stats.lines), 0),
    };
    let digest = totals
        .as_ref()
        .map(|t| serde_json::to_string(t).map(|s| format!("{:016x}", fnv64(s.as_bytes()))))
        .transpose()?;
    let out = json!({
        "mode": mode.name(),
        "threads": threads,
        "slice": slice,
        "rep": rep,
        "at": now_epoch(),
        "manifest_created": manifest.created,
        "cutoff_day": epoch_to_day(manifest.cutoff),
        "load1_before": load_before,
        "load1_after": load_after,
        "files": entries.len(),
        "source_bytes": source_bytes,
        "logical_bytes": logical_bytes,
        "bytes_read": read_bytes,
        "cache_bytes": cache_bytes,
        "verify_bytes": sum(&|o| o.verify_bytes),
        "lines_read": sum(&|o| o.stats.lines),
        "source_lines": source_lines,
        "parsed_lines": sum(&|o| o.acc.parsed),
        "parse_errors": sum(&|o| o.acc.parse_errors),
        "shape_errors": sum(&|o| o.acc.shape_errors),
        "pending_bytes": sum(&|o| o.stats.pending_bytes),
        "io_errors": sum(&|o| o.io_errors),
        "stale_sources": sum(&|o| o.stale_sources),
        "wall_s": wall_s,
        "scan_s": scan_s,
        "merge_s": wall_s - scan_s,
        "user_s": ru.user_s,
        "sys_s": ru.sys_s,
        "cpu_s": ru.user_s + ru.sys_s,
        "max_rss_mib": ru.max_rss_bytes as f64 / MIB,
        "mib_per_s": logical_bytes as f64 / MIB / wall_s,
        "lines_per_s": source_lines as f64 / wall_s,
        "digest": digest,
        "totals": totals,
        "diagnostics": diagnostics,
    });
    println!("{}", serde_json::to_string(&out)?);
    Ok(())
}
