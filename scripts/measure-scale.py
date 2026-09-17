#!/usr/bin/env python3
"""
Measure how urollup's wall time and peak memory scale with input size.

For each requested corpus size, this generates a synthetic corpus with
`scripts/generate-synthetic-corpus.py`, runs `sessions --all`, `daily --all` and
`report --all` against it, records wall time and peak memory, deletes the corpus, and
moves on. It never reads real agent logs and never needs `--source` pointed at a real
`~/.claude` or `~/.codex` (every invocation passes `--no-default-sources`).

Peak memory comes from the operating system's own accounting: `/usr/bin/time -l` on
macOS ("peak memory footprint") or `/usr/bin/time -v` on Linux ("Maximum resident set
size"). Each measured run is additionally wrapped in `scripts/run-rss-watchdog.py` as a
kill switch, so a regression that grows without bound is terminated instead of
exhausting the machine.

The current engine refuses any discovered input estimated over 512 MiB (see
`crates/urollup/src/cli.rs`); sizes above that are reported as a clean refusal rather
than treated as a crash. Default sizes (32/64/128/256 MiB) stay under that limit.

A final section runs the raw-bytes-independence check: two small corpora with
identical usage records (same seed, same session/rollout counts) but very different
content padding, reporting the peak-memory difference between them.

Run with `uv --config-file uv.toml run --frozen python scripts/measure-scale.py`.
"""

from __future__ import annotations

import argparse
import json
import re
import shutil
import statistics
import subprocess
import sys
import tempfile
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Any

REPO_ROOT = Path(__file__).resolve().parents[1]
GENERATOR = REPO_ROOT / "scripts" / "generate-synthetic-corpus.py"
WATCHDOG = REPO_ROOT / "scripts" / "run-rss-watchdog.py"
DEFAULT_BINARY = REPO_ROOT / "target" / "release" / "urollup"

MIB = 1024 * 1024
DEFAULT_SIZES_MIB = [32, 64, 128, 256]
# The engine's own input-size refusal threshold (crates/urollup/src/cli.rs,
# MAX_ESTIMATED_SOURCE_BYTES); measuring above it is expected to be refused, not crash.
ENGINE_SAFETY_LIMIT_MIB = 512
COMMANDS = ["sessions", "daily", "report"]
REFUSAL_MARKER = "v0.1 safety limit"


@dataclass
class RunResult:
    """One `/usr/bin/time`-wrapped, watchdog-supervised urollup invocation."""

    command: str
    exit_code: int
    wall_seconds: float | None
    peak_bytes: int | None
    watchdog_killed: bool
    watchdog_peak_kib: int
    refused: bool
    error: str | None = None


def time_flag() -> str:
    return "-l" if sys.platform == "darwin" else "-v"


def parse_elapsed(value: str) -> float:
    """Parses GNU time's `[[h:]mm:]ss[.cc]` elapsed-time format into seconds."""
    seconds = 0.0
    for part in value.split(":"):
        seconds = seconds * 60 + float(part)
    return seconds


def parse_time_output(text: str) -> tuple[float | None, int | None]:
    """Extracts (wall_seconds, peak_bytes) from `/usr/bin/time -l`/`-v` output."""
    if sys.platform == "darwin":
        real_match = re.search(r"^\s*([\d.]+)\s+real", text, re.MULTILINE)
        peak_match = re.search(r"^\s*(\d+)\s+peak memory footprint\s*$", text, re.MULTILINE)
        wall = float(real_match.group(1)) if real_match else None
        peak = int(peak_match.group(1)) if peak_match else None
        return wall, peak
    peak_match = re.search(r"Maximum resident set size \(kbytes\):\s*(\d+)", text)
    elapsed_match = re.search(r"Elapsed \(wall clock\) time[^:]*:\s*([\d:.]+)", text)
    peak = int(peak_match.group(1)) * 1024 if peak_match else None
    wall = parse_elapsed(elapsed_match.group(1)) if elapsed_match else None
    return wall, peak


def run_generator(out_dir: Path, extra_args: list[str]) -> dict[str, Any]:
    """Runs the synthetic-corpus generator and returns its parsed JSON summary."""
    argv = [sys.executable, str(GENERATOR), "--out", str(out_dir), *extra_args]
    result = subprocess.run(argv, capture_output=True, text=True, check=False)
    if result.returncode != 0:
        raise RuntimeError(
            f"corpus generation failed (exit {result.returncode}): {result.stderr.strip()}"
        )
    return json.loads(result.stdout)


def run_measured(
    binary: Path,
    command_args: list[str],
    watchdog_limit_mib: int,
    work_dir: Path,
    label: str,
) -> RunResult:
    """Runs one urollup invocation under `/usr/bin/time` and the RSS watchdog."""
    report_path = work_dir / f"watchdog-{label}.json"
    stdout_path = work_dir / f"stdout-{label}.json"
    argv = [
        sys.executable,
        str(WATCHDOG),
        "--limit-mib",
        str(watchdog_limit_mib),
        "--interval-ms",
        "50",
        "--report",
        str(report_path),
        "--",
        "/usr/bin/time",
        time_flag(),
        str(binary),
        *command_args,
    ]
    with stdout_path.open("wb") as stdout_file:
        completed = subprocess.run(argv, stdout=stdout_file, stderr=subprocess.PIPE, check=False)
    stderr_text = completed.stderr.decode("utf-8", errors="replace")
    watchdog_report: dict[str, Any] = {}
    if report_path.exists():
        watchdog_report = json.loads(report_path.read_text(encoding="utf-8"))
    wall_seconds, peak_bytes = parse_time_output(stderr_text)
    refused = REFUSAL_MARKER in stderr_text
    error = None
    if refused:
        error = next(
            (line.strip() for line in stderr_text.splitlines() if REFUSAL_MARKER in line), None
        )
    elif watchdog_report.get("killed_for_rss"):
        error = "killed by the RSS watchdog (peak exceeded the kill-switch limit)"
    elif watchdog_report.get("exit_code", completed.returncode) != 0:
        tail = "\n".join(stderr_text.strip().splitlines()[-5:])
        error = f"exited {watchdog_report.get('exit_code', completed.returncode)}: {tail}"
    return RunResult(
        command=label,
        exit_code=watchdog_report.get("exit_code", completed.returncode),
        wall_seconds=wall_seconds,
        peak_bytes=peak_bytes,
        watchdog_killed=bool(watchdog_report.get("killed_for_rss", False)),
        watchdog_peak_kib=int(watchdog_report.get("peak_rss_kib", 0)),
        refused=refused,
        error=error,
    )


def urollup_args(claude_src: Path, codex_src: Path, command: str) -> list[str]:
    return [
        command,
        "--all",
        "--source",
        str(claude_src),
        "--source",
        str(codex_src),
        "--no-default-sources",
        "--format",
        "json",
    ]


def measure_one_size(binary: Path, size_mib: int, args: argparse.Namespace) -> dict[str, Any]:
    tmp_dir = Path(tempfile.mkdtemp(prefix="urollup-scale-"))
    try:
        corpus_dir = tmp_dir / "corpus"
        summary = run_generator(
            corpus_dir,
            [
                "--seed",
                str(args.seed),
                "--max-bytes",
                str(size_mib * MIB),
                "--claude-fraction",
                str(args.claude_fraction),
            ],
        )
        claude_src = corpus_dir / "claude" / "projects"
        codex_src = corpus_dir / "codex"
        commands: dict[str, dict[str, Any]] = {}
        for command in COMMANDS:
            result = run_measured(
                binary,
                urollup_args(claude_src, codex_src, command),
                args.watchdog_limit_mib,
                tmp_dir,
                f"{size_mib}mib-{command}",
            )
            commands[command] = asdict(result)
        return {
            "input_mib": size_mib,
            "content_bytes": summary["content_bytes"],
            "disk_bytes": summary["disk_bytes"],
            "usage_records": summary["usage_records"],
            "commands": commands,
        }
    finally:
        shutil.rmtree(tmp_dir, ignore_errors=True)


def fit_line(xs: list[float], ys: list[float]) -> tuple[float, float] | None:
    """Ordinary least squares; `None` when there are fewer than two distinct points."""
    if len(xs) < 2 or len(set(xs)) < 2:
        return None
    mean_x = statistics.fmean(xs)
    mean_y = statistics.fmean(ys)
    covariance = sum((x - mean_x) * (y - mean_y) for x, y in zip(xs, ys, strict=True))
    variance = sum((x - mean_x) ** 2 for x in xs)
    slope = covariance / variance
    intercept = mean_y - slope * mean_x
    return slope, intercept


def print_scale_table(rows: list[dict[str, Any]]) -> None:
    header = (
        f"{'input MiB':>10}  {'usage recs':>11}  {'command':>8}  {'wall s':>8}  "
        f"{'peak MiB':>9}  {'status':>10}"
    )
    print(header)
    print("-" * len(header))
    for row in rows:
        for command in COMMANDS:
            result = row["commands"][command]
            status = "ok"
            if result["refused"]:
                status = "refused"
            elif result["watchdog_killed"]:
                status = "killed"
            elif result["exit_code"] != 0:
                status = f"exit {result['exit_code']}"
            wall = f"{result['wall_seconds']:.2f}" if result["wall_seconds"] is not None else "n/a"
            peak = (
                f"{result['peak_bytes'] / MIB:.1f}" if result["peak_bytes"] is not None else "n/a"
            )
            print(
                f"{row['input_mib']:>10}  {row['usage_records']:>11}  {command:>8}  "
                f"{wall:>8}  {peak:>9}  {status:>10}"
            )
    print()
    for command in COMMANDS:
        xs = [row["input_mib"] for row in rows if row["commands"][command]["peak_bytes"] is not None]
        ys = [
            row["commands"][command]["peak_bytes"]
            for row in rows
            if row["commands"][command]["peak_bytes"] is not None
        ]
        fit = fit_line([float(x) for x in xs], [float(y) for y in ys])
        if fit is None:
            print(f"{command}: not enough successful runs to fit a slope")
            continue
        slope, intercept = fit
        print(
            f"{command}: peak_bytes ~= {slope:.0f} * input_MiB + {intercept:.0f} "
            f"(slope {slope / MIB:.4f} MiB peak per MiB input, intercept {intercept / MIB:.1f} MiB)"
        )


def measure_independence(binary: Path, args: argparse.Namespace) -> dict[str, Any]:
    tmp_dir = Path(tempfile.mkdtemp(prefix="urollup-scale-independence-"))
    try:
        fixed_args = [
            "--seed",
            str(args.seed),
            "--max-bytes",
            "0",
            "--days",
            str(args.independence_days),
            "--claude-sessions-per-day",
            str(args.independence_sessions_per_day),
            "--codex-rollouts-per-day",
            str(args.independence_rollouts_per_day),
        ]
        variants = {
            "base": args.independence_base_padding_bytes,
            "padded": args.independence_base_padding_bytes * args.independence_padding_multiplier,
        }
        results: dict[str, Any] = {}
        for label, padding in variants.items():
            corpus_dir = tmp_dir / label
            summary = run_generator(
                corpus_dir, [*fixed_args, "--content-padding-bytes", str(padding)]
            )
            claude_src = corpus_dir / "claude" / "projects"
            codex_src = corpus_dir / "codex"
            run_result = run_measured(
                binary,
                urollup_args(claude_src, codex_src, "report"),
                args.watchdog_limit_mib,
                tmp_dir,
                f"independence-{label}",
            )
            results[label] = {
                "padding_bytes": padding,
                "content_bytes": summary["content_bytes"],
                "usage_records": summary["usage_records"],
                "run": asdict(run_result),
            }
            shutil.rmtree(corpus_dir, ignore_errors=True)

        base_records = results["base"]["usage_records"]
        padded_records = results["padded"]["usage_records"]
        identical_usage = base_records == padded_records
        base_peak = results["base"]["run"]["peak_bytes"]
        padded_peak = results["padded"]["run"]["peak_bytes"]
        peak_diff = padded_peak - base_peak if base_peak is not None and padded_peak is not None else None
        return {
            "identical_usage_records": identical_usage,
            "results": results,
            "peak_diff_bytes": peak_diff,
        }
    finally:
        shutil.rmtree(tmp_dir, ignore_errors=True)


def print_independence(independence: dict[str, Any]) -> None:
    print()
    print("Raw-bytes independence check (identical usage records, different content padding):")
    base = independence["results"]["base"]
    padded = independence["results"]["padded"]
    print(
        f"  base:   {base['padding_bytes']:>8} bytes/field padding, "
        f"{base['content_bytes'] / MIB:.2f} MiB content, {base['usage_records']} usage records, "
        f"peak {base['run']['peak_bytes'] / MIB:.2f} MiB"
        if base["run"]["peak_bytes"] is not None
        else f"  base:   run failed: {base['run']['error']}"
    )
    print(
        f"  padded: {padded['padding_bytes']:>8} bytes/field padding, "
        f"{padded['content_bytes'] / MIB:.2f} MiB content, {padded['usage_records']} usage records, "
        f"peak {padded['run']['peak_bytes'] / MIB:.2f} MiB"
        if padded["run"]["peak_bytes"] is not None
        else f"  padded: run failed: {padded['run']['error']}"
    )
    if not independence["identical_usage_records"]:
        print(
            "  WARNING: usage record counts differ between the two corpora; "
            "the independence comparison is not valid"
        )
    if independence["peak_diff_bytes"] is not None:
        print(f"  peak difference: {independence['peak_diff_bytes'] / MIB:+.2f} MiB")


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument(
        "--sizes-mib",
        type=str,
        default=",".join(str(size) for size in DEFAULT_SIZES_MIB),
        help="comma-separated corpus sizes in MiB (default: 32,64,128,256)",
    )
    parser.add_argument("--binary", type=Path, default=DEFAULT_BINARY, help="urollup binary to measure (default: target/release/urollup)")
    parser.add_argument("--seed", type=int, default=1, help="generator seed (default: 1)")
    parser.add_argument("--claude-fraction", type=float, default=0.2, help="share of each corpus given to Claude content (default: 0.2)")
    parser.add_argument("--watchdog-limit-mib", type=int, default=2048, help="RSS watchdog kill-switch limit in MiB (default: 2048)")
    parser.add_argument("--skip-independence", action="store_true", help="skip the raw-bytes independence check")
    parser.add_argument("--independence-base-padding-bytes", type=int, default=300, help="baseline per-field content padding for the independence check (default: 300)")
    parser.add_argument("--independence-padding-multiplier", type=int, default=10, help="padding multiplier for the independence check's padded corpus (default: 10)")
    parser.add_argument("--independence-days", type=int, default=5, help="days spanned by each independence-check corpus (default: 5)")
    parser.add_argument("--independence-sessions-per-day", type=int, default=8, help="Claude sessions per day for the independence check (default: 8)")
    parser.add_argument("--independence-rollouts-per-day", type=int, default=8, help="Codex rollouts per day for the independence check (default: 8)")
    args = parser.parse_args(argv)
    try:
        args.sizes_mib_list = [int(part) for part in args.sizes_mib.split(",") if part.strip()]
    except ValueError:
        parser.error(f"--sizes-mib must be a comma-separated list of integers, got {args.sizes_mib!r}")
    if not args.sizes_mib_list:
        parser.error("--sizes-mib must name at least one size")
    return args


def main(argv: list[str] | None = None) -> int:
    args = parse_args(sys.argv[1:] if argv is None else argv)
    binary = args.binary
    if not binary.exists():
        print(
            f"error: binary not found: {binary}; build it first with "
            "`cargo build --release --locked --workspace`",
            file=sys.stderr,
        )
        return 1

    for size_mib in args.sizes_mib_list:
        if size_mib > ENGINE_SAFETY_LIMIT_MIB:
            print(
                f"note: {size_mib} MiB exceeds the engine's {ENGINE_SAFETY_LIMIT_MIB} MiB "
                "input-size safety limit; expect a clean refusal, not a crash",
                file=sys.stderr,
            )

    rows = []
    for size_mib in args.sizes_mib_list:
        print(f"measuring {size_mib} MiB...", file=sys.stderr)
        rows.append(measure_one_size(binary, size_mib, args))

    print()
    print_scale_table(rows)

    if not args.skip_independence:
        print("measuring raw-bytes independence...", file=sys.stderr)
        independence = measure_independence(binary, args)
        print_independence(independence)

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
