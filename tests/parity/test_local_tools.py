"""Privacy and aggregation tests for maintainer-only local acceptance tools."""

from __future__ import annotations

import json
import unittest
from contextlib import redirect_stderr
from io import StringIO
from pathlib import Path
from unittest.mock import patch

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

    def test_active_sessions_are_excluded_from_the_session_histogram(self) -> None:
        ccusage_rows = {
            "stable-native": tokens(100),
            "active-native": tokens(200),
        }

        def fake_safe_run(
            command: list[str], *, cwd: Path, allow_failure: bool = False
        ) -> dict[str, object]:
            del cwd, allow_failure
            if "--session" not in command:
                return {"rows": []}
            selector = command[command.index("--session") + 1]
            return {
                "rows": [
                    {
                        "thread": f"thr-{selector}",
                        "tokens": tokens(100 if selector == "stable-native" else 200),
                    }
                ]
            }

        with (
            patch.object(
                local_diff,
                "stable_urollup_threads",
                return_value={"thr-stable-native"},
            ),
            patch.object(local_diff, "safe_run", side_effect=fake_safe_run),
            patch.object(local_diff, "ccusage_payload", return_value={}),
            patch.object(local_diff.compare, "parse_ccusage_rows", return_value=ccusage_rows),
        ):
            counts, histogram = local_diff.compare_sessions(
                urollup=Path("urollup"),
                ccusage=Path("ccusage"),
                timezone="UTC",
                until=local_diff.date(2026, 9, 15),
                cutoff="2026-09-16",
                cwd=Path("."),
            )

        # Both agent loops see the two synthetic rows. Only the stable session counts.
        self.assertEqual(counts, {"matched": 2, "urollup_only": 0, "ccusage_only": 0})
        self.assertEqual(histogram["exact"], 2)
        self.assertEqual(sum(histogram.values()), 2)

    def test_consent_flag_is_required(self) -> None:
        with redirect_stderr(StringIO()), self.assertRaises(SystemExit):
            local_diff.arguments(
                ["--urollup", "urollup", "--ccusage-package", "tests/parity/ccusage"]
            )


if __name__ == "__main__":
    unittest.main()
