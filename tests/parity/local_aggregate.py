#!/usr/bin/env python3
"""Produce a privacy-safe aggregate report over consented local agent logs."""

from __future__ import annotations

import argparse
import json
import os
import platform
import re
import subprocess
import sys
from collections.abc import Callable
from datetime import datetime
from pathlib import Path
from typing import Any
from zoneinfo import ZoneInfo, ZoneInfoNotFoundError

METRICS = (
    "uncached_input",
    "cache_read",
    "cache_write",
    "output",
    "reasoning",
    "provider_only",
    "total",
)
AGENTS = {"claude", "codex", "unknown"}
VERSION_PATTERN = re.compile(r"^urollup [0-9]+\.[0-9]+\.[0-9]+(?:[-+][0-9A-Za-z.-]+)?$")


class LocalAggregateError(RuntimeError):
    """A local aggregate could not be produced without risking private output."""


Runner = Callable[[list[str]], dict[str, Any]]


def integer(record: dict[str, Any], key: str) -> int:
    """Read a non-negative count, treating an absent or null count as zero."""

    value = record.get(key, 0)
    if value is None:
        return 0
    if isinstance(value, bool) or not isinstance(value, int) or value < 0:
        raise LocalAggregateError(f"invalid aggregate field {key}")
    return value


def token_counts(row: dict[str, Any]) -> dict[str, int]:
    """Copy only the fixed token metric allowlist from one tool row."""

    tokens = row.get("tokens")
    if not isinstance(tokens, dict):
        raise LocalAggregateError("tool output lacks token aggregates")
    return {metric: integer(tokens, metric) for metric in METRICS}


def add_counts(left: dict[str, int], right: dict[str, int]) -> dict[str, int]:
    """Add two fixed metric records."""

    return {metric: left.get(metric, 0) + right.get(metric, 0) for metric in METRICS}


def stable_daily_rows(payload: dict[str, Any], cutoff: str) -> list[dict[str, Any]]:
    """Keep complete local days before the half-open cutoff."""

    rows = payload.get("rows")
    if not isinstance(rows, list):
        raise LocalAggregateError("daily output has no rows")
    stable = []
    for row in rows:
        if not isinstance(row, dict):
            raise LocalAggregateError("daily output contains an invalid row")
        day = row.get("date")
        if isinstance(day, str) and day < cutoff:
            stable.append(row)
    return stable


def request_counts(row: dict[str, Any]) -> dict[str, int]:
    """Copy only the fixed request ownership counters."""

    requests = row.get("requests")
    if not isinstance(requests, dict):
        raise LocalAggregateError("tool output lacks request aggregates")
    return {name: integer(requests, name) for name in ("owned", "ambiguous", "unknown")}


def add_requests(left: dict[str, int], right: dict[str, int]) -> dict[str, int]:
    """Add two ownership records."""

    return {name: left.get(name, 0) + right.get(name, 0) for name in right}


def safe_diagnostics(payload: dict[str, Any]) -> dict[str, Any]:
    """Return internal diagnostic codes and counts, never free-form details."""

    diagnostics = payload.get("diagnostics", [])
    if not isinstance(diagnostics, list):
        raise LocalAggregateError("tool output has invalid diagnostics")
    by_code: dict[str, int] = {}
    for diagnostic in diagnostics:
        if not isinstance(diagnostic, dict):
            raise LocalAggregateError("tool output has an invalid diagnostic")
        code = diagnostic.get("code")
        if not isinstance(code, str) or re.fullmatch(r"[a-z0-9_.-]+", code) is None:
            raise LocalAggregateError("tool output has an unsafe diagnostic code")
        by_code[code] = by_code.get(code, 0) + integer(diagnostic, "count")
    return {"count": sum(by_code.values()), "by_code": dict(sorted(by_code.items()))}


def safe_coverage(report: dict[str, Any]) -> dict[str, Any]:
    """Copy the fixed coverage contract without any evidence or source fields."""

    coverage = report.get("coverage")
    totals = report.get("totals")
    if not isinstance(coverage, dict) or not isinstance(totals, dict):
        raise LocalAggregateError("report output lacks coverage aggregates")
    unresolved = totals.get("unresolved")
    possible = totals.get("possible")
    if not isinstance(unresolved, dict) or not isinstance(possible, dict):
        raise LocalAggregateError("report output lacks side aggregates")
    return {
        "complete": coverage.get("complete") is True,
        "copies_excluded": integer(coverage, "copies_excluded"),
        "limit_observations": integer(coverage, "limit_observations"),
        "requests_without_usage": integer(coverage, "requests_without_usage"),
        "unresolved_requests": integer(unresolved, "requests"),
        "possible_requests": integer(possible, "requests"),
    }


def stable_session_summary(
    sessions: dict[str, Any], *, cutoff: str, timezone: str, run_urollup: Runner
) -> dict[str, Any]:
    """Count complete sessions while keeping their identifiers in memory only."""

    rows = sessions.get("rows")
    if not isinstance(rows, list):
        raise LocalAggregateError("sessions output has no rows")
    stable = 0
    active_or_unknown = 0
    by_agent = {agent: 0 for agent in sorted(AGENTS)}
    for row in rows:
        if not isinstance(row, dict):
            raise LocalAggregateError("sessions output contains an invalid row")
        thread = row.get("thread")
        agent = row.get("agent")
        if not isinstance(thread, str) or agent not in AGENTS:
            active_or_unknown += 1
            continue
        daily = run_urollup(
            [
                "daily",
                "--session",
                thread,
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
        )
        days = daily.get("rows")
        if not isinstance(days, list):
            raise LocalAggregateError("per-session daily output has no rows")
        dated = [item.get("date") for item in days if isinstance(item, dict)]
        if dated and all(isinstance(day, str) and day < cutoff for day in dated):
            stable += 1
            by_agent[agent] += 1
        else:
            active_or_unknown += 1
    return {
        "stable": stable,
        "active_or_unknown_excluded": active_or_unknown,
        "stable_by_agent": by_agent,
    }


def build_aggregate(
    *,
    daily: dict[str, Any],
    report: dict[str, Any],
    sessions: dict[str, Any],
    version: str,
    timezone: str,
    cutoff: str,
    platform_name: str,
    session_summary: dict[str, Any],
) -> dict[str, Any]:
    """Build the complete allowlisted output record."""

    if VERSION_PATTERN.fullmatch(version) is None:
        raise LocalAggregateError("urollup returned an invalid version")
    rows = stable_daily_rows(daily, cutoff)
    totals = {metric: 0 for metric in METRICS}
    requests = {name: 0 for name in ("owned", "ambiguous", "unknown")}
    per_day = []
    for row in rows:
        tokens = token_counts(row)
        owned = request_counts(row)
        totals = add_counts(totals, tokens)
        requests = add_requests(requests, owned)
        per_day.append({"date": row["date"], "requests": owned, "tokens": tokens})
    return {
        "format": "urollup.local-aggregate/v1",
        "tool": version,
        "platform": platform_name,
        "timezone": timezone,
        "interval": {"start": None, "until_exclusive": cutoff},
        "totals": {"requests": requests, "tokens": totals},
        "per_day": per_day,
        "sessions": session_summary,
        "coverage": safe_coverage(report),
        "diagnostics": safe_diagnostics(report),
    }


def execute_json(binary: Path, args: list[str]) -> dict[str, Any]:
    """Run urollup without a shell and return JSON or a path-free error."""

    environment = dict(os.environ)
    environment.update({"NO_COLOR": "1", "FORCE_COLOR": "0"})
    completed = subprocess.run(  # noqa: S603 - the maintainer supplies this binary
        [str(binary), *args],
        check=False,
        capture_output=True,
        text=True,
        encoding="utf-8",
        env=environment,
    )
    if completed.returncode != 0:
        raise LocalAggregateError("urollup could not produce a local aggregate")
    try:
        payload = json.loads(completed.stdout)
    except json.JSONDecodeError as error:
        raise LocalAggregateError("urollup returned invalid JSON") from error
    if not isinstance(payload, dict):
        raise LocalAggregateError("urollup returned a non-object JSON document")
    return payload


def version(binary: Path) -> str:
    """Read the binary version without forwarding its diagnostics."""

    completed = subprocess.run(  # noqa: S603 - the maintainer supplies this binary
        [str(binary), "--version"],
        check=False,
        capture_output=True,
        text=True,
        encoding="utf-8",
        env={**os.environ, "NO_COLOR": "1", "FORCE_COLOR": "0"},
    )
    value = completed.stdout.strip()
    if completed.returncode != 0 or VERSION_PATTERN.fullmatch(value) is None:
        raise LocalAggregateError("urollup version check failed")
    return value


def system_timezone() -> str:
    """Resolve a local IANA zone without printing platform paths."""

    candidates = [os.environ.get("TZ")]
    try:
        target = Path("/etc/localtime").resolve().as_posix()
        candidates.append(target.rsplit("/zoneinfo/", 1)[1] if "/zoneinfo/" in target else None)
    except OSError:
        pass
    for candidate in candidates:
        if not candidate:
            continue
        try:
            ZoneInfo(candidate)
        except ZoneInfoNotFoundError:
            continue
        return candidate
    raise LocalAggregateError("pass --timezone with an IANA zone for the local aggregate")


def arguments(argv: list[str]) -> argparse.Namespace:
    """Parse the deliberately narrow maintainer-only interface."""

    parser = argparse.ArgumentParser()
    parser.add_argument("--urollup", type=Path, required=True)
    parser.add_argument("--timezone")
    parser.add_argument("--output", type=Path)
    parser.add_argument("--consent-local-logs", action="store_true", required=True)
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    """Run the aggregate after explicit consent and write only the safe record."""

    options = arguments(sys.argv[1:] if argv is None else argv)
    timezone = options.timezone or system_timezone()
    try:
        zone = ZoneInfo(timezone)
    except ZoneInfoNotFoundError as error:
        raise LocalAggregateError("the requested timezone is not an IANA zone") from error
    cutoff = datetime.now(zone).date().isoformat()
    binary = options.urollup.resolve()
    if not binary.is_file():
        raise LocalAggregateError("urollup binary is missing")
    run_urollup: Runner = lambda args: execute_json(binary, args)
    common = [
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
    daily = run_urollup(["daily", *common])
    report = run_urollup(["report", *common])
    sessions = run_urollup(["sessions", *common])
    safe = build_aggregate(
        daily=daily,
        report=report,
        sessions=sessions,
        version=version(binary),
        timezone=timezone,
        cutoff=cutoff,
        platform_name=f"{platform.system().lower()}-{platform.machine().lower()}",
        session_summary=stable_session_summary(
            sessions, cutoff=cutoff, timezone=timezone, run_urollup=run_urollup
        ),
    )
    rendered = json.dumps(safe, indent=2, sort_keys=True) + "\n"
    if options.output is None:
        sys.stdout.write(rendered)
    else:
        options.output.parent.mkdir(parents=True, exist_ok=True)
        options.output.write_text(rendered, encoding="utf-8")
        print("local aggregate written")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except LocalAggregateError as error:
        print(f"local aggregate: {error}", file=sys.stderr)
        raise SystemExit(1) from error
