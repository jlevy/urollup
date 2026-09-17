"""Tests for scripts/check-scale.py's parsing, fitting and threshold logic.

These exercise only the gate's pure decision functions (no subprocess, no urollup
binary, no corpus generation), so they run in a fraction of a second and do not need a
release build. The gate's end-to-end behavior against a real binary is verified
manually (docs/project/qa/scale-measurement.md) and by `make scale-gate` itself, which
this suite deliberately does not invoke.
"""

from __future__ import annotations

import importlib.util
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
CHECK_SCALE_PATH = ROOT / "scripts" / "check-scale.py"


def _load_check_scale():
    """Loads scripts/check-scale.py as a module (hyphenated filename, no package)."""
    spec = importlib.util.spec_from_file_location("urollup_check_scale", CHECK_SCALE_PATH)
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


check_scale = _load_check_scale()
measure_scale = check_scale.measure_scale
MIB = check_scale.MIB


def run_result(
    *,
    refused: bool = False,
    watchdog_killed: bool = False,
    exit_code: int = 0,
    peak_bytes: int | None = 100 * MIB,
    watchdog_peak_kib: int = 100 * 1024,
    error: str | None = None,
) -> "measure_scale.RunResult":
    return measure_scale.RunResult(
        command="daily",
        exit_code=exit_code,
        wall_seconds=1.0,
        peak_bytes=peak_bytes,
        watchdog_killed=watchdog_killed,
        watchdog_peak_kib=watchdog_peak_kib,
        refused=refused,
        error=error,
    )


class ParseSizeListTest(unittest.TestCase):
    def test_accepts_comma_separated_sizes(self) -> None:
        self.assertEqual(check_scale.parse_size_list("4,8,16"), [4, 8, 16])

    def test_rejects_non_integer(self) -> None:
        with self.assertRaisesRegex(ValueError, "comma-separated list of integers"):
            check_scale.parse_size_list("4,eight")

    def test_rejects_fewer_than_two_sizes(self) -> None:
        with self.assertRaisesRegex(ValueError, "at least two sizes"):
            check_scale.parse_size_list("16")

    def test_rejects_duplicate_only_sizes(self) -> None:
        with self.assertRaisesRegex(ValueError, "at least two distinct sizes"):
            check_scale.parse_size_list("8,8")

    def test_rejects_non_positive_sizes(self) -> None:
        with self.assertRaisesRegex(ValueError, "must be positive"):
            check_scale.parse_size_list("0,8")

    def test_rejects_sizes_over_the_generation_safety_cap(self) -> None:
        with self.assertRaisesRegex(ValueError, "safety cap"):
            check_scale.parse_size_list("4,300")


class FitPeakLineTest(unittest.TestCase):
    def test_fits_an_exact_line(self) -> None:
        fit = check_scale.fit_peak_line([1000, 2000, 3000], [10_000_000, 20_000_000, 30_000_000])
        self.assertAlmostEqual(fit.slope, 10_000.0)
        self.assertAlmostEqual(fit.intercept, 0.0, places=3)

    def test_fits_a_line_with_an_intercept(self) -> None:
        # peak_bytes = 5_000_000 + 2_000 * records
        fit = check_scale.fit_peak_line([1000, 4000], [7_000_000, 13_000_000])
        self.assertAlmostEqual(fit.slope, 2_000.0)
        self.assertAlmostEqual(fit.intercept, 5_000_000.0)

    def test_requires_at_least_two_distinct_record_counts(self) -> None:
        with self.assertRaises(ValueError):
            check_scale.fit_peak_line([1000, 1000], [10_000_000, 12_000_000])


class EvaluateIndependenceTest(unittest.TestCase):
    def test_passes_within_the_ceiling(self) -> None:
        outcome = check_scale.evaluate_independence(1.5 * MIB, 64.0, True)
        self.assertTrue(outcome.ok)
        self.assertIn("1.50 MiB", outcome.message)

    def test_fails_over_the_ceiling(self) -> None:
        outcome = check_scale.evaluate_independence(100.0 * MIB, 64.0, True)
        self.assertFalse(outcome.ok)
        self.assertIn("raw-bytes independence violated", outcome.message)
        self.assertIn("100.00 MiB", outcome.message)

    def test_negative_difference_is_judged_by_magnitude(self) -> None:
        outcome = check_scale.evaluate_independence(-100.0 * MIB, 64.0, True)
        self.assertFalse(outcome.ok)

    def test_fails_when_usage_records_were_not_identical(self) -> None:
        outcome = check_scale.evaluate_independence(0.1 * MIB, 64.0, False)
        self.assertFalse(outcome.ok)
        self.assertIn("not valid", outcome.message)

    def test_fails_when_peak_could_not_be_measured(self) -> None:
        outcome = check_scale.evaluate_independence(None, 64.0, True)
        self.assertFalse(outcome.ok)


class EvaluateExtrapolationTest(unittest.TestCase):
    def test_passes_under_both_ceilings(self) -> None:
        fit = check_scale.LineFit(slope=3000.0, intercept=9.0 * MIB)
        outcome = check_scale.evaluate_extrapolation(fit, 8192.0, 48.0)
        self.assertTrue(outcome.ok)

    def test_fails_on_slope_alone(self) -> None:
        fit = check_scale.LineFit(slope=20_000.0, intercept=9.0 * MIB)
        outcome = check_scale.evaluate_extrapolation(fit, 8192.0, 48.0)
        self.assertFalse(outcome.ok)
        self.assertIn("slope", outcome.message)
        self.assertNotIn("intercept 9", outcome.message)

    def test_fails_on_intercept_alone(self) -> None:
        fit = check_scale.LineFit(slope=3000.0, intercept=200.0 * MIB)
        outcome = check_scale.evaluate_extrapolation(fit, 8192.0, 48.0)
        self.assertFalse(outcome.ok)
        self.assertIn("intercept", outcome.message)

    def test_fails_on_both(self) -> None:
        fit = check_scale.LineFit(slope=20_000.0, intercept=200.0 * MIB)
        outcome = check_scale.evaluate_extrapolation(fit, 8192.0, 48.0)
        self.assertFalse(outcome.ok)
        self.assertIn("slope", outcome.message)
        self.assertIn("intercept", outcome.message)

    def test_boundary_value_passes(self) -> None:
        fit = check_scale.LineFit(slope=8192.0, intercept=48.0 * MIB)
        outcome = check_scale.evaluate_extrapolation(fit, 8192.0, 48.0)
        self.assertTrue(outcome.ok)


class EvaluateDailyGateTest(unittest.TestCase):
    def test_passes_a_clean_run(self) -> None:
        outcome = check_scale.evaluate_daily_gate(run_result(), 512)
        self.assertTrue(outcome.ok)
        self.assertIn("512 MiB", outcome.message)

    def test_reports_engine_refusal_clearly(self) -> None:
        outcome = check_scale.evaluate_daily_gate(
            run_result(
                refused=True, error="request observations exceed the reconciliation capacity"
            ),
            512,
        )
        self.assertFalse(outcome.ok)
        self.assertIn("refused", outcome.message)
        self.assertIn("2 GiB", outcome.message)
        self.assertIn("--daily-gate-mib", outcome.message)

    def test_reports_watchdog_kill_clearly(self) -> None:
        outcome = check_scale.evaluate_daily_gate(
            run_result(watchdog_killed=True, watchdog_peak_kib=600 * 1024), 512
        )
        self.assertFalse(outcome.ok)
        self.assertIn("watchdog", outcome.message)
        self.assertIn("600.0 MiB", outcome.message)

    def test_reports_nonzero_exit_clearly(self) -> None:
        outcome = check_scale.evaluate_daily_gate(
            run_result(exit_code=1, error="boom"), 512
        )
        self.assertFalse(outcome.ok)
        self.assertIn("exited 1", outcome.message)
        self.assertIn("boom", outcome.message)

    def test_refusal_takes_priority_over_other_failure_signals(self) -> None:
        # A refused run also reports a nonzero exit; the refusal message must win so the
        # gate never reads as a generic crash (the plan requires this be reported clearly).
        outcome = check_scale.evaluate_daily_gate(
            run_result(
                refused=True,
                exit_code=1,
                error="request observations exceed the reconciliation capacity",
            ),
            512,
        )
        self.assertFalse(outcome.ok)
        self.assertIn("refused", outcome.message)


class ParseArgsTest(unittest.TestCase):
    def test_defaults_parse_cleanly(self) -> None:
        args = check_scale.parse_args([])
        self.assertEqual(args.extrapolation_sizes, [4, 8, 16])
        self.assertEqual(args.daily_gate_mib, 32)

    def test_rejects_oversized_extrapolation_sizes(self) -> None:
        with self.assertRaises(SystemExit):
            check_scale.parse_args(["--extrapolation-sizes-mib", "4,300"])

    def test_rejects_too_few_extrapolation_sizes(self) -> None:
        with self.assertRaises(SystemExit):
            check_scale.parse_args(["--extrapolation-sizes-mib", "16"])

    def test_rejects_oversized_daily_gate_corpus(self) -> None:
        with self.assertRaises(SystemExit):
            check_scale.parse_args(["--daily-gate-mib", "300"])

    def test_rejects_non_positive_daily_gate_corpus(self) -> None:
        with self.assertRaises(SystemExit):
            check_scale.parse_args(["--daily-gate-mib", "0"])

    def test_rejects_padding_multiplier_of_one(self) -> None:
        with self.assertRaises(SystemExit):
            check_scale.parse_args(["--independence-padding-multiplier", "1"])

    def test_accepts_custom_thresholds(self) -> None:
        args = check_scale.parse_args(
            [
                "--extrapolation-slope-ceiling-bytes-per-record",
                "10",
                "--independence-ceiling-mib",
                "0.01",
            ]
        )
        self.assertEqual(args.extrapolation_slope_ceiling_bytes_per_record, 10.0)
        self.assertEqual(args.independence_ceiling_mib, 0.01)


if __name__ == "__main__":
    unittest.main()
