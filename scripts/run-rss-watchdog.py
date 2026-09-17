#!/usr/bin/env python3
"""
Run one command under an external resident-memory watchdog.

The watchdog samples the process and its descendants with `ps`. It kills the process
group when their observed aggregate RSS exceeds the configured limit and writes only
privacy-safe execution metrics, never the command or environment, to its JSON report.
"""

from __future__ import annotations

import argparse
import json
import os
import signal
import subprocess
import sys
import tempfile
import time
from pathlib import Path

RSS_LIMIT_EXIT = 97
WATCHDOG_ERROR_EXIT = 98


def parse_args(arguments: list[str]) -> argparse.Namespace:
    """Parse the limit, sampling interval, report path, and child command."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--limit-mib", type=int, required=True)
    parser.add_argument("--interval-ms", type=int, default=50)
    parser.add_argument("--report", type=Path, required=True)
    parser.add_argument("command", nargs=argparse.REMAINDER)
    args = parser.parse_args(arguments)
    if args.limit_mib <= 0:
        parser.error("--limit-mib must be positive")
    if args.interval_ms <= 0:
        parser.error("--interval-ms must be positive")
    if args.command[:1] == ["--"]:
        args.command = args.command[1:]
    if not args.command:
        parser.error("a command is required after --")
    return args


def process_group_rss_kib(group_id: int) -> int:
    """Return aggregate RSS for one supervised process group from a `ps` snapshot."""
    result = subprocess.run(
        ["ps", "-axo", "pgid=,rss="],
        check=True,
        capture_output=True,
        text=True,
    )
    total = 0
    for line in result.stdout.splitlines():
        fields = line.split()
        if len(fields) != 2:
            raise RuntimeError("unexpected output from ps -axo pgid=,rss=")
        process_group, rss_kib = (int(field) for field in fields)
        if process_group == group_id:
            total += rss_kib
    return total


def write_report(path: Path, report: dict[str, int | bool | str]) -> None:
    """Atomically publish the privacy-safe JSON metrics."""
    path.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.NamedTemporaryFile(
        mode="w",
        encoding="utf-8",
        dir=path.parent,
        prefix=f".{path.name}.",
        suffix=".tmp",
        delete=False,
    ) as temporary:
        temporary_path = Path(temporary.name)
        json.dump(report, temporary, sort_keys=True)
        temporary.write("\n")
    try:
        os.replace(temporary_path, path)
    except OSError:
        temporary_path.unlink(missing_ok=True)
        raise


def terminate_process_group(process: subprocess.Popen[bytes]) -> None:
    """Kill the supervised process group without affecting the watchdog."""
    try:
        os.killpg(process.pid, signal.SIGKILL)
    except ProcessLookupError:
        pass
    process.wait()


def shell_exit_code(return_code: int) -> int:
    """Map a signal-terminated child to the conventional shell exit status."""
    return return_code if return_code >= 0 else 128 + abs(return_code)


def supervise(args: argparse.Namespace) -> int:
    """Run the child, enforce its aggregate observed RSS limit, and record metrics."""
    limit_kib = args.limit_mib * 1024
    started = time.monotonic()
    peak_kib = 0
    killed_for_rss = False
    watchdog_error = ""
    leader_return_code: int | None = None
    process = subprocess.Popen(args.command, start_new_session=True)
    try:
        while True:
            if leader_return_code is None:
                leader_return_code = process.poll()
            try:
                current_kib = process_group_rss_kib(process.pid)
            except (OSError, subprocess.SubprocessError, ValueError, RuntimeError) as error:
                watchdog_error = str(error)
                terminate_process_group(process)
                return_code = WATCHDOG_ERROR_EXIT
                break
            peak_kib = max(peak_kib, current_kib)
            if current_kib > limit_kib:
                killed_for_rss = True
                terminate_process_group(process)
                return_code = RSS_LIMIT_EXIT
                break
            if leader_return_code is not None and current_kib == 0:
                return_code = shell_exit_code(leader_return_code)
                break
            time.sleep(args.interval_ms / 1000)
    except KeyboardInterrupt:
        terminate_process_group(process)
        return_code = 130

    elapsed_ms = round((time.monotonic() - started) * 1000)
    report: dict[str, int | bool | str] = {
        "schema_version": 1,
        "exit_code": return_code,
        "killed_for_rss": killed_for_rss,
        "limit_rss_kib": limit_kib,
        "peak_rss_kib": peak_kib,
        "elapsed_ms": elapsed_ms,
        "sample_interval_ms": args.interval_ms,
    }
    if watchdog_error:
        report["watchdog_error"] = watchdog_error
    try:
        write_report(args.report, report)
    except OSError as error:
        detail = error.strerror or type(error).__name__
        print(f"rss-watchdog: cannot publish metrics: {detail}", file=sys.stderr)
        return WATCHDOG_ERROR_EXIT
    print(
        "rss-watchdog: "
        f"exit={return_code} peak_kib={peak_kib} limit_kib={limit_kib} "
        f"elapsed_ms={elapsed_ms} killed={str(killed_for_rss).lower()}",
        file=sys.stderr,
    )
    if watchdog_error:
        print(f"rss-watchdog: sampling failed: {watchdog_error}", file=sys.stderr)
    return return_code


def main() -> int:
    """Run the command-line watchdog."""
    if os.name != "posix":
        print("error: the RSS watchdog currently supports macOS and Linux", file=sys.stderr)
        return WATCHDOG_ERROR_EXIT
    return supervise(parse_args(sys.argv[1:]))


if __name__ == "__main__":
    raise SystemExit(main())
