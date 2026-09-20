#!/usr/bin/env python3
"""Compare privacy-safe local aggregates from urollup and pinned ccusage."""

from __future__ import annotations

import argparse
import json
import os
import platform
import re
import subprocess
import sys
import tempfile
from dataclasses import dataclass
from datetime import date, datetime, timedelta
from pathlib import Path
from typing import Any
from zoneinfo import ZoneInfo, ZoneInfoNotFoundError

import compare
import local_aggregate

METRICS = compare.CODEX_METRICS
HISTOGRAM_KEYS = ("exact", "within_1_percent", "within_5_percent", "over_5_percent")
AGENTS = ("claude", "codex")
CODEX_THREAD_ID = re.compile(
    r"([0-9A-Fa-f]{8}-[0-9A-Fa-f]{4}-[0-9A-Fa-f]{4}-[0-9A-Fa-f]{4}-[0-9A-Fa-f]{12})"
    r"(?:\.jsonl(?:\.zst)?)?$"
)


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


def safe_run(command: list[str], *, cwd: Path) -> dict[str, Any]:
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
        raise LocalDiffError("a comparison command failed")
    try:
        payload = json.loads(completed.stdout)
    except json.JSONDecodeError as error:
        raise LocalDiffError("a comparison command returned invalid JSON") from error
    if not isinstance(payload, dict):
        raise LocalDiffError("a comparison command returned non-object JSON")
    return payload


def urollup_command(binary: Path, command: str, timezone: str) -> list[str]:
    """Build one noninteractive whole-history urollup invocation over default roots."""

    return [
        str(binary),
        command,
        "--all",
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
    binary: Path, *, agent: str, view: str, timezone: str, until: date | None, cwd: Path
) -> dict[str, Any]:
    """Run one pinned ccusage path, through the inclusive `until` date when one is given."""

    interval = ["--until", until.strftime("%Y%m%d")] if until is not None else []
    return safe_run(
        [
            str(binary),
            agent,
            view,
            "--offline",
            "--json",
            "--timezone",
            timezone,
            *interval,
            "--no-color",
        ],
        cwd=cwd,
    )


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


@dataclass
class CcusageSession:
    """One ccusage session's whole-session metrics, keyed by native ID in memory only."""

    metrics: dict[str, int]
    stable: bool


def native_session_key(agent: str, value: Any) -> str | None:
    """Map a ccusage session ID to the native ID urollup reports in `session`.

    Claude session IDs are native. ccusage keys a Codex session by its rollout path, such
    as `2026/09/09/rollout-2026-09-09T08-00-00-<thread>`, so the trailing thread ID is
    the native key; a path without one stays as it is and joins nothing.
    """

    if not isinstance(value, str) or not value:
        return None
    if agent == "codex":
        match = CODEX_THREAD_ID.search(value.rsplit("/", 1)[-1])
        if match is not None:
            return match.group(1)
    return value


def last_activity_date(value: Any, zone: ZoneInfo) -> str | None:
    """Return a ccusage last-activity instant's calendar date in the report timezone."""

    if not isinstance(value, str):
        return None
    try:
        moment = datetime.fromisoformat(value.replace("Z", "+00:00"))
    except ValueError:
        return None
    if moment.tzinfo is None:
        return None
    return moment.astimezone(zone).date().isoformat()


def ccusage_sessions(
    payload: dict[str, Any], *, agent: str, zone: ZoneInfo, cutoff: str
) -> dict[str, CcusageSession]:
    """Read whole-session ccusage rows by native ID, with their stability at the cutoff.

    A session is stable when its last activity falls on a local day before the cutoff, so
    its whole-session total is complete. Rows for one native ID, such as one Codex rollout
    found at two locations, sum as `compare.parse_ccusage_rows` sums them, and are stable
    only when every row is.
    """

    rows = payload.get("sessions")
    if not isinstance(rows, list):
        raise LocalDiffError("ccusage session output has no sessions")
    sessions: dict[str, CcusageSession] = {}
    for row in rows:
        if not isinstance(row, dict):
            raise LocalDiffError("ccusage returned an invalid session row")
        key = native_session_key(agent, row.get("sessionId"))
        if key is None:
            raise LocalDiffError("ccusage returned a session row without a session ID")
        try:
            metrics = compare.metrics_from_ccusage(row, reasoning=agent == "codex")
        except compare.ParityError as error:
            raise LocalDiffError("ccusage returned an invalid session row") from error
        day = last_activity_date(row.get("lastActivity"), zone)
        stable = day is not None and day < cutoff
        existing = sessions.get(key)
        if existing is None:
            sessions[key] = CcusageSession(metrics, stable)
        else:
            sessions[key] = CcusageSession(
                add_metrics(existing.metrics, metrics), existing.stable and stable
            )
    return sessions


def join_sessions(
    urollup_sessions: dict[str, Any], ccusage_by_agent: dict[str, dict[str, CcusageSession]]
) -> tuple[dict[str, int], dict[str, int]]:
    """Join whole-history session rows on native IDs and return only counts and a histogram.

    urollup's `sessions` rows are whole-session totals without dates, so ccusage's last
    activity decides which sessions ended before the cutoff; active sessions are left out
    of every count. A urollup row that joins no ccusage session counts as urollup-only
    when it has requests, unless it is a Claude subagent whose parent session is active,
    since ccusage folds subagents into the parent row. Other rows without a join, such as
    inline sidechains, carry no activity date of their own and still count.
    """

    rows = urollup_sessions.get("rows")
    if not isinstance(rows, list):
        raise LocalDiffError("urollup sessions output has no rows")
    counts = {"matched": 0, "urollup_only": 0, "ccusage_only": 0}
    histogram = {key: 0 for key in HISTOGRAM_KEYS}
    joined: dict[str, set[str]] = {agent: set() for agent in ccusage_by_agent}
    for row in rows:
        if not isinstance(row, dict):
            raise LocalDiffError("urollup returned an invalid session row")
        if not isinstance(row.get("thread"), str):
            continue  # The unowned group is no session.
        agent = row.get("agent")
        native = row.get("session")
        sessions = ccusage_by_agent.get(agent, {}) if isinstance(agent, str) else {}
        match = sessions.get(native) if isinstance(native, str) else None
        if match is not None:
            joined[agent].add(native)
            if not match.stable:
                continue
            try:
                left = compare.metrics_from_urollup(row, reasoning=agent == "codex")
            except compare.ParityError as error:
                raise LocalDiffError("urollup returned an invalid session row") from error
            counts["matched"] += 1
            histogram[histogram_bucket(left, match.metrics)] += 1
            continue
        if sum(local_aggregate.request_counts(row).values()) == 0:
            continue  # No dated usage, and ccusage drops zero-token sessions.
        if agent == "claude" and isinstance(native, str) and "/" in native:
            parent = sessions.get(native.split("/", 1)[0])
            if parent is not None and not parent.stable:
                continue
        counts["urollup_only"] += 1
    for agent, sessions in ccusage_by_agent.items():
        counts["ccusage_only"] += sum(
            1
            for key, session in sessions.items()
            if session.stable and key not in joined[agent]
        )
    return counts, histogram


def compare_sessions(
    *,
    urollup: Path,
    ccusage: Path,
    timezone: str,
    cutoff: str,
    cwd: Path,
) -> tuple[dict[str, int], dict[str, int]]:
    """Run whole-history sessions once per tool and join them in memory."""

    all_sessions = safe_run(urollup_command(urollup, "sessions", timezone), cwd=cwd)
    zone = ZoneInfo(timezone)
    ccusage_by_agent = {
        agent: ccusage_sessions(
            ccusage_payload(
                ccusage, agent=agent, view="session", timezone=timezone, until=None, cwd=cwd
            ),
            agent=agent,
            zone=zone,
            cutoff=cutoff,
        )
        for agent in AGENTS
    }
    return join_sessions(all_sessions, ccusage_by_agent)


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
                for agent in AGENTS
            )
        )
        session_counts, histogram = compare_sessions(
            urollup=urollup,
            ccusage=ccusage,
            timezone=timezone,
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
    except OSError:
        # OS exceptions carry private paths; keep launch and publication failures safe.
        print("parity-local: operating-system operation failed", file=sys.stderr)
        raise SystemExit(1) from None
