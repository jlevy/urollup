#!/usr/bin/env python3
"""Compare privacy-safe local aggregates from urollup and pinned ccusage."""

from __future__ import annotations

import argparse
import json
import os
import platform
import subprocess
import sys
import tempfile
from datetime import date, datetime, timedelta
from pathlib import Path
from typing import Any
from zoneinfo import ZoneInfo, ZoneInfoNotFoundError

import compare
import local_aggregate

METRICS = compare.CODEX_METRICS
HISTOGRAM_KEYS = ("exact", "within_1_percent", "within_5_percent", "over_5_percent")


class LocalDiffError(RuntimeError):
    """The local comparison failed without exposing private source data."""


def zero_metrics() -> dict[str, int]:
    """Create one complete, fixed metric record."""

    return {metric: 0 for metric in METRICS}


def add_metrics(left: dict[str, int], right: dict[str, int]) -> dict[str, int]:
    """Add two normalized metric records."""

    return {metric: left.get(metric, 0) + right.get(metric, 0) for metric in METRICS}


def subtract_metrics(left: dict[str, int], right: dict[str, int]) -> dict[str, int]:
    """Subtract ccusage metrics from urollup metrics."""

    return {metric: left.get(metric, 0) - right.get(metric, 0) for metric in METRICS}


def safe_run(command: list[str], *, cwd: Path, allow_failure: bool = False) -> dict[str, Any] | None:
    """Run a tool without a shell, returning no private diagnostics on failure."""

    environment = dict(os.environ)
    environment.update({"NO_COLOR": "1", "FORCE_COLOR": "0", "LOG_LEVEL": "0"})
    completed = subprocess.run(  # noqa: S603 - reviewed executables are supplied explicitly
        command,
        cwd=cwd,
        env=environment,
        check=False,
        capture_output=True,
        text=True,
        encoding="utf-8",
    )
    if completed.returncode != 0:
        if allow_failure:
            return None
        raise LocalDiffError("a comparison command failed")
    try:
        payload = json.loads(completed.stdout)
    except json.JSONDecodeError as error:
        raise LocalDiffError("a comparison command returned invalid JSON") from error
    if not isinstance(payload, dict):
        raise LocalDiffError("a comparison command returned non-object JSON")
    return payload


def urollup_command(binary: Path, command: str, timezone: str, extra: list[str] | None = None) -> list[str]:
    """Build one noninteractive urollup invocation over default roots."""

    return [
        str(binary),
        command,
        *(extra or ["--all"]),
        "--scope",
        "self",
        "--format",
        "json",
        "--timezone",
        timezone,
        "--color",
        "never",
        "--no-progress",
    ]


def ccusage_payload(
    binary: Path, *, agent: str, view: str, timezone: str, until: date, cwd: Path
) -> dict[str, Any]:
    """Run one pinned ccusage path through the stable half-open interval."""

    payload = safe_run(
        [
            str(binary),
            agent,
            view,
            "--offline",
            "--json",
            "--timezone",
            timezone,
            "--until",
            until.strftime("%Y%m%d"),
            "--no-color",
        ],
        cwd=cwd,
    )
    if payload is None:
        raise LocalDiffError("ccusage produced no comparison payload")
    return payload


def daily_rows(payload: dict[str, Any], *, urollup: bool, cutoff: str) -> dict[str, dict[str, int]]:
    """Normalize complete daily rows while retaining only safe date keys."""

    if urollup:
        return {
            row["date"]: compare.metrics_from_urollup(row, reasoning=True)
            for row in local_aggregate.stable_daily_rows(payload, cutoff)
        }
    rows = compare.parse_ccusage_rows(
        payload, view="daily", reasoning=True, expected_threads=[]
    )
    return {day: metrics for day, metrics in rows.items() if day < cutoff}


def combine_rows(*groups: dict[str, dict[str, int]]) -> dict[str, dict[str, int]]:
    """Combine keyed metric rows from several agents."""

    combined: dict[str, dict[str, int]] = {}
    for rows in groups:
        for key, metrics in rows.items():
            combined[key] = add_metrics(combined.get(key, zero_metrics()), metrics)
    return combined


def total_rows(rows: dict[str, dict[str, int]]) -> dict[str, int]:
    """Sum normalized rows."""

    total = zero_metrics()
    for metrics in rows.values():
        total = add_metrics(total, metrics)
    return total


def safe_day_deltas(
    urollup: dict[str, dict[str, int]], ccusage: dict[str, dict[str, int]]
) -> list[dict[str, Any]]:
    """Report date-keyed deltas and no source-derived identifiers."""

    return [
        {
            "date": day,
            "delta": subtract_metrics(
                urollup.get(day, zero_metrics()), ccusage.get(day, zero_metrics())
            ),
        }
        for day in sorted(set(urollup) | set(ccusage))
    ]


def histogram_bucket(left: dict[str, int], right: dict[str, int]) -> str:
    """Classify one session by relative total-token delta."""

    delta = abs(left.get("total", 0) - right.get("total", 0))
    denominator = max(right.get("total", 0), 1)
    if delta == 0:
        return "exact"
    if delta * 100 <= denominator:
        return "within_1_percent"
    if delta * 20 <= denominator:
        return "within_5_percent"
    return "over_5_percent"


def stable_urollup_threads(
    binary: Path,
    sessions: dict[str, Any],
    *,
    timezone: str,
    cutoff: str,
    cwd: Path,
) -> set[str]:
    """Identify stable analytical threads without returning them in the report."""

    rows = sessions.get("rows")
    if not isinstance(rows, list):
        raise LocalDiffError("urollup sessions output has no rows")
    stable = set()
    for row in rows:
        if not isinstance(row, dict) or not isinstance(row.get("thread"), str):
            continue
        thread = row["thread"]
        daily = safe_run(
            urollup_command(binary, "daily", timezone, ["--session", thread]), cwd=cwd
        )
        if daily is None:
            continue
        days = daily.get("rows")
        if not isinstance(days, list):
            raise LocalDiffError("urollup per-session output has no rows")
        dates = [item.get("date") for item in days if isinstance(item, dict)]
        if dates and all(isinstance(day, str) and day < cutoff for day in dates):
            stable.add(thread)
    return stable


def compare_sessions(
    *,
    urollup: Path,
    ccusage: Path,
    timezone: str,
    until: date,
    cutoff: str,
    cwd: Path,
) -> tuple[dict[str, int], dict[str, int]]:
    """Match sessions in memory and return only counts plus a delta histogram."""

    all_sessions = safe_run(urollup_command(urollup, "sessions", timezone), cwd=cwd)
    if all_sessions is None:
        raise LocalDiffError("urollup produced no sessions payload")
    stable_threads = stable_urollup_threads(
        urollup, all_sessions, timezone=timezone, cutoff=cutoff, cwd=cwd
    )
    matched_threads: set[str] = set()
    counts = {"matched": 0, "urollup_only": 0, "ccusage_only": 0}
    histogram = {key: 0 for key in HISTOGRAM_KEYS}
    for agent in ("claude", "codex"):
        payload = ccusage_payload(
            ccusage, agent=agent, view="session", timezone=timezone, until=until, cwd=cwd
        )
        rows = compare.parse_ccusage_rows(
            payload, view="session", reasoning=agent == "codex", expected_threads=[]
        )
        for native_id, right in rows.items():
            result = safe_run(
                urollup_command(urollup, "sessions", timezone, ["--session", native_id]),
                cwd=cwd,
                allow_failure=True,
            )
            if result is None:
                counts["ccusage_only"] += 1
                continue
            result_rows = result.get("rows")
            if not isinstance(result_rows, list) or not result_rows:
                counts["ccusage_only"] += 1
                continue
            left = zero_metrics()
            returned_threads = set()
            for row in result_rows:
                if not isinstance(row, dict):
                    raise LocalDiffError("urollup returned an invalid session row")
                left = add_metrics(
                    left, compare.metrics_from_urollup(row, reasoning=agent == "codex")
                )
                if isinstance(row.get("thread"), str):
                    returned_threads.add(row["thread"])
            # `sessions` reports whole-session totals and has no interval flag in 0.1.
            # Exclude a match unless every returned thread is known to end before the
            # cutoff; otherwise it would be compared with ccusage's cutoff total.
            if not returned_threads or not returned_threads.issubset(stable_threads):
                continue
            matched_threads.update(returned_threads)
            counts["matched"] += 1
            histogram[histogram_bucket(left, right)] += 1
    counts["urollup_only"] = len(stable_threads - matched_threads)
    return counts, histogram


def build_diff(
    *,
    urollup_version: str,
    ccusage_version: str,
    platform_name: str,
    timezone: str,
    cutoff: str,
    urollup_days: dict[str, dict[str, int]],
    ccusage_days: dict[str, dict[str, int]],
    session_counts: dict[str, int],
    histogram: dict[str, int],
) -> dict[str, Any]:
    """Build the fixed, privacy-safe local parity record."""

    left = total_rows(urollup_days)
    right = total_rows(ccusage_days)
    delta = subtract_metrics(left, right)
    review = {
        metric: abs(value) * 1000 > max(right.get(metric, 0), 1)
        for metric, value in delta.items()
    }
    return {
        "format": "urollup.local-parity/v1",
        "tools": {"urollup": urollup_version, "ccusage": ccusage_version},
        "platform": platform_name,
        "timezone": timezone,
        "interval": {"start": None, "until_exclusive": cutoff},
        "totals": {"urollup": left, "ccusage": right, "delta": delta},
        "per_day": safe_day_deltas(urollup_days, ccusage_days),
        "per_model": [{"model": "other", "delta": delta}],
        "sessions": {**session_counts, "relative_delta_histogram": histogram},
        "explained": [],
        "unexplained_residual": delta,
        "review_threshold_exceeded": review,
    }


def arguments(argv: list[str]) -> argparse.Namespace:
    """Parse the maintainer-only comparison interface."""

    parser = argparse.ArgumentParser()
    parser.add_argument("--urollup", type=Path, required=True)
    parser.add_argument("--ccusage-package", type=Path, required=True)
    parser.add_argument("--timezone")
    parser.add_argument("--output", type=Path)
    parser.add_argument("--consent-local-logs", action="store_true", required=True)
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    """Run both tools on complete local days and emit the aggregate diff."""

    options = arguments(sys.argv[1:] if argv is None else argv)
    timezone = options.timezone or local_aggregate.system_timezone()
    try:
        zone = ZoneInfo(timezone)
    except ZoneInfoNotFoundError as error:
        raise LocalDiffError("the requested timezone is not an IANA zone") from error
    cutoff_date = datetime.now(zone).date()
    cutoff = cutoff_date.isoformat()
    until = cutoff_date - timedelta(days=1)
    urollup = options.urollup.resolve()
    if not urollup.is_file():
        raise LocalDiffError("urollup binary is missing")
    try:
        ccusage = compare.native_ccusage_binary(options.ccusage_package.resolve())
        compare.prepare_ccusage_binary(ccusage)
    except compare.ParityError as error:
        raise LocalDiffError("the pinned ccusage binary is unavailable") from error

    with tempfile.TemporaryDirectory(prefix="urollup-local-parity-") as temporary:
        cwd = Path(temporary)
        urollup_daily = safe_run(urollup_command(urollup, "daily", timezone), cwd=cwd)
        if urollup_daily is None:
            raise LocalDiffError("urollup produced no daily payload")
        urollup_days = daily_rows(urollup_daily, urollup=True, cutoff=cutoff)
        ccusage_days = combine_rows(
            *(
                daily_rows(
                    ccusage_payload(
                        ccusage,
                        agent=agent,
                        view="daily",
                        timezone=timezone,
                        until=until,
                        cwd=cwd,
                    ),
                    urollup=False,
                    cutoff=cutoff,
                )
                for agent in ("claude", "codex")
            )
        )
        session_counts, histogram = compare_sessions(
            urollup=urollup,
            ccusage=ccusage,
            timezone=timezone,
            until=until,
            cutoff=cutoff,
            cwd=cwd,
        )

    safe = build_diff(
        urollup_version=local_aggregate.version(urollup),
        ccusage_version=f"ccusage {compare.CCUSAGE_VERSION}",
        platform_name=f"{platform.system().lower()}-{platform.machine().lower()}",
        timezone=timezone,
        cutoff=cutoff,
        urollup_days=urollup_days,
        ccusage_days=ccusage_days,
        session_counts=session_counts,
        histogram=histogram,
    )
    rendered = json.dumps(safe, indent=2, sort_keys=True) + "\n"
    if options.output is None:
        sys.stdout.write(rendered)
    else:
        options.output.parent.mkdir(parents=True, exist_ok=True)
        options.output.write_text(rendered, encoding="utf-8")
        print("local parity aggregate written")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (LocalDiffError, local_aggregate.LocalAggregateError) as error:
        print(f"parity-local: {error}", file=sys.stderr)
        raise SystemExit(1) from error
