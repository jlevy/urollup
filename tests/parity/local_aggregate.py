#!/usr/bin/env python3
"""Produce a privacy-safe aggregate report over consented local agent logs.

The report comes from `--version` and three whole-history urollup runs (`sessions`,
`daily` and `report`), however many sessions the history holds. Every count it carries
is urollup's own. This script filters complete days, sums day rows, labels how many
summed days reported each token field, classifies sessions as stable from urollup's
per-session dates, and copies a fixed allowlist of fields.
"""

from __future__ import annotations

import argparse
import json
import os
import platform
import re
import subprocess
import sys
from datetime import datetime
from pathlib import Path
from typing import Any
from zoneinfo import ZoneInfo, ZoneInfoNotFoundError

FORMAT = "urollup.local-aggregate/v2"
# urollup's JSON token fields. `cache_write` sums the three lifetime buckets and
# `reasoning` is a subset of `output`; both are copied as urollup reports them.
TOKEN_METRICS = (
    "uncached_input",
    "cache_read",
    "cache_write",
    "cache_write_5m",
    "cache_write_1h",
    "cache_write_unspecified",
    "output",
    "reasoning",
    "provider_only",
    "total",
)
REQUEST_CLASSES = ("owned", "ambiguous", "unknown")
AGENTS = {"claude", "codex", "unknown"}
VERSION_PATTERN = re.compile(r"^urollup [0-9]+\.[0-9]+\.[0-9]+(?:[-+][0-9A-Za-z.-]+)?$")
DATE_PATTERN = re.compile(r"^[0-9]{4}-[0-9]{2}-[0-9]{2}$")


class LocalAggregateError(RuntimeError):
    """A local aggregate could not be produced without risking private output."""


def required_count(record: dict[str, Any], key: str) -> int:
    """Read a counter urollup always emits; a missing or null one means a changed contract."""

    value = record.get(key)
    if isinstance(value, bool) or not isinstance(value, int) or value < 0:
        raise LocalAggregateError(f"invalid or missing aggregate field {key}")
    return value


def token_count(tokens: dict[str, Any], key: str) -> int | None:
    """Read one token counter, keeping an absent or null counter unknown, not zero.

    urollup omits a token field that no counted request in the row reported, so absence
    is the only signal that the value was never observed.
    """

    value = tokens.get(key)
    if value is None:
        return None
    if isinstance(value, bool) or not isinstance(value, int) or value < 0:
        raise LocalAggregateError(f"invalid aggregate field {key}")
    return value


def token_counts(row: dict[str, Any]) -> dict[str, int | None]:
    """Copy only the fixed token metric allowlist from one tool row."""

    tokens = row.get("tokens")
    if not isinstance(tokens, dict):
        raise LocalAggregateError("tool output lacks token aggregates")
    return {metric: token_count(tokens, metric) for metric in TOKEN_METRICS}


def token_totals(
    days: list[dict[str, int | None]],
) -> tuple[dict[str, int | None], dict[str, str]]:
    """Sum day rows per metric and label how many of them carried it.

    The label is `all_days` when every summed day row carried the metric, even as zero;
    `some_days` when only some did, so the sum covers those days; and `no_days`, with a
    null sum, when none did. A urollup day row carries a field when any one of its
    counted requests reported it, so a day mixing Claude and Codex requests counts as
    carrying an agent-specific field such as `reasoning` although some requests lacked
    it. Request-level availability needs per-metric request counts that urollup
    does not report yet.
    """

    totals: dict[str, int | None] = {}
    coverage: dict[str, str] = {}
    for metric in TOKEN_METRICS:
        reported = [day[metric] for day in days if day[metric] is not None]
        totals[metric] = sum(reported) if reported else None
        if not reported:
            coverage[metric] = "no_days"
        elif len(reported) == len(days):
            coverage[metric] = "all_days"
        else:
            coverage[metric] = "some_days"
    return totals, coverage


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
        if day is None:
            continue  # Undated requests cannot be placed before the cutoff.
        if not isinstance(day, str) or DATE_PATTERN.fullmatch(day) is None:
            raise LocalAggregateError("daily output contains an invalid date")
        if day < cutoff:
            stable.append(row)
    return stable


def request_counts(row: dict[str, Any]) -> dict[str, int]:
    """Copy only the fixed request ownership counters."""

    requests = row.get("requests")
    if not isinstance(requests, dict):
        raise LocalAggregateError("tool output lacks request aggregates")
    return {name: required_count(requests, name) for name in REQUEST_CLASSES}


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
        by_code[code] = by_code.get(code, 0) + required_count(diagnostic, "count")
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
    complete = coverage.get("complete")
    if not isinstance(complete, bool):
        raise LocalAggregateError("invalid or missing aggregate field complete")
    return {
        "complete": complete,
        "copies_excluded": required_count(coverage, "copies_excluded"),
        "limit_observations": required_count(coverage, "limit_observations"),
        "requests_without_usage": required_count(coverage, "requests_without_usage"),
        "unresolved_requests": required_count(unresolved, "requests"),
        "possible_requests": required_count(possible, "requests"),
    }


def stable_session_summary(sessions: dict[str, Any], *, cutoff: str) -> dict[str, Any]:
    """Count sessions whose owned requests all fall on complete days before `cutoff`.

    urollup dates each whole-history `sessions` row by the rule `daily` uses:
    `last_date` is the latest dated counted request the session owns and
    `undated_requests` counts those without a timestamp. A session is stable when it
    owns a counted request, none undated, and its last date is before the cutoff.

    Classification uses the whole-history ledger, as the record's totals do. A request
    that two sessions both prove they own is ambiguous there and belongs to neither, so
    a finished session whose requests are all ambiguous counts as
    `without_owned_requests`, not stable. A per-session `daily --session` run narrows
    discovery and may own such requests. Identifiers stay in memory; only counts return.
    """

    rows = sessions.get("rows")
    if not isinstance(rows, list):
        raise LocalAggregateError("sessions output has no rows")
    stable = 0
    excluded = {
        "active_or_undated": 0,
        "without_owned_requests": 0,
        "unowned_or_unrecognized_agent": 0,
    }
    by_agent = {agent: 0 for agent in sorted(AGENTS)}
    for row in rows:
        if not isinstance(row, dict):
            raise LocalAggregateError("sessions output contains an invalid row")
        if "undated_requests" not in row:
            raise LocalAggregateError(
                "sessions output lacks undated_requests; rebuild urollup from this revision"
            )
        undated = required_count(row, "undated_requests")
        requests = sum(request_counts(row).values())
        last_date = row.get("last_date")
        if last_date is not None and (
            not isinstance(last_date, str) or DATE_PATTERN.fullmatch(last_date) is None
        ):
            raise LocalAggregateError("sessions output contains an invalid last date")
        thread = row.get("thread")
        agent = row.get("agent")
        if not isinstance(thread, str) or not isinstance(agent, str) or agent not in AGENTS:
            excluded["unowned_or_unrecognized_agent"] += 1
        elif requests == 0:
            excluded["without_owned_requests"] += 1
        elif undated > 0 or last_date is None or last_date >= cutoff:
            excluded["active_or_undated"] += 1
        else:
            stable += 1
            by_agent[agent] += 1
    return {"stable": stable, "stable_by_agent": by_agent, "excluded": excluded}


def build_aggregate(
    *,
    daily: dict[str, Any],
    report: dict[str, Any],
    session_summary: dict[str, Any],
    version: str,
    timezone: str,
    cutoff: str,
    platform_name: str,
) -> dict[str, Any]:
    """Build the complete allowlisted output record around a `stable_session_summary`."""

    if VERSION_PATTERN.fullmatch(version) is None:
        raise LocalAggregateError("urollup returned an invalid version")
    requests = {name: 0 for name in REQUEST_CLASSES}
    per_day = []
    for row in stable_daily_rows(daily, cutoff):
        owned = request_counts(row)
        requests = add_requests(requests, owned)
        per_day.append({"date": row["date"], "requests": owned, "tokens": token_counts(row)})
    tokens, day_coverage = token_totals([day["tokens"] for day in per_day])
    return {
        "format": FORMAT,
        "tool": version,
        "platform": platform_name,
        "timezone": timezone,
        "interval": {"start": None, "until_exclusive": cutoff},
        "totals": {"requests": requests, "tokens": tokens, "token_day_coverage": day_coverage},
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
    # A binary that cannot report its version fails before any ingest, and one whose
    # `sessions` rows lack the calendar fields fails before the other two ingests.
    tool = version(binary)
    session_summary = stable_session_summary(
        execute_json(binary, ["sessions", *common]), cutoff=cutoff
    )
    safe = build_aggregate(
        daily=execute_json(binary, ["daily", *common]),
        report=execute_json(binary, ["report", *common]),
        session_summary=session_summary,
        version=tool,
        timezone=timezone,
        cutoff=cutoff,
        platform_name=f"{platform.system().lower()}-{platform.machine().lower()}",
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
    except OSError:
        # OS exceptions carry private paths; keep launch and publication failures safe.
        print("local aggregate: operating-system operation failed", file=sys.stderr)
        raise SystemExit(1) from None
