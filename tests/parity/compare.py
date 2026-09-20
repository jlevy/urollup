#!/usr/bin/env python3
"""Compare urollup token rows with pinned ccusage over the frozen fixtures."""

from __future__ import annotations

import argparse
import fnmatch
import json
import os
import platform
import shutil
import stat
import subprocess
import sys
import tempfile
import tomllib
from dataclasses import asdict, dataclass, replace
from datetime import date, datetime, timedelta
from pathlib import Path
from typing import Any
from zoneinfo import ZoneInfo

CCUSAGE_VERSION = "20.0.20"
METRICS = ("uncached_input", "cache_read", "cache_write", "output", "total")
CODEX_METRICS = (*METRICS, "reasoning")
LEDGER_CAUSES = {"ccusage-bug", "dedupe", "semantics", "pricing", "unsupported"}
MATCH_FIELDS = ("case", "report", "variant", "timezone", "key", "metric")
RULE_FIELDS = MATCH_FIELDS[1:]


class ParityError(RuntimeError):
    """The parity harness could not produce an accepted comparison."""


@dataclass(frozen=True)
class Difference:
    """One unequal metric or one-sided row."""

    case: str
    report: str
    variant: str
    timezone: str
    key: str
    metric: str
    urollup: int
    ccusage: int
    delta: int
    detail: str | None = None
    explained_by: str | None = None


@dataclass(frozen=True)
class RunResult:
    """A subprocess result with decoded standard streams."""

    returncode: int
    stdout: str
    stderr: str


def load_toml(path: Path) -> dict[str, Any]:
    """Read one TOML document."""

    with path.open("rb") as stream:
        return tomllib.load(stream)


def native_ccusage_binary(package_root: Path) -> Path:
    """Return the current platform's executable from ccusage's optional package."""

    system = platform.system().lower()
    machine = platform.machine().lower()
    systems = {"darwin": "darwin", "linux": "linux", "windows": "win32"}
    machines = {
        "aarch64": "arm64",
        "amd64": "x64",
        "arm64": "arm64",
        "x86_64": "x64",
    }
    if system not in systems or machine not in machines:
        raise ParityError(f"ccusage 20.0.20 has no reviewed binary for {system}-{machine}")
    target = f"{systems[system]}-{machines[machine]}"
    suffix = ".exe" if system == "windows" else ""
    return (
        package_root
        / "node_modules"
        / "@ccusage"
        / f"ccusage-{target}"
        / "bin"
        / f"ccusage{suffix}"
    )


def prepare_ccusage_binary(binary: Path) -> None:
    """Validate the installed native package and make its scriptless install executable."""

    if not binary.is_file():
        raise ParityError(f"pinned ccusage native binary is missing: {binary}")
    if os.name != "nt":
        binary.chmod(binary.stat().st_mode | stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH)
    result = run([str(binary), "--version"], cwd=binary.parent, env=isolated_base_env(binary.parent))
    expected = f"ccusage {CCUSAGE_VERSION}"
    if result.returncode != 0 or result.stdout.strip() != expected:
        raise ParityError(
            f"expected {expected!r} from the native binary, got "
            f"exit {result.returncode}: {result.stdout.strip()!r} {result.stderr.strip()!r}"
        )


def isolated_base_env(temp_root: Path) -> dict[str, str]:
    """Build the small environment shared by isolated tool runs."""

    allowed = ("COMSPEC", "PATH", "PATHEXT", "SYSTEMROOT", "TEMP", "TMP", "TMPDIR", "WINDIR")
    environment = {name: os.environ[name] for name in allowed if name in os.environ}
    home = temp_root / "home"
    xdg = temp_root / "xdg"
    home.mkdir(parents=True, exist_ok=True)
    xdg.mkdir(parents=True, exist_ok=True)
    environment.update(
        {
            "HOME": str(home),
            "XDG_CONFIG_HOME": str(xdg),
            "NO_COLOR": "1",
            "LOG_LEVEL": "0",
        }
    )
    return environment


def run(command: list[str], *, cwd: Path, env: dict[str, str]) -> RunResult:
    """Run one tool without a shell."""

    completed = subprocess.run(  # noqa: S603 - every executable is resolved by this harness
        command,
        cwd=cwd,
        env=env,
        check=False,
        capture_output=True,
        text=True,
        encoding="utf-8",
    )
    return RunResult(completed.returncode, completed.stdout, completed.stderr)


def metrics_from_ccusage(row: dict[str, Any], *, reasoning: bool) -> dict[str, int]:
    """Normalize one ccusage row to urollup's disjoint token categories."""

    output = {
        "uncached_input": integer(row, "inputTokens"),
        "cache_read": integer(row, "cacheReadTokens"),
        "cache_write": integer(row, "cacheCreationTokens"),
        "output": integer(row, "outputTokens"),
        "total": integer(row, "totalTokens"),
    }
    if reasoning:
        output["reasoning"] = integer(row, "reasoningOutputTokens")
    return output


def metrics_from_urollup(row: dict[str, Any], *, reasoning: bool) -> dict[str, int]:
    """Normalize one urollup row, treating an absent known category as zero."""

    tokens = row.get("tokens")
    if not isinstance(tokens, dict):
        raise ParityError("urollup row has no tokens object")
    output = {metric: integer(tokens, metric) for metric in METRICS}
    if reasoning:
        output["reasoning"] = integer(tokens, "reasoning")
    return output


def integer(record: dict[str, Any], key: str) -> int:
    """Read a non-negative integral JSON field, with missing or null meaning zero."""

    value = record.get(key, 0)
    if value is None:
        return 0
    if isinstance(value, bool) or not isinstance(value, int) or value < 0:
        raise ParityError(f"{key} must be a non-negative integer, got {value!r}")
    return value


def parse_json(result: RunResult, tool: str) -> dict[str, Any]:
    """Decode a successful tool result."""

    if result.returncode != 0:
        raise ParityError(f"{tool} exited {result.returncode}: {result.stderr.strip()}")
    try:
        value = json.loads(result.stdout)
    except json.JSONDecodeError as error:
        raise ParityError(f"{tool} did not write JSON: {error}") from error
    if not isinstance(value, dict):
        raise ParityError(f"{tool} JSON root must be an object")
    return value


def parse_ccusage_rows(
    payload: dict[str, Any],
    *,
    view: str,
    reasoning: bool,
    expected_threads: list[str],
) -> dict[str, dict[str, int]]:
    """Read daily or session rows from ccusage JSON."""

    field = "daily" if view == "daily" else "sessions"
    rows = payload.get(field)
    if not isinstance(rows, list):
        raise ParityError(f"ccusage JSON has no {field} array")
    output: dict[str, dict[str, int]] = {}
    for row in rows:
        if not isinstance(row, dict):
            raise ParityError(f"ccusage {field} member is not an object")
        if view == "daily":
            key = row.get("date", row.get("period"))
        else:
            key = normalize_session_key(row.get("sessionId"), expected_threads)
        if not isinstance(key, str) or not key:
            raise ParityError(f"ccusage {field} row has no stable key")
        metrics = metrics_from_ccusage(row, reasoning=reasoning)
        if key in output:
            for metric, value in metrics.items():
                output[key][metric] += value
        else:
            output[key] = metrics
    return output


def normalize_session_key(value: Any, expected_threads: list[str]) -> str | None:
    """Map ccusage's path-like Codex session key back to the fixture's native thread ID."""

    if not isinstance(value, str) or not value:
        return None
    exact = [thread for thread in expected_threads if thread == value]
    if exact:
        return exact[0]
    matches = [
        thread
        for thread in expected_threads
        if thread in value or ("/" in thread and thread.rsplit("/", 1)[-1] == value)
    ]
    if len(matches) > 1:
        raise ParityError(f"ccusage session key {value!r} contains several fixture thread IDs")
    return matches[0] if matches else value


def parse_urollup_daily(payload: dict[str, Any], *, reasoning: bool) -> dict[str, dict[str, int]]:
    """Read daily rows from urollup JSON."""

    rows = payload.get("rows")
    if not isinstance(rows, list):
        raise ParityError("urollup JSON has no rows array")
    output: dict[str, dict[str, int]] = {}
    for row in rows:
        if not isinstance(row, dict) or not isinstance(row.get("date"), str):
            raise ParityError("urollup daily row has no date")
        output[row["date"]] = metrics_from_urollup(row, reasoning=reasoning)
    return output


def parse_urollup_session(payload: dict[str, Any], *, reasoning: bool) -> dict[str, int]:
    """Sum the selected session document, normally one row."""

    rows = payload.get("rows")
    if not isinstance(rows, list):
        raise ParityError("urollup JSON has no rows array")
    total = {metric: 0 for metric in (CODEX_METRICS if reasoning else METRICS)}
    for row in rows:
        if not isinstance(row, dict):
            raise ParityError("urollup session row is not an object")
        for metric, value in metrics_from_urollup(row, reasoning=reasoning).items():
            total[metric] += value
    return total


def difference_rows(
    *,
    case: str,
    report: str,
    variant: str,
    timezone: str,
    urollup: dict[str, dict[str, int]],
    ccusage: dict[str, dict[str, int]],
) -> list[Difference]:
    """Compare keyed rows, making row presence itself an explicit difference."""

    differences: list[Difference] = []
    for key in sorted(set(urollup) | set(ccusage)):
        left = urollup.get(key)
        right = ccusage.get(key)
        if left is None or right is None:
            left_present = int(left is not None)
            right_present = int(right is not None)
            differences.append(
                Difference(
                    case,
                    report,
                    variant,
                    timezone,
                    key,
                    "row",
                    left_present,
                    right_present,
                    left_present - right_present,
                )
            )
            continue
        for metric in sorted(set(left) | set(right)):
            left_value = left.get(metric, 0)
            right_value = right.get(metric, 0)
            if left_value != right_value:
                differences.append(
                    Difference(
                        case,
                        report,
                        variant,
                        timezone,
                        key,
                        metric,
                        left_value,
                        right_value,
                        left_value - right_value,
                    )
                )
    return differences


def execution_difference(
    *,
    case: str,
    report: str,
    variant: str,
    timezone: str,
    result: RunResult,
    temp_root: Path,
) -> Difference:
    """Represent a ccusage failure without leaking its temporary path."""

    detail = result.stderr.strip().replace(str(temp_root), "<tmp>")
    return Difference(
        case,
        report,
        variant,
        timezone,
        "*",
        "execution",
        0,
        result.returncode or 1,
        -(result.returncode or 1),
        detail=detail,
    )


def filter_daily_since(rows: dict[str, dict[str, int]], since: date) -> dict[str, dict[str, int]]:
    """Apply ccusage's inclusive calendar start to urollup daily rows."""

    return {key: value for key, value in rows.items() if date.fromisoformat(key) >= since}


def filter_sessions_since(
    rows: dict[str, dict[str, int]],
    last_activity: dict[str, date | None],
    since: date,
) -> dict[str, dict[str, int]]:
    """Mirror ccusage's last-activity session filter from frozen fixture metadata."""

    return {
        key: value
        for key, value in rows.items()
        if last_activity.get(key) is None or last_activity[key] >= since
    }


def inclusive_until_to_exclusive(value: str) -> date:
    """Map ccusage's inclusive YYYYMMDD end to a half-open calendar bound."""

    return datetime.strptime(value, "%Y%m%d").date() + timedelta(days=1)


def expected_metadata(expected_path: Path, timezone: str) -> tuple[list[str], dict[str, date | None], date]:
    """Read native threads, their last owned activity, and the case's last local day."""

    payload = json.loads(expected_path.read_text(encoding="utf-8"))
    threads = [thread["id"] for thread in payload["threads"]]
    last: dict[str, datetime] = {}
    zone = ZoneInfo(timezone)
    all_times: list[datetime] = []
    for request in payload["requests"]:
        timestamp = request.get("timestamp")
        owner = request.get("owner_thread")
        if not isinstance(timestamp, str):
            continue
        moment = datetime.fromisoformat(timestamp.replace("Z", "+00:00")).astimezone(zone)
        all_times.append(moment)
        if isinstance(owner, str) and (owner not in last or moment > last[owner]):
            last[owner] = moment
    if not all_times:
        raise ParityError(f"fixture has no dated requests: {expected_path.parent.name}")
    return threads, {thread: last.get(thread).date() if thread in last else None for thread in threads}, max(all_times).date()


def urollup_command(
    binary: Path,
    view: str,
    sources: list[Path],
    timezone: str,
    session: str | None = None,
) -> list[str]:
    """Build one explicit-source urollup command."""

    command = [str(binary), view]
    for source in sources:
        command.extend(("--source", str(source)))
    command.extend(("--no-default-sources", "--scope", "self", "--format", "json", "--timezone", timezone))
    command.extend(("--session", session) if session is not None else ("--all",))
    return command


def ccusage_command(
    binary: Path,
    agent: str | None,
    view: str,
    timezone: str,
    since: date | None,
) -> list[str]:
    """Build one direct native ccusage command."""

    command = [str(binary)]
    if agent is not None:
        command.append(agent)
    command.extend((view, "--offline", "--json", "--timezone", timezone, "--no-color"))
    if since is not None:
        command.extend(("--since", since.strftime("%Y%m%d")))
    return command


def compare_ccusage(
    *,
    binary: Path,
    agent: str | None,
    view: str,
    timezone: str,
    since: date | None,
    expected_threads: list[str],
    urollup_rows: dict[str, dict[str, int]],
    case_id: str,
    report: str,
    cwd: Path,
    env: dict[str, str],
    temp_root: Path,
) -> list[Difference]:
    """Run ccusage and compare it with already-normalized urollup rows."""

    variant = "since" if since is not None else "full"
    result = run(ccusage_command(binary, agent, view, timezone, since), cwd=cwd, env=env)
    if result.returncode != 0:
        return [
            execution_difference(
                case=case_id,
                report=report,
                variant=variant,
                timezone=timezone,
                result=result,
                temp_root=temp_root,
            )
        ]
    payload = parse_json(result, "ccusage")
    rows = parse_ccusage_rows(
        payload,
        view=view,
        reasoning=agent == "codex",
        expected_threads=expected_threads,
    )
    return difference_rows(
        case=case_id,
        report=report,
        variant=variant,
        timezone=timezone,
        urollup=urollup_rows,
        ccusage=rows,
    )


def compare_case(
    *,
    case_path: Path,
    dialect: str,
    agent: str,
    timezones: list[str],
    urollup: Path,
    ccusage: Path,
) -> tuple[dict[str, Any], list[Difference]]:
    """Run every milestone 0.1 comparison for one fixture case."""

    case_id = f"{dialect}/{case_path.name}"
    differences: list[Difference] = []
    comparisons = 0
    with tempfile.TemporaryDirectory(prefix="urollup-parity-") as temporary:
        temp_root = Path(temporary)
        claude_root = temp_root / "claude"
        codex_root = temp_root / "codex"
        (claude_root / "projects").mkdir(parents=True)
        (codex_root / "sessions").mkdir(parents=True)
        fixture_root = claude_root if agent == "claude" else codex_root
        shutil.copytree(case_path, fixture_root, dirs_exist_ok=True, symlinks=True)
        work = temp_root / "work"
        work.mkdir()
        environment = isolated_base_env(temp_root)
        environment.update(
            {"CLAUDE_CONFIG_DIR": str(claude_root), "CODEX_HOME": str(codex_root)}
        )

        for timezone in timezones:
            expected_threads, last_activity, since = expected_metadata(
                case_path / "expected.json", timezone
            )
            reasoning = agent == "codex"
            source = fixture_root

            daily_result = run(
                urollup_command(urollup, "daily", [source], timezone), cwd=work, env=environment
            )
            daily = parse_urollup_daily(
                parse_json(daily_result, "urollup daily"), reasoning=reasoning
            )
            for selected_since, rows in (
                (None, daily),
                (since, filter_daily_since(daily, since)),
            ):
                differences.extend(
                    compare_ccusage(
                        binary=ccusage,
                        agent=agent,
                        view="daily",
                        timezone=timezone,
                        since=selected_since,
                        expected_threads=expected_threads,
                        urollup_rows=rows,
                        case_id=case_id,
                        report=f"{agent}-daily",
                        cwd=work,
                        env=environment,
                        temp_root=temp_root,
                    )
                )
                comparisons += 1

            sessions: dict[str, dict[str, int]] = {}
            for thread in expected_threads:
                selector = thread.rsplit("/", 1)[-1]
                result = run(
                    urollup_command(urollup, "sessions", [source], timezone, session=selector),
                    cwd=work,
                    env=environment,
                )
                sessions[thread] = parse_urollup_session(
                    parse_json(result, f"urollup sessions --session {selector}"),
                    reasoning=reasoning,
                )
            for selected_since, rows in (
                (None, sessions),
                (since, filter_sessions_since(sessions, last_activity, since)),
            ):
                differences.extend(
                    compare_ccusage(
                        binary=ccusage,
                        agent=agent,
                        view="session",
                        timezone=timezone,
                        since=selected_since,
                        expected_threads=expected_threads,
                        urollup_rows=rows,
                        case_id=case_id,
                        report=f"{agent}-session",
                        cwd=work,
                        env=environment,
                        temp_root=temp_root,
                    )
                )
                comparisons += 1

            unified_result = run(
                urollup_command(urollup, "daily", [claude_root, codex_root], timezone),
                cwd=work,
                env=environment,
            )
            unified = parse_urollup_daily(
                parse_json(unified_result, "urollup unified daily"), reasoning=False
            )
            differences.extend(
                compare_ccusage(
                    binary=ccusage,
                    agent=None,
                    view="daily",
                    timezone=timezone,
                    since=None,
                    expected_threads=expected_threads,
                    urollup_rows=unified,
                    case_id=case_id,
                    report="unified-daily",
                    cwd=work,
                    env=environment,
                    temp_root=temp_root,
                )
            )
            comparisons += 1

    return {"case": case_id, "comparisons": comparisons}, differences


def validate_ledger(
    differences: list[Difference], ledger: dict[str, Any]
) -> tuple[list[Difference], list[str]]:
    """Require each difference and cited ledger rule to match exactly as declared."""

    if ledger.get("schema") != 1 or not isinstance(ledger.get("entry", []), list):
        raise ParityError("ledger.toml must use schema 1 and [[entry]] records")
    entries = ledger.get("entry", [])
    ids: set[str] = set()
    errors: list[str] = []
    rules: list[tuple[str, str, dict[str, str], int]] = []
    for entry in entries:
        if not isinstance(entry, dict):
            raise ParityError("ledger entries must be tables")
        identifier = entry.get("id")
        if not isinstance(identifier, str) or not identifier or identifier in ids:
            raise ParityError(f"ledger entry has a missing or duplicate ID: {identifier!r}")
        ids.add(identifier)
        if entry.get("cause") not in LEDGER_CAUSES:
            raise ParityError(f"ledger entry {identifier} has an invalid cause")
        for field in ("case", "citation", "design", "retirement"):
            if not isinstance(entry.get(field), str) or not entry[field].strip():
                raise ParityError(f"ledger entry {identifier} needs {field}")
        expected = entry.get("difference")
        if not isinstance(expected, list) or not expected:
            raise ParityError(f"ledger entry {identifier} needs difference rules")
        for index, rule in enumerate(expected, start=1):
            rule_id = f"{identifier}#{index}"
            if not isinstance(rule, dict):
                raise ParityError(f"ledger rule {rule_id} must be a table")
            patterns = {"case": entry["case"]}
            for field in RULE_FIELDS:
                pattern = rule.get(field, "*")
                if not isinstance(pattern, str) or not pattern.strip():
                    raise ParityError(f"ledger rule {rule_id} has an invalid {field}")
                patterns[field] = pattern
            delta = rule.get("expected_delta")
            if isinstance(delta, bool) or not isinstance(delta, int):
                raise ParityError(f"ledger rule {rule_id} needs an integral expected_delta")
            expected_matches = rule.get("expected_matches")
            if (
                isinstance(expected_matches, bool)
                or not isinstance(expected_matches, int)
                or expected_matches < 1
            ):
                raise ParityError(f"ledger rule {rule_id} needs positive expected_matches")
            rules.append((identifier, rule_id, patterns, delta))

    matches = {rule_id: 0 for _, rule_id, _, _ in rules}
    expected_counts = {
        f"{entry['id']}#{index}": rule["expected_matches"]
        for entry in entries
        for index, rule in enumerate(entry["difference"], start=1)
    }

    explained: list[Difference] = []
    for difference in differences:
        candidates = []
        for identifier, rule_id, patterns, delta in rules:
            fields_match = all(
                fnmatch.fnmatchcase(str(getattr(difference, field)), patterns[field])
                for field in MATCH_FIELDS
            )
            if fields_match and difference.delta == delta:
                candidates.append((identifier, rule_id))
        if len(candidates) != 1:
            errors.append(
                f"difference {difference.case} {difference.report}/{difference.variant} "
                f"{difference.timezone} {difference.key} {difference.metric} delta "
                f"{difference.delta:+d} matched {len(candidates)} ledger entries"
            )
            explained.append(difference)
            continue
        identifier, rule_id = candidates[0]
        matches[rule_id] += 1
        explained.append(replace(difference, explained_by=identifier))
    for rule_id, count in matches.items():
        expected_count = expected_counts[rule_id]
        if count != expected_count:
            errors.append(
                f"ledger rule {rule_id} matched {count} differences; expected {expected_count}"
            )
    return explained, errors


def discover_cases(repository: Path, config: dict[str, Any]) -> list[tuple[Path, str, str]]:
    """Discover every expected fixture case named by cases.toml."""

    if config.get("schema") != 1 or not isinstance(config.get("suite"), list):
        raise ParityError("cases.toml must use schema 1 and [[suite]] records")
    cases = []
    for suite in config["suite"]:
        dialect = suite.get("dialect")
        agent = suite.get("agent")
        root_value = suite.get("fixtures")
        if agent not in {"claude", "codex"} or not all(
            isinstance(value, str) and value for value in (dialect, root_value)
        ):
            raise ParityError("each parity suite needs a dialect, agent, and fixture root")
        root = repository / root_value
        discovered = sorted(path for path in root.iterdir() if (path / "expected.json").is_file())
        if not discovered:
            raise ParityError(f"parity suite has no fixture cases: {root_value}")
        cases.extend((path, dialect, agent) for path in discovered)
    return cases


def arguments(argv: list[str]) -> argparse.Namespace:
    """Parse the narrow harness command line."""

    parser = argparse.ArgumentParser()
    parser.add_argument("--urollup", type=Path, required=True)
    parser.add_argument("--ccusage-package", type=Path, required=True)
    parser.add_argument("--cases", type=Path, required=True)
    parser.add_argument("--ledger", type=Path, required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    """Run the full fixture matrix and write deterministic per-case reports."""

    options = arguments(sys.argv[1:] if argv is None else argv)
    repository = Path(__file__).resolve().parents[2]
    urollup = options.urollup.resolve()
    if not urollup.is_file():
        raise ParityError(f"urollup binary is missing: {urollup}")
    ccusage = native_ccusage_binary(options.ccusage_package.resolve())
    prepare_ccusage_binary(ccusage)
    config = load_toml(options.cases)
    timezones = config.get("timezones")
    if not isinstance(timezones, list) or not timezones or not all(
        isinstance(timezone, str) for timezone in timezones
    ):
        raise ParityError("cases.toml needs a non-empty timezones array")

    reports = []
    differences = []
    for case_path, dialect, agent in discover_cases(repository, config):
        report, case_differences = compare_case(
            case_path=case_path,
            dialect=dialect,
            agent=agent,
            timezones=timezones,
            urollup=urollup,
            ccusage=ccusage,
        )
        reports.append(report)
        differences.extend(case_differences)

    explained, errors = validate_ledger(differences, load_toml(options.ledger))
    by_case: dict[str, list[Difference]] = {}
    for difference in explained:
        by_case.setdefault(difference.case, []).append(difference)
    options.output_dir.mkdir(parents=True, exist_ok=True)
    for report in reports:
        case_id = report["case"]
        destination = options.output_dir / f"{case_id.replace('/', '__')}.json"
        destination.write_text(
            json.dumps(
                {
                    **report,
                    "ccusage_version": CCUSAGE_VERSION,
                    "differences": [asdict(item) for item in by_case.get(case_id, [])],
                },
                indent=2,
                sort_keys=True,
            )
            + "\n",
            encoding="utf-8",
        )
    summary = {
        "ccusage_version": CCUSAGE_VERSION,
        "comparisons": sum(report["comparisons"] for report in reports),
        "differences": len(differences),
        "errors": errors,
        "fixture_cases": len(reports),
        "urollup_version": parse_version(urollup),
    }
    (options.output_dir / "summary.json").write_text(
        json.dumps(summary, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    if errors:
        preview = "\n".join(f"  - {error}" for error in errors[:20])
        suffix = "\n  - ..." if len(errors) > 20 else ""
        raise ParityError(f"parity ledger rejected {len(errors)} item(s):\n{preview}{suffix}")
    print(
        f"parity: {len(reports)} fixture cases, {summary['comparisons']} comparisons, "
        f"{len(differences)} explained differences"
    )
    return 0


def parse_version(binary: Path) -> str:
    """Read urollup's exact version outside a fixture environment."""

    result = run([str(binary), "--version"], cwd=binary.parent, env=isolated_base_env(binary.parent))
    if result.returncode != 0 or not result.stdout.strip():
        raise ParityError("urollup --version failed")
    return result.stdout.strip()


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except ParityError as error:
        print(f"parity: {error}", file=sys.stderr)
        raise SystemExit(1) from error
