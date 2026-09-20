#!/usr/bin/env python3
"""
CI scale gate for the scalable-ingestion Memory Model (Phase 2).

This asserts the three properties the plan's Phase 2 requires of the engine, using
small synthetic corpora so the whole gate runs in well under two minutes even on a
loaded machine:

1. **Raw-bytes independence.** Two corpora with identical usage records but very
   different content padding must have peak-memory footprints within
   `--independence-ceiling-mib` of each other (default 64 MiB, the plan's own bound).
   A large difference would mean memory is scaling with raw log bytes rather than with
   decoded usage records.
2. **Footprint extrapolation bound.** Peak memory at a few small corpus sizes is fit to
   `peak_bytes ~= intercept + slope * usage_records` (not `* input_MiB`: see "Usage-record
   density" below), and both the fitted slope and intercept must stay under calibrated
   ceilings.
3. **`daily --all` under the RSS watchdog at 512 MiB.** A smoke test that a realistic
   invocation stays within the plan's 512 MiB footprint goal, using
   `scripts/run-rss-watchdog.py` as an external kill switch.

This never reads real agent logs (`--no-default-sources` always) and never generates
more than 256 MiB in one corpus. Every generated corpus lives in a temporary directory
outside the repository and is deleted as soon as it has been measured.

## Usage-record density

`scripts/generate-synthetic-corpus.py` writes usage records far more densely than real
logs do: with its default (unpadded) content it produces about 660 usage records per
MiB, versus roughly 130 per MiB for real Claude Code logs and roughly 40 per MiB for
real Codex logs (see docs/project/qa/scale-measurement.md). The engine's memory model
tracks usage-*record* count, not raw bytes, so the extrapolation bound in this script is
expressed and fit against `usage_records` (reported by the generator's JSON summary),
never against `--max-bytes`/input MiB. `--content-padding-bytes` on the generator lowers
a corpus's records-per-MiB density without changing its usage-record count, which is
also what makes the raw-bytes independence check possible: two corpora at very different
densities but identical usage-record counts.

Run with `uv --config-file uv.toml run --frozen python scripts/check-scale.py --binary
target/release/urollup`, or `make scale-gate`, which builds that binary first.
"""

from __future__ import annotations

import argparse
import importlib.util
import shutil
import sys
import tempfile
from dataclasses import dataclass
from pathlib import Path
from types import SimpleNamespace
from typing import Any

REPO_ROOT = Path(__file__).resolve().parents[1]
MEASURE_SCALE_PATH = REPO_ROOT / "scripts" / "measure-scale.py"


def _load_measure_scale() -> Any:
    """Loads scripts/measure-scale.py as a module.

    The hyphenated filename cannot be `import`ed directly, and this script builds on
    measure-scale.py's generator/watchdog/`/usr/bin/time` plumbing rather than
    duplicating it, per the plan's instruction to extend that harness.
    """
    spec = importlib.util.spec_from_file_location("urollup_measure_scale", MEASURE_SCALE_PATH)
    if spec is None or spec.loader is None:
        raise ImportError(f"cannot load {MEASURE_SCALE_PATH}")
    module = importlib.util.module_from_spec(spec)
    # dataclasses.dataclass() looks the defining module up in sys.modules; register
    # before exec_module or its decorators inside measure-scale.py raise AttributeError.
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


measure_scale = _load_measure_scale()

MIB = 1024 * 1024
DEFAULT_BINARY = REPO_ROOT / "target" / "release" / "urollup"
# Never generate more than this in one corpus (AGENTS.md / scale-measurement.md safety
# rule); the gate's own defaults stay far below it.
MAX_CORPUS_MIB = 256


# ---------------------------------------------------------------------------
# Pure decision logic (fitting and thresholds): unit-tested without a binary.
# ---------------------------------------------------------------------------


@dataclass
class LineFit:
    """`peak_bytes ~= intercept + slope * usage_records`."""

    slope: float
    intercept: float


@dataclass
class GateOutcome:
    ok: bool
    message: str


def parse_size_list(text: str) -> list[int]:
    """Parses a comma-separated list of corpus sizes (MiB) for the extrapolation fit.

    At least two distinct sizes are required to fit a line, and every size must respect
    the corpus-generation safety cap.
    """
    try:
        sizes = [int(part) for part in text.split(",") if part.strip()]
    except ValueError as error:
        raise ValueError(f"must be a comma-separated list of integers, got {text!r}") from error
    if len(sizes) < 2:
        raise ValueError("must name at least two sizes to fit a slope")
    if len(set(sizes)) < 2:
        raise ValueError("must name at least two distinct sizes to fit a slope")
    if any(size <= 0 for size in sizes):
        raise ValueError("sizes must be positive")
    oversized = [size for size in sizes if size > MAX_CORPUS_MIB]
    if oversized:
        raise ValueError(
            f"sizes {oversized} exceed the {MAX_CORPUS_MIB} MiB corpus-generation safety cap"
        )
    return sizes


def fit_peak_line(records: list[int], peak_bytes: list[int]) -> LineFit:
    """Ordinary least squares fit of peak memory (bytes) against usage-record count.

    The plan's Memory Model predicts peak footprint is roughly linear in the number of
    usage-bearing records, independent of raw input bytes; this is that fit.
    """
    fit = measure_scale.fit_line([float(r) for r in records], [float(p) for p in peak_bytes])
    if fit is None:
        raise ValueError("need at least two distinct usage-record counts to fit a slope")
    slope, intercept = fit
    return LineFit(slope=slope, intercept=intercept)


def evaluate_independence(
    peak_diff_bytes: float | None,
    ceiling_mib: float,
    identical_usage_records: bool,
) -> GateOutcome:
    """Judges the raw-bytes independence check (Phase 2's first scale gate)."""
    if not identical_usage_records:
        return GateOutcome(
            False,
            "usage-record counts differed between the base and padded corpora; the "
            "independence comparison is not valid (check --independence-* options)",
        )
    if peak_diff_bytes is None:
        return GateOutcome(
            False, "could not measure a peak-memory difference; see the run errors above"
        )
    ceiling_bytes = ceiling_mib * MIB
    if abs(peak_diff_bytes) > ceiling_bytes:
        return GateOutcome(
            False,
            "raw-bytes independence violated: peak memory differed by "
            f"{peak_diff_bytes / MIB:.2f} MiB between identical-usage-record corpora "
            f"with different content padding, exceeding the {ceiling_mib:.1f} MiB ceiling",
        )
    return GateOutcome(
        True,
        f"peak memory differed by {peak_diff_bytes / MIB:+.2f} MiB between "
        f"identical-usage-record corpora (ceiling {ceiling_mib:.1f} MiB)",
    )


def evaluate_extrapolation(
    fit: LineFit,
    slope_ceiling_bytes_per_record: float,
    intercept_ceiling_mib: float,
) -> GateOutcome:
    """Judges the footprint extrapolation bound (Phase 2's second scale gate)."""
    intercept_ceiling_bytes = intercept_ceiling_mib * MIB
    problems = []
    if fit.slope > slope_ceiling_bytes_per_record:
        problems.append(
            f"slope {fit.slope:.0f} B/record exceeds the "
            f"{slope_ceiling_bytes_per_record:.0f} B/record ceiling"
        )
    if fit.intercept > intercept_ceiling_bytes:
        problems.append(
            f"intercept {fit.intercept / MIB:.2f} MiB exceeds the "
            f"{intercept_ceiling_mib:.2f} MiB ceiling"
        )
    if problems:
        return GateOutcome(False, "footprint extrapolation bound violated: " + "; ".join(problems))
    return GateOutcome(
        True,
        f"peak MiB ~= {fit.intercept / MIB:.2f} + {fit.slope:.0f} B/record * records "
        f"(ceilings: {slope_ceiling_bytes_per_record:.0f} B/record, "
        f"{intercept_ceiling_mib:.2f} MiB)",
    )


def evaluate_daily_gate(run: Any, watchdog_limit_mib: int) -> GateOutcome:
    """Judges the `daily --all` watchdog gate (Phase 2's third scale gate).

    `run` is a `measure_scale.RunResult`; refusal is reported as a distinct, explained
    failure rather than folded into a generic non-zero-exit message. The engine's
    configurable per-agent row ceiling is distinct from this external RSS watchdog.
    """
    if run.refused:
        return GateOutcome(
            False,
            "engine refused the input at its configured compact-row capacity ceiling: "
            f"{run.error}; lower --daily-gate-mib so the generated corpus stays well "
            "under that limit",
        )
    if run.watchdog_killed:
        return GateOutcome(
            False,
            "daily --all was killed by the RSS watchdog: peak "
            f"{run.watchdog_peak_kib / 1024:.1f} MiB exceeded the {watchdog_limit_mib} "
            "MiB limit",
        )
    if run.exit_code != 0:
        return GateOutcome(False, f"daily --all exited {run.exit_code}: {run.error}")
    return GateOutcome(
        True,
        f"daily --all completed at peak {run.watchdog_peak_kib / 1024:.1f} MiB, under "
        f"the {watchdog_limit_mib} MiB watchdog limit",
    )


# ---------------------------------------------------------------------------
# Measurement (subprocess-driving): generates corpora, runs the binary, cleans up.
# ---------------------------------------------------------------------------


def run_independence_check(binary: Path, args: argparse.Namespace) -> tuple[GateOutcome, dict[str, Any]]:
    independence_args = SimpleNamespace(
        seed=args.seed,
        independence_days=args.independence_days,
        independence_sessions_per_day=args.independence_sessions_per_day,
        independence_rollouts_per_day=args.independence_rollouts_per_day,
        independence_base_padding_bytes=args.independence_base_padding_bytes,
        independence_padding_multiplier=args.independence_padding_multiplier,
        watchdog_limit_mib=args.calibration_watchdog_limit_mib,
    )
    result = measure_scale.measure_independence(binary, independence_args)
    for label in ("base", "padded"):
        run = result["results"][label]["run"]
        if run["refused"]:
            return (
                GateOutcome(
                    False,
                    f"engine refused the independence check's {label!r} corpus at "
                    f"its configured compact-row capacity ceiling: {run['error']}",
                ),
                result,
            )
        if run["watchdog_killed"] or run["exit_code"] != 0:
            return (
                GateOutcome(
                    False,
                    f"the independence check's {label!r} corpus run failed: {run['error']}",
                ),
                result,
            )
    outcome = evaluate_independence(
        result["peak_diff_bytes"], args.independence_ceiling_mib, result["identical_usage_records"]
    )
    return outcome, result


def measure_extrapolation_point(binary: Path, size_mib: int, args: argparse.Namespace) -> dict[str, Any]:
    """One (usage_records, peak_bytes) sample for the extrapolation fit.

    Deliberately measures only `--extrapolation-command`, not all three CLI commands
    (unlike `measure_scale.measure_one_size`), so the fit stays cheap: three commands at
    three sizes would triple the gate's runtime for no extra signal.
    """
    tmp_dir = Path(tempfile.mkdtemp(prefix="urollup-scale-gate-extrap-"))
    try:
        corpus_dir = tmp_dir / "corpus"
        summary = measure_scale.run_generator(
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
        run = measure_scale.run_measured(
            binary,
            measure_scale.urollup_args(claude_src, codex_src, args.extrapolation_command),
            args.calibration_watchdog_limit_mib,
            tmp_dir,
            f"extrap-{size_mib}mib",
        )
        return {"input_mib": size_mib, "usage_records": summary["usage_records"], "run": run}
    finally:
        if not args.keep_temp:
            shutil.rmtree(tmp_dir, ignore_errors=True)


def run_extrapolation_check(binary: Path, args: argparse.Namespace) -> tuple[GateOutcome, dict[str, Any]]:
    points = [measure_extrapolation_point(binary, size, args) for size in args.extrapolation_sizes]
    for point in points:
        run = point["run"]
        if run.refused:
            return (
                GateOutcome(
                    False,
                    f"engine refused the {point['input_mib']} MiB extrapolation corpus "
                    f"at its configured compact-row capacity ceiling: {run.error}",
                ),
                {"points": points},
            )
        if run.watchdog_killed or run.exit_code != 0:
            return (
                GateOutcome(
                    False,
                    f"the {point['input_mib']} MiB extrapolation run failed: {run.error}",
                ),
                {"points": points},
            )
    records = [point["usage_records"] for point in points]
    peaks = [point["run"].peak_bytes for point in points]
    if any(peak is None for peak in peaks):
        return GateOutcome(False, "could not read peak memory for an extrapolation run"), {
            "points": points
        }
    fit = fit_peak_line(records, peaks)
    outcome = evaluate_extrapolation(
        fit, args.extrapolation_slope_ceiling_bytes_per_record, args.extrapolation_intercept_ceiling_mib
    )
    return outcome, {"points": points, "fit": fit}


def run_daily_watchdog_gate(binary: Path, args: argparse.Namespace) -> tuple[GateOutcome, dict[str, Any]]:
    tmp_dir = Path(tempfile.mkdtemp(prefix="urollup-scale-gate-daily-"))
    try:
        corpus_dir = tmp_dir / "corpus"
        summary = measure_scale.run_generator(
            corpus_dir,
            [
                "--seed",
                str(args.seed),
                "--max-bytes",
                str(args.daily_gate_mib * MIB),
                "--claude-fraction",
                str(args.claude_fraction),
            ],
        )
        claude_src = corpus_dir / "claude" / "projects"
        codex_src = corpus_dir / "codex"
        run = measure_scale.run_measured(
            binary,
            measure_scale.urollup_args(claude_src, codex_src, "daily"),
            args.watchdog_limit_mib,
            tmp_dir,
            "daily-gate",
        )
        outcome = evaluate_daily_gate(run, args.watchdog_limit_mib)
        return outcome, {"summary": summary, "run": run}
    finally:
        if not args.keep_temp:
            shutil.rmtree(tmp_dir, ignore_errors=True)


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--binary", type=Path, default=DEFAULT_BINARY, help="urollup release binary to gate (default: target/release/urollup)")
    parser.add_argument("--seed", type=int, default=1, help="generator seed (default: 1)")
    parser.add_argument("--claude-fraction", type=float, default=0.2, help="share of each corpus given to Claude content (default: 0.2)")
    parser.add_argument("--keep-temp", action="store_true", help="keep generated corpora and run artifacts instead of deleting them (extrapolation and daily-gate steps only)")

    independence = parser.add_argument_group("raw-bytes independence")
    independence.add_argument("--independence-days", type=int, default=8)
    independence.add_argument("--independence-sessions-per-day", type=int, default=16)
    independence.add_argument("--independence-rollouts-per-day", type=int, default=16)
    independence.add_argument("--independence-base-padding-bytes", type=int, default=500)
    independence.add_argument("--independence-padding-multiplier", type=int, default=50)
    independence.add_argument(
        "--independence-ceiling-mib",
        type=float,
        default=64.0,
        help="max allowed peak-memory difference between identical-usage-record "
        "corpora with different content padding (default: 64, the plan's own bound)",
    )

    extrapolation = parser.add_argument_group("footprint extrapolation")
    extrapolation.add_argument(
        "--extrapolation-sizes-mib",
        type=str,
        default="4,8,16",
        help="comma-separated corpus sizes (MiB) used to fit peak memory against "
        "usage-record count (default: 4,8,16)",
    )
    extrapolation.add_argument(
        "--extrapolation-command",
        choices=["sessions", "daily", "report"],
        default="report",
        help="which urollup command to measure at each extrapolation size (default: report)",
    )
    extrapolation.add_argument(
        "--extrapolation-slope-ceiling-bytes-per-record",
        type=float,
        default=8192.0,
        help="ceiling on the fitted slope, in bytes of peak memory per usage record "
        "(default: 8192, about 2.7x the current engine's measured slope; see "
        "docs/project/qa/scale-measurement.md for how to recalibrate)",
    )
    extrapolation.add_argument(
        "--extrapolation-intercept-ceiling-mib",
        type=float,
        default=48.0,
        help="ceiling on the fitted intercept, the fixed baseline footprint, in MiB "
        "(default: 48, about 5x the current engine's measured intercept)",
    )

    daily = parser.add_argument_group("daily --all watchdog gate")
    daily.add_argument("--daily-gate-mib", type=int, default=32, help="corpus size (MiB) for the `daily --all` watchdog gate (default: 32)")
    daily.add_argument(
        "--watchdog-limit-mib",
        type=int,
        default=512,
        help="RSS watchdog kill-switch limit for the daily --all gate (default: 512, "
        "matching the plan's whole-history footprint goal)",
    )
    daily.add_argument(
        "--calibration-watchdog-limit-mib",
        type=int,
        default=2048,
        help="a looser watchdog kill-switch for the independence and extrapolation "
        "runs, whose corpora are much smaller than the daily gate's but still want a "
        "backstop (default: 2048)",
    )

    args = parser.parse_args(argv)

    try:
        args.extrapolation_sizes = parse_size_list(args.extrapolation_sizes_mib)
    except ValueError as error:
        parser.error(f"--extrapolation-sizes-mib {error}")
    if args.daily_gate_mib <= 0:
        parser.error("--daily-gate-mib must be positive")
    if args.daily_gate_mib > MAX_CORPUS_MIB:
        parser.error(
            f"--daily-gate-mib must be <= {MAX_CORPUS_MIB} (the corpus-generation safety cap)"
        )
    if args.independence_base_padding_bytes <= 0:
        parser.error("--independence-base-padding-bytes must be positive")
    if args.independence_padding_multiplier <= 1:
        parser.error("--independence-padding-multiplier must be greater than 1")
    return args


def print_result(name: str, outcome: GateOutcome) -> None:
    status = "PASS" if outcome.ok else "FAIL"
    print(f"[{status}] {name}: {outcome.message}")


def main(argv: list[str] | None = None) -> int:
    args = parse_args(sys.argv[1:] if argv is None else argv)
    binary = args.binary
    if not binary.exists():
        print(
            f"error: binary not found: {binary}; build it first with "
            "`cargo build --locked --release -p urollup` (or run `make scale-gate`, "
            "which does that automatically)",
            file=sys.stderr,
        )
        return 1

    print(f"scale gate: measuring {binary}", file=sys.stderr)
    outcomes: list[tuple[str, GateOutcome]] = []

    print("running raw-bytes independence check...", file=sys.stderr)
    independence_outcome, independence_detail = run_independence_check(binary, args)
    outcomes.append(("raw-bytes independence", independence_outcome))
    print_result("raw-bytes independence", independence_outcome)
    if "results" in independence_detail:
        for label in ("base", "padded"):
            entry = independence_detail["results"][label]
            run = entry["run"]
            peak = f"{run['peak_bytes'] / MIB:.2f} MiB" if run["peak_bytes"] is not None else "n/a"
            print(
                f"    {label:>6}: {entry['padding_bytes']} B/field padding, "
                f"{entry['usage_records']} usage records, peak {peak}",
                file=sys.stderr,
            )

    print("running footprint extrapolation check...", file=sys.stderr)
    extrapolation_outcome, extrapolation_detail = run_extrapolation_check(binary, args)
    outcomes.append(("footprint extrapolation", extrapolation_outcome))
    print_result("footprint extrapolation", extrapolation_outcome)
    for point in extrapolation_detail.get("points", []):
        peak = point["run"].peak_bytes
        peak_display = f"{peak / MIB:.2f} MiB" if peak is not None else "n/a"
        print(
            f"    {point['input_mib']:>4} MiB corpus: {point['usage_records']:>6} "
            f"usage records, peak {peak_display}",
            file=sys.stderr,
        )

    print("running daily --all watchdog gate...", file=sys.stderr)
    daily_outcome, daily_detail = run_daily_watchdog_gate(binary, args)
    outcomes.append(("daily --all watchdog", daily_outcome))
    print_result("daily --all watchdog", daily_outcome)
    if "summary" in daily_detail:
        print(
            f"    {args.daily_gate_mib} MiB corpus: "
            f"{daily_detail['summary']['usage_records']} usage records",
            file=sys.stderr,
        )

    failed = [name for name, outcome in outcomes if not outcome.ok]
    if failed:
        print(f"\nscale gate FAILED: {', '.join(failed)}", file=sys.stderr)
        return 1
    print("\nscale gate passed", file=sys.stderr)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
