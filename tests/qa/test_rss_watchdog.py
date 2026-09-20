from __future__ import annotations

import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
WATCHDOG = ROOT / "scripts" / "run-rss-watchdog.py"


class RssWatchdogTest(unittest.TestCase):
    def run_watchdog(
        self, limit_mib: int, child: str
    ) -> tuple[subprocess.CompletedProcess[str], dict[str, object]]:
        with tempfile.TemporaryDirectory() as directory:
            report = Path(directory) / "report.json"
            result = subprocess.run(
                [
                    sys.executable,
                    str(WATCHDOG),
                    "--limit-mib",
                    str(limit_mib),
                    "--interval-ms",
                    "10",
                    "--report",
                    str(report),
                    "--",
                    sys.executable,
                    "-c",
                    child,
                ],
                check=False,
                capture_output=True,
                text=True,
                timeout=10,
            )
            return result, json.loads(report.read_text(encoding="utf-8"))

    def test_preserves_child_failure_and_writes_metrics(self) -> None:
        result, report = self.run_watchdog(256, "raise SystemExit(7)")

        self.assertEqual(result.returncode, 7)
        self.assertEqual(report["exit_code"], 7)
        self.assertFalse(report["killed_for_rss"])
        self.assertGreaterEqual(report["peak_rss_kib"], 0)
        self.assertNotIn("command", report)

    def test_kills_process_group_above_limit(self) -> None:
        result, report = self.run_watchdog(
            1,
            "import time; payload = bytearray(8 * 1024 * 1024); time.sleep(2)",
        )

        self.assertEqual(result.returncode, 97)
        self.assertEqual(report["exit_code"], 97)
        self.assertTrue(report["killed_for_rss"])
        self.assertGreater(report["peak_rss_kib"], report["limit_rss_kib"])

    def test_supervises_descendant_after_group_leader_exits(self) -> None:
        result, report = self.run_watchdog(
            1,
            "import subprocess, sys; subprocess.Popen([sys.executable, '-c', "
            "'import time; payload = bytearray(8 * 1024 * 1024); time.sleep(2)'])",
        )

        self.assertEqual(result.returncode, 97)
        self.assertTrue(report["killed_for_rss"])

    def test_report_publication_failure_is_a_watchdog_error(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            parent_file = Path(directory) / "not-a-directory"
            parent_file.write_text("occupied", encoding="utf-8")
            result = subprocess.run(
                [
                    sys.executable,
                    str(WATCHDOG),
                    "--limit-mib",
                    "256",
                    "--report",
                    str(parent_file / "report.json"),
                    "--",
                    sys.executable,
                    "-c",
                    "pass",
                ],
                check=False,
                capture_output=True,
                text=True,
                timeout=10,
            )

        self.assertEqual(result.returncode, 98)
        self.assertIn("cannot publish metrics", result.stderr)
        self.assertNotIn("Traceback", result.stderr)


if __name__ == "__main__":
    unittest.main()
