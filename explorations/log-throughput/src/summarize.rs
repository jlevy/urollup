//! Median tables and cross-variant agreement from `run` JSON lines.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde_json::Value;

use crate::Result;
use crate::args::Args;
use crate::metrics::median;

fn f(v: &Value, k: &str) -> f64 {
    v.get(k).and_then(Value::as_f64).unwrap_or(f64::NAN)
}

/// `summarize --results FILE`
pub fn cmd(args: &Args) -> Result<()> {
    let text = fs::read_to_string(Path::new(args.required("results")?))?;
    let runs: Vec<Value> = text
        .lines()
        .filter(|l| l.starts_with('{'))
        .map(serde_json::from_str)
        .collect::<std::result::Result<_, _>>()?;

    let mode_order = ["read", "value", "typed", "prefilter", "cache", "cache-verify"];
    let mut groups: BTreeMap<(String, usize, u64), Vec<&Value>> = BTreeMap::new();
    for r in &runs {
        let slice = r["slice"].as_str().unwrap_or("").to_string();
        let mode = r["mode"].as_str().unwrap_or("");
        let order = mode_order.iter().position(|m| *m == mode).unwrap_or(99);
        groups.entry((slice, order, r["threads"].as_u64().unwrap_or(0))).or_default().push(r);
    }

    println!(
        "| Slice | Mode | Threads | Runs | Median wall s | First-run wall s | Median CPU s | Peak RSS MiB | MiB/s | Lines/s | Load1 (median) |"
    );
    println!("| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |");
    for ((slice, order, threads), rs) in &groups {
        let med = |k: &str| median(&mut rs.iter().map(|r| f(r, k)).collect::<Vec<_>>());
        let first = rs
            .iter()
            .min_by_key(|r| r["at"].as_i64().unwrap_or(0))
            .map(|r| f(r, "wall_s"))
            .unwrap_or(f64::NAN);
        println!(
            "| {slice} | {} | {threads} | {} | {:.2} | {:.2} | {:.2} | {:.0} | {:.0} | {:.0} | {:.1} |",
            mode_order.get(*order).unwrap_or(&"?"),
            rs.len(),
            med("wall_s"),
            first,
            med("cpu_s"),
            med("max_rss_mib"),
            med("mib_per_s"),
            med("lines_per_s"),
            med("load1_before"),
        );
    }

    println!();
    let mut by_slice: BTreeMap<String, BTreeMap<String, Vec<String>>> = BTreeMap::new();
    for r in &runs {
        if let Some(d) = r["digest"].as_str() {
            let key = format!("{}@{}", r["mode"].as_str().unwrap_or(""), r["threads"]);
            by_slice
                .entry(r["slice"].as_str().unwrap_or("").into())
                .or_default()
                .entry(d.into())
                .or_default()
                .push(key);
        }
    }
    for (slice, digests) in &by_slice {
        if digests.len() == 1 {
            println!(
                "slice {slice}: all parsing variants agree on totals ({} runs)",
                digests.values().map(Vec::len).sum::<usize>()
            );
        } else {
            println!("slice {slice}: MISMATCH across {} distinct totals", digests.len());
            for (d, who) in digests {
                let mut who = who.clone();
                who.sort();
                who.dedup();
                println!("  {d}: {}", who.join(", "));
            }
        }
    }
    Ok(())
}
