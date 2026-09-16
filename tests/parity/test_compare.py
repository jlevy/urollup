"""Tests for the ccusage parity comparator's failure rules."""

from __future__ import annotations

import tempfile
import unittest
from datetime import date
from pathlib import Path
from unittest.mock import patch

import compare


class ParityComparatorTests(unittest.TestCase):
    def difference(self, **changes: object) -> compare.Difference:
        values: dict[str, object] = {
            "case": "claude-project/example",
            "report": "claude-daily",
            "variant": "full",
            "timezone": "UTC",
            "key": "2026-09-01",
            "metric": "output",
            "urollup": 7,
            "ccusage": 3,
            "delta": 4,
        }
        values.update(changes)
        return compare.Difference(**values)  # type: ignore[arg-type]

    def entry(self, **changes: object) -> dict[str, object]:
        values: dict[str, object] = {
            "id": "known-difference",
            "case": "claude-project/example",
            "cause": "ccusage-bug",
            "citation": "docs/research.md#fact",
            "design": "docs/urollup-design.md#rule",
            "retirement": "Remove when the pinned upstream release fixes the parser.",
            "difference": [
                {
                    "report": "claude-daily",
                    "variant": "full",
                    "timezone": "UTC",
                    "key": "2026-09-01",
                    "metric": "output",
                    "expected_delta": 4,
                    "expected_matches": 1,
                }
            ],
        }
        values.update(changes)
        return values

    def test_row_and_metric_differences_are_exact(self) -> None:
        differences = compare.difference_rows(
            case="x",
            report="daily",
            variant="full",
            timezone="UTC",
            urollup={"a": {"output": 7}, "only-left": {"output": 9}},
            ccusage={"a": {"output": 3}, "only-right": {"output": 5}},
        )

        self.assertEqual(
            [(item.key, item.metric, item.delta) for item in differences],
            [("a", "output", 4), ("only-left", "row", 1), ("only-right", "row", -1)],
        )

    def test_unmatched_wrong_delta_double_match_and_stale_entries_fail(self) -> None:
        difference = self.difference()
        _, unmatched = compare.validate_ledger([difference], {"schema": 1})
        _, wrong = compare.validate_ledger(
            [difference],
            {
                "schema": 1,
                "entry": [
                    self.entry(
                        difference=[
                            {
                                "metric": "output",
                                "expected_delta": 5,
                                "expected_matches": 1,
                            }
                        ]
                    )
                ],
            },
        )
        _, doubled = compare.validate_ledger(
            [difference],
            {"schema": 1, "entry": [self.entry(), self.entry(id="duplicate-match")]},
        )
        _, stale = compare.validate_ledger([], {"schema": 1, "entry": [self.entry()]})

        self.assertIn("matched 0 ledger entries", unmatched[0])
        self.assertTrue(any("matched 0 ledger entries" in error for error in wrong))
        self.assertIn("matched 2 ledger entries", doubled[0])
        self.assertIn("matched 0 differences; expected 1", stale[0])

    def test_ledger_rule_match_count_detects_overbroad_patterns(self) -> None:
        differences = [self.difference(), self.difference(timezone="America/Los_Angeles")]
        _, errors = compare.validate_ledger(
            differences,
            {
                "schema": 1,
                "entry": [
                    self.entry(
                        difference=[
                            {
                                "metric": "output",
                                "expected_delta": 4,
                                "expected_matches": 1,
                            }
                        ]
                    )
                ],
            },
        )

        self.assertIn("matched 2 differences; expected 1", errors[-1])

    def test_ledger_requires_evidence_and_retirement_metadata(self) -> None:
        for field in ("citation", "design", "retirement"):
            with self.subTest(field=field), self.assertRaises(compare.ParityError):
                compare.validate_ledger(
                    [self.difference()], {"schema": 1, "entry": [self.entry(**{field: ""})]}
                )

    def test_session_keys_and_since_bounds_are_normalized(self) -> None:
        thread = "019f0000-0000-7000-8000-000100000001"
        self.assertEqual(
            compare.normalize_session_key(f"2026/09/02/rollout-{thread}", [thread]), thread
        )
        self.assertEqual(compare.inclusive_until_to_exclusive("20260902"), date(2026, 9, 3))

    def test_native_binary_paths_cover_reviewed_platforms(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            package = Path(temporary)
            with patch("platform.system", return_value="Linux"), patch(
                "platform.machine", return_value="aarch64"
            ):
                path = compare.native_ccusage_binary(package)
            self.assertEqual(
                path.relative_to(package).as_posix(),
                "node_modules/@ccusage/ccusage-linux-arm64/bin/ccusage",
            )


if __name__ == "__main__":
    unittest.main()
