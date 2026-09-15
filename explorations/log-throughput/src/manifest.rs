//! Source discovery and a frozen snapshot manifest.
//!
//! The manifest records each source file's byte extent at discovery, so repeated runs
//! read identical bytes even while agents keep appending to active logs. It holds local
//! paths, so it must live only in an ephemeral directory; nothing path-bearing is printed.

use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::time::{Instant, UNIX_EPOCH};

use serde_json::json;

use crate::Result;
use crate::args::Args;
use crate::metrics::{epoch_to_day, fnv64, now_epoch, percentile};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Agent {
    Claude,
    Codex,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// Claude top-level session transcript.
    Session,
    /// Claude `subagents/agent-*.jsonl` transcript.
    Subagent,
    /// Codex JSONL rollout (plain or compressed).
    Rollout,
    /// Codex pre-JSONL `rollout-*.json` file; listed but never parsed.
    LegacyJson,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Comp {
    Plain,
    Gzip,
    Zstd,
}

#[derive(Clone, Debug)]
pub struct Entry {
    pub idx: usize,
    pub agent: Agent,
    pub kind: Kind,
    pub comp: Comp,
    pub size: u64,
    pub mtime: i64,
    pub path: PathBuf,
}

pub struct Manifest {
    pub created: i64,
    pub cutoff: i64,
    pub entries: Vec<Entry>,
}

impl Entry {
    /// Stable, path-free name for this source's captured records.
    pub fn capture_name(&self) -> String {
        format!("{:016x}.jsonl.zst", fnv64(self.path.as_os_str().as_encoded_bytes()))
    }
}

impl Manifest {
    pub fn cutoff_day(&self) -> u32 {
        epoch_to_day(self.cutoff)
    }

    pub fn load(path: &Path) -> Result<Manifest> {
        let reader = BufReader::new(fs::File::open(path)?);
        let (mut created, mut cutoff) = (0, 0);
        let mut entries = Vec::new();
        for line in reader.lines() {
            let line = line?;
            if let Some(rest) = line.strip_prefix("#created=") {
                created = rest.parse()?;
                continue;
            }
            if let Some(rest) = line.strip_prefix("#cutoff=") {
                cutoff = rest.parse()?;
                continue;
            }
            let f: Vec<&str> = line.splitn(7, '\t').collect();
            if f.len() != 7 {
                return Err("malformed manifest row".into());
            }
            entries.push(Entry {
                idx: f[0].parse()?,
                agent: match f[1] {
                    "claude" => Agent::Claude,
                    _ => Agent::Codex,
                },
                kind: match f[2] {
                    "session" => Kind::Session,
                    "subagent" => Kind::Subagent,
                    "rollout" => Kind::Rollout,
                    _ => Kind::LegacyJson,
                },
                comp: match f[3] {
                    "gzip" => Comp::Gzip,
                    "zstd" => Comp::Zstd,
                    _ => Comp::Plain,
                },
                size: f[4].parse()?,
                mtime: f[5].parse()?,
                path: PathBuf::from(f[6]),
            });
        }
        Ok(Manifest { created, cutoff, entries })
    }
}

fn classify(agent: Agent, path: &Path) -> Option<(Kind, Comp)> {
    let name = path.file_name()?.to_str()?;
    match agent {
        Agent::Claude => {
            if !name.ends_with(".jsonl") {
                return None;
            }
            let parent = path.parent().and_then(Path::file_name).and_then(|n| n.to_str());
            let kind = if parent == Some("subagents") { Kind::Subagent } else { Kind::Session };
            Some((kind, Comp::Plain))
        }
        Agent::Codex => {
            if !name.starts_with("rollout-") {
                return None;
            }
            if name.ends_with(".jsonl") {
                Some((Kind::Rollout, Comp::Plain))
            } else if name.ends_with(".jsonl.zst") {
                Some((Kind::Rollout, Comp::Zstd))
            } else if name.ends_with(".jsonl.gz") {
                Some((Kind::Rollout, Comp::Gzip))
            } else if name.ends_with(".json") {
                Some((Kind::LegacyJson, Comp::Plain))
            } else {
                None
            }
        }
    }
}

#[derive(Default)]
struct WalkStats {
    other_files: u64,
    symlinks: u64,
}

fn walk(dir: &Path, agent: Agent, out: &mut Vec<Entry>, stats: &mut WalkStats) {
    let Ok(read) = fs::read_dir(dir) else { return };
    for item in read.flatten() {
        let path = item.path();
        let Ok(meta) = fs::symlink_metadata(&path) else { continue };
        if meta.file_type().is_symlink() {
            stats.symlinks += 1;
        } else if meta.is_dir() {
            walk(&path, agent, out, stats);
        } else if let Some((kind, comp)) = classify(agent, &path) {
            let s = path.to_string_lossy();
            if s.contains('\t') || s.contains('\n') {
                stats.other_files += 1;
                continue;
            }
            let mtime = meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);
            out.push(Entry { idx: 0, agent, kind, comp, size: meta.len(), mtime, path });
        } else {
            stats.other_files += 1;
        }
    }
}

fn agent_name(a: Agent) -> &'static str {
    match a {
        Agent::Claude => "claude",
        Agent::Codex => "codex",
    }
}

/// Volume statistics from file metadata alone (no content reads).
fn volume(entries: &[&Entry]) -> serde_json::Value {
    let parsed: Vec<&&Entry> = entries.iter().filter(|e| e.kind != Kind::LegacyJson).collect();
    let mut sizes: Vec<u64> = parsed.iter().map(|e| e.size).collect();
    sizes.sort_unstable();
    let count = |k: Kind| entries.iter().filter(|e| e.kind == k).count();
    let comp = |c: Comp| {
        let v: Vec<_> = parsed.iter().filter(|e| e.comp == c).collect();
        json!({"files": v.len(), "bytes": v.iter().map(|e| e.size).sum::<u64>()})
    };
    let legacy: Vec<_> = entries.iter().filter(|e| e.kind == Kind::LegacyJson).collect();
    json!({
        "files": parsed.len(),
        "bytes": sizes.iter().sum::<u64>(),
        "size_p50": percentile(&sizes, 50.0),
        "size_p90": percentile(&sizes, 90.0),
        "size_p99": percentile(&sizes, 99.0),
        "size_max": sizes.last().copied().unwrap_or(0),
        "top10_share": if sizes.is_empty() { 0.0 } else {
            sizes.iter().rev().take(10).sum::<u64>() as f64 / sizes.iter().sum::<u64>().max(1) as f64
        },
        "sessions": count(Kind::Session),
        "subagent_files": count(Kind::Subagent),
        "rollouts": count(Kind::Rollout),
        "plain": comp(Comp::Plain),
        "gzip": comp(Comp::Gzip),
        "zstd": comp(Comp::Zstd),
        "legacy_json": {"files": legacy.len(), "bytes": legacy.iter().map(|e| e.size).sum::<u64>()},
    })
}

/// `manifest --out FILE [--days N] [--claude-root DIR] [--codex-root DIR]`
pub fn cmd(args: &Args) -> Result<()> {
    let out = PathBuf::from(args.required("out")?);
    let days: i64 = args.value("days").unwrap_or("30").parse()?;
    let home = PathBuf::from(std::env::var("HOME")?);
    let claude_root = args
        .value("claude-root")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".claude/projects"));
    let codex_roots: Vec<PathBuf> = match args.value("codex-root") {
        Some(r) => vec![PathBuf::from(r)],
        None => vec![home.join(".codex/sessions"), home.join(".codex/archived_sessions")],
    };

    let t0 = Instant::now();
    let mut entries = Vec::new();
    let mut stats = WalkStats::default();
    walk(&claude_root, Agent::Claude, &mut entries, &mut stats);
    for root in &codex_roots {
        walk(root, Agent::Codex, &mut entries, &mut stats);
    }
    let walk_s = t0.elapsed().as_secs_f64();
    entries.sort_by(|a, b| a.path.cmp(&b.path));
    for (i, e) in entries.iter_mut().enumerate() {
        e.idx = i;
    }

    let created = now_epoch();
    let cutoff = created - days * 86_400;
    let mut w = std::io::BufWriter::new(fs::File::create(&out)?);
    writeln!(w, "#created={created}")?;
    writeln!(w, "#cutoff={cutoff}")?;
    for e in &entries {
        let kind = match e.kind {
            Kind::Session => "session",
            Kind::Subagent => "subagent",
            Kind::Rollout => "rollout",
            Kind::LegacyJson => "legacy",
        };
        let comp = match e.comp {
            Comp::Plain => "plain",
            Comp::Gzip => "gzip",
            Comp::Zstd => "zstd",
        };
        writeln!(
            w,
            "{}\t{}\t{kind}\t{comp}\t{}\t{}\t{}",
            e.idx,
            agent_name(e.agent),
            e.size,
            e.mtime,
            e.path.display()
        )?;
    }
    w.flush()?;

    let mut report = serde_json::Map::new();
    for agent in [Agent::Claude, Agent::Codex] {
        let all: Vec<&Entry> = entries.iter().filter(|e| e.agent == agent).collect();
        let recent: Vec<&Entry> = all.iter().copied().filter(|e| e.mtime >= cutoff).collect();
        report.insert(
            agent_name(agent).into(),
            json!({"all": volume(&all), "mtime_window": volume(&recent)}),
        );
    }
    let report = json!({
        "created": created,
        "cutoff_day": epoch_to_day(cutoff),
        "days": days,
        "walk_s": walk_s,
        "other_files_skipped": stats.other_files,
        "symlinks_skipped": stats.symlinks,
        "agents": report,
    });
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_by_dialect_layout() {
        let c = |p: &str| classify(Agent::Claude, Path::new(p));
        assert_eq!(c("/r/proj/s.jsonl"), Some((Kind::Session, Comp::Plain)));
        assert_eq!(c("/r/proj/s/subagents/agent-a.jsonl"), Some((Kind::Subagent, Comp::Plain)));
        assert_eq!(c("/r/proj/s/subagents/agent-a.meta.json"), None);
        assert_eq!(c("/r/proj/s/tool-results/x.txt"), None);
        let x = |p: &str| classify(Agent::Codex, Path::new(p));
        assert_eq!(x("/r/2026/09/14/rollout-a.jsonl"), Some((Kind::Rollout, Comp::Plain)));
        assert_eq!(x("/r/2026/09/14/rollout-a.jsonl.zst"), Some((Kind::Rollout, Comp::Zstd)));
        assert_eq!(x("/r/2026/09/14/rollout-a.jsonl.gz"), Some((Kind::Rollout, Comp::Gzip)));
        assert_eq!(x("/r/rollout-a.json"), Some((Kind::LegacyJson, Comp::Plain)));
        assert_eq!(x("/r/.DS_Store"), None);
    }
}
