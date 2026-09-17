"""Privacy and aggregation tests for maintainer-only local acceptance tools."""

from __future__ import annotations

import json
import os
import subprocess
import sys
import tempfile
import unittest
from contextlib import redirect_stderr
from io import StringIO
from pathlib import Path
from unittest.mock import patch
from zoneinfo import ZoneInfo

import compare
import local_aggregate
import local_diff

PRIVATE_MARKERS = (
    "PRIVATE_PATH_MARKER",
    "PRIVATE_PROJECT_MARKER",
    "PRIVATE_SESSION_MARKER",
    "PRIVATE_REQUEST_MARKER",
    "PRIVATE_PROMPT_MARKER",
    "PRIVATE_MODEL_MARKER",
)


def tokens(total: int) -> dict[str, int]:
    """Build one complete token record for a synthetic tool result."""

    return {
        "uncached_input": total,
        "cache_read": 0,
        "cache_write": 0,
        "output": 0,
        "reasoning": 0,
        "provider_only": 0,
        "total": total,
    }


def session_row(
    native: str | None, agent: str, total: int, *, requests: int = 1
) -> dict[str, object]:
    """Build one synthetic urollup `sessions` row."""

    return {
        "thread": f"thr-{PRIVATE_MARKERS[2]}-{native}",
        "session": native,
        "agent": agent,
        "project": PRIVATE_MARKERS[1],
        "requests": {"owned": requests, "ambiguous": 0, "unknown": 0},
        "tokens": tokens(total),
    }


def ccusage_row(session: str, total: int, last_activity: str | None) -> dict[str, object]:
    """Build one synthetic ccusage session row."""

    return {
        "sessionId": session,
        "projectPath": PRIVATE_MARKERS[0],
        "inputTokens": total,
        "outputTokens": 0,
        "cacheCreationTokens": 0,
        "cacheReadTokens": 0,
        "reasoningOutputTokens": 0,
        "totalTokens": total,
        "lastActivity": last_activity,
    }


class LocalAggregateTests(unittest.TestCase):
    def test_private_markers_cannot_reach_the_aggregate(self) -> None:
        daily = {
            "rows": [
                {
                    "date": "2026-09-14",
                    "requests": {"owned": 1, "ambiguous": 0, "unknown": 0},
                    "tokens": tokens(7),
                    "project": PRIVATE_MARKERS[1],
                    "prompt": PRIVATE_MARKERS[4],
                },
                {
                    "date": "2026-09-16",
                    "requests": {"owned": 1, "ambiguous": 0, "unknown": 0},
                    "tokens": tokens(99),
                },
            ]
        }
        report = {
            "coverage": {
                "complete": False,
                "copies_excluded": 2,
                "limit_observations": 3,
                "requests_without_usage": 4,
                "path": PRIVATE_MARKERS[0],
            },
            "totals": {
                "unresolved": {"requests": 5, "request_id": PRIVATE_MARKERS[3]},
                "possible": {"requests": 6},
            },
            "diagnostics": [
                {
                    "code": "source.parse",
                    "count": 2,
                    "detail": " ".join(PRIVATE_MARKERS),
                }
            ],
            "breakdowns": {"model": [{"value": PRIVATE_MARKERS[5]}]},
        }
        sessions = {
            "rows": [
                {
                    "thread": PRIVATE_MARKERS[2],
                    "project": PRIVATE_MARKERS[1],
                    "tokens": tokens(7),
                }
            ]
        }
        aggregate = local_aggregate.build_aggregate(
            daily=daily,
            report=report,
            sessions=sessions,
            version="urollup 0.1.0",
            timezone="UTC",
            cutoff="2026-09-16",
            platform_name="test-platform",
            session_summary={
                "stable": 1,
                "active_or_unknown_excluded": 0,
                "stable_by_agent": {"claude": 1, "codex": 0, "unknown": 0},
            },
        )
        rendered = json.dumps(aggregate, sort_keys=True)
        for marker in PRIVATE_MARKERS:
            self.assertNotIn(marker, rendered)
        self.assertEqual(aggregate["totals"]["tokens"]["total"], 7)
        self.assertEqual(aggregate["per_day"][0]["date"], "2026-09-14")
        self.assertEqual(aggregate["diagnostics"]["by_code"], {"source.parse": 2})

    def test_session_identifiers_remain_inside_the_runner(self) -> None:
        calls: list[list[str]] = []

        def run(args: list[str]) -> dict[str, object]:
            calls.append(args)
            return {"rows": [{"date": "2026-09-14"}]}

        summary = local_aggregate.stable_session_summary(
            {
                "rows": [
                    {
                        "thread": PRIVATE_MARKERS[2],
                        "agent": "claude",
                        "project": PRIVATE_MARKERS[1],
                    }
                ]
            },
            cutoff="2026-09-16",
            timezone="UTC",
            run_urollup=run,
        )
        self.assertIn(PRIVATE_MARKERS[2], calls[0])
        self.assertNotIn(PRIVATE_MARKERS[2], json.dumps(summary))
        self.assertEqual(summary["stable"], 1)

    def test_consent_flag_is_required(self) -> None:
        with redirect_stderr(StringIO()), self.assertRaises(SystemExit):
            local_aggregate.arguments(["--urollup", "urollup"])


class LocalDiffTests(unittest.TestCase):
    def test_diff_record_contains_only_aggregate_dimensions(self) -> None:
        left = {"2026-09-14": {**tokens(100), "private": PRIVATE_MARKERS[4]}}
        right = {"2026-09-14": tokens(90)}
        report = local_diff.build_diff(
            urollup_version="urollup 0.1.0",
            ccusage_version="ccusage 20.0.20",
            platform_name="test-platform",
            timezone="UTC",
            cutoff="2026-09-16",
            urollup_days=left,
            ccusage_days=right,
            session_counts={"matched": 1, "urollup_only": 0, "ccusage_only": 0},
            histogram={key: int(key == "exact") for key in local_diff.HISTOGRAM_KEYS},
        )
        rendered = json.dumps(report, sort_keys=True)
        for marker in PRIVATE_MARKERS:
            self.assertNotIn(marker, rendered)
        self.assertEqual(report["totals"]["delta"]["total"], 10)
        self.assertEqual(report["per_model"][0]["model"], "other")
        self.assertTrue(report["review_threshold_exceeded"]["total"])

    def test_relative_delta_histogram_boundaries_are_integer_exact(self) -> None:
        exact = local_diff.histogram_bucket(tokens(100), tokens(100))
        one_percent = local_diff.histogram_bucket(tokens(101), tokens(100))
        five_percent = local_diff.histogram_bucket(tokens(105), tokens(100))
        over = local_diff.histogram_bucket(tokens(106), tokens(100))
        self.assertEqual(
            [exact, one_percent, five_percent, over],
            ["exact", "within_1_percent", "within_5_percent", "over_5_percent"],
        )

    def test_sessions_join_one_whole_history_run_on_native_ids(self) -> None:
        stable = "2026-09-15T23:59:59.000Z"
        active = "2026-09-16T00:00:01.000Z"
        main = f"{PRIVATE_MARKERS[2]}-main"
        busy = f"{PRIVATE_MARKERS[2]}-busy"
        codex = "019f0000-0000-7000-8000-000000000001"
        busy_codex = "019f0000-0000-7000-8000-000000000002"
        urollup_sessions = {
            "rows": [
                session_row(main, "claude", 100),
                session_row(f"{main}/agent-1", "claude", 5),
                session_row(busy, "claude", 200),
                session_row(f"{busy}/agent-2", "claude", 7),
                session_row(None, "claude", 3),  # an inline sidechain
                session_row(f"{PRIVATE_MARKERS[2]}-empty", "claude", 0, requests=0),
                session_row(codex, "codex", 104),
                {**session_row(None, "unknown", 9), "thread": None},  # the unowned group
            ]
        }
        ccusage = {
            "claude": {
                "sessions": [
                    ccusage_row(main, 100, stable),
                    ccusage_row(busy, 150, active),
                    ccusage_row(f"{PRIVATE_MARKERS[2]}-workflow", 11, stable),
                ]
            },
            "codex": {
                "sessions": [
                    ccusage_row(f"2026/09/14/rollout-2026-09-14T08-00-00-{codex}", 100, stable),
                    ccusage_row(f"2026/09/16/rollout-2026-09-16T08-00-00-{busy_codex}", 1, active),
                ]
            },
        }
        commands: list[list[str]] = []

        def fake_safe_run(command: list[str], *, cwd: Path) -> dict[str, object]:
            del cwd
            commands.append(command)
            return urollup_sessions

        def fake_ccusage(
            binary: Path, *, agent: str, view: str, until: object, **_: object
        ) -> dict[str, object]:
            del binary
            # Sessions compare whole-session totals, so ccusage runs without `--until`.
            self.assertEqual((view, until), ("session", None))
            return ccusage[agent]

        with (
            patch.object(local_diff, "safe_run", side_effect=fake_safe_run),
            patch.object(local_diff, "ccusage_payload", side_effect=fake_ccusage),
        ):
            counts, histogram = local_diff.compare_sessions(
                urollup=Path("urollup"),
                ccusage=Path("ccusage"),
                timezone="UTC",
                cutoff="2026-09-16",
                cwd=Path("."),
            )

        self.assertEqual(len(commands), 1, "urollup reads the whole history once")
        self.assertEqual(commands[0][1:3], ["sessions", "--all"])
        self.assertNotIn("--session", commands[0])
        # Matched: the stable Claude session and the Codex rollout joined by thread ID.
        # urollup-only: the stable session's subagent and the inline sidechain. The busy
        # session, its subagent, the zero-request row and the unowned group are left out.
        # ccusage-only: the stable workflow row; the active Codex rollout is left out.
        self.assertEqual(counts, {"matched": 2, "urollup_only": 2, "ccusage_only": 1})
        self.assertEqual(
            histogram,
            {"exact": 1, "within_1_percent": 0, "within_5_percent": 1, "over_5_percent": 0},
        )
        rendered = json.dumps([counts, histogram])
        for marker in PRIVATE_MARKERS:
            self.assertNotIn(marker, rendered)

    def test_ccusage_sessions_key_by_native_id_and_ended_local_day(self) -> None:
        thread = "019f0000-0000-7000-8000-001100000001"
        undated = "019f0000-0000-7000-8000-001100000002"
        morning = "2026-09-16T06:30:00.000Z"
        payload = {
            "sessions": [
                ccusage_row(f"2026/09/09/rollout-2026-09-09T08-00-00-{thread}", 10, morning),
                ccusage_row(f"rollout-2026-09-09T08-00-00-{thread}", 10, morning),
                ccusage_row("rollout-without-a-thread-id", 1, "2026-09-01T00:00:00.000Z"),
                ccusage_row(undated, 1, None),
            ]
        }

        def read(
            source: dict[str, object], agent: str, zone: str
        ) -> dict[str, local_diff.CcusageSession]:
            return local_diff.ccusage_sessions(
                source, agent=agent, zone=ZoneInfo(zone), cutoff="2026-09-16"
            )

        pacific = read(payload, "codex", "America/Los_Angeles")
        # One rollout at two locations sums, as the fixture parity comparison does.
        self.assertEqual(pacific[thread].metrics["total"], 20)
        self.assertTrue(pacific[thread].stable, "06:30 UTC is still the previous Pacific day")
        self.assertIn("rollout-without-a-thread-id", pacific)
        self.assertFalse(pacific[undated].stable, "no last activity is never stable")
        utc = read(payload, "codex", "UTC")
        self.assertFalse(utc[thread].stable, "the session was active on the cutoff day")
        path_like = f"2026/09/09/rollout-{thread}"
        claude = read({"sessions": [ccusage_row(path_like, 1, None)]}, "claude", "UTC")
        self.assertEqual(list(claude), [path_like], "Claude session IDs are already native")

    def test_invalid_session_rows_fail_without_forwarding_their_values(self) -> None:
        marker = PRIVATE_MARKERS[4]
        payload = {"sessions": [{**ccusage_row("session", 1, None), "outputTokens": marker}]}
        with self.assertRaises(local_diff.LocalDiffError) as raised:
            local_diff.ccusage_sessions(
                payload, agent="claude", zone=ZoneInfo("UTC"), cutoff="2026-09-16"
            )
        self.assertNotIn(marker, str(raised.exception))
        rows = {"rows": [{**session_row("session", "claude", 1), "tokens": {"total": marker}}]}
        stable = {"claude": {"session": local_diff.CcusageSession(tokens(1), True)}}
        with self.assertRaises(local_diff.LocalDiffError) as raised:
            local_diff.join_sessions(rows, stable)
        self.assertNotIn(marker, str(raised.exception))

    def test_consent_flag_is_required(self) -> None:
        with redirect_stderr(StringIO()), self.assertRaises(SystemExit):
            local_diff.arguments(
                ["--urollup", "urollup", "--ccusage-package", "tests/parity/ccusage"]
            )


class LocalCliFailurePrivacyTests(unittest.TestCase):
    def test_os_failures_do_not_print_private_paths_or_tracebacks(self) -> None:
        with tempfile.TemporaryDirectory(prefix="PRIVATE_PATH_MARKER-") as directory:
            root = Path(directory)
            binary = root / "PRIVATE_EXECUTABLE_MARKER"
            binary.write_bytes(b"not an executable")
            package = root / "package"
            ccusage = compare.native_ccusage_binary(package)
            ccusage.parent.mkdir(parents=True)
            ccusage.write_bytes(b"not an executable")
            # These fixtures cannot execute or discover logs. The parity fixture fails
            # native-binary preparation; the aggregate fixture fails process launch.
            for script, extra in (
                ("local_aggregate.py", []),
                ("local_diff.py", ["--ccusage-package", str(package)]),
            ):
                with self.subTest(script=script):
                    result = subprocess.run(
                        [
                            sys.executable, "-B", str(Path(__file__).with_name(script)),
                            "--urollup", str(binary), "--timezone", "UTC",
                            "--consent-local-logs", *extra,
                        ],
                        env={**os.environ, "PYTHONDONTWRITEBYTECODE": "1"},
                        capture_output=True,
                        text=True,
                        check=False,
                        timeout=10,
                    )
                    self.assertEqual(result.returncode, 1)
                    self.assertEqual(result.stdout, "")
                    self.assertIn("operating-system operation failed", result.stderr)
                    self.assertNotIn("Traceback", result.stderr)
                    self.assertNotIn("PRIVATE_", result.stderr)
                    self.assertNotIn(str(root), result.stderr)


if __name__ == "__main__":
    unittest.main()
