from __future__ import annotations

import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
GENERATOR = ROOT / "scripts" / "generate-synthetic-corpus.py"


def run_generator(out_dir: Path, *extra_args: str, env: dict[str, str] | None = None) -> subprocess.CompletedProcess[str]:
    argv = [sys.executable, str(GENERATOR), "--out", str(out_dir), *extra_args]
    return subprocess.run(argv, capture_output=True, text=True, check=False, env=env, timeout=60)


def read_tree(root: Path) -> dict[str, bytes]:
    """Every file under `root`, keyed by its path relative to `root`."""
    return {
        str(path.relative_to(root)): path.read_bytes()
        for path in root.rglob("*")
        if path.is_file()
    }


class SyntheticCorpusDeterminismTest(unittest.TestCase):
    """The generator must be a pure function of its seed and options (design goal:
    scale measurements must be reproducible without reading anyone's real logs)."""

    def test_same_seed_produces_byte_identical_trees(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            first = Path(directory) / "first"
            second = Path(directory) / "second"
            common_args = [
                "--seed",
                "1234",
                "--max-bytes",
                "262144",
                "--days",
                "10",
                "--claude-sessions-per-day",
                "8",
                "--codex-rollouts-per-day",
                "8",
            ]

            first_result = run_generator(first, *common_args)
            second_result = run_generator(second, *common_args)

            self.assertEqual(first_result.returncode, 0, first_result.stderr)
            self.assertEqual(second_result.returncode, 0, second_result.stderr)
            first_tree = read_tree(first)
            second_tree = read_tree(second)
            self.assertGreater(len(first_tree), 0)
            self.assertEqual(set(first_tree), set(second_tree))
            for relative_path, content in first_tree.items():
                self.assertEqual(
                    content, second_tree[relative_path], f"content differs for {relative_path}"
                )

    def test_same_seed_produces_identical_summary_counts(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            first = Path(directory) / "first"
            second = Path(directory) / "second"
            common_args = [
                "--seed",
                "99",
                "--max-bytes",
                "131072",
                "--days",
                "6",
                "--claude-sessions-per-day",
                "6",
                "--codex-rollouts-per-day",
                "6",
            ]

            first_summary = json.loads(run_generator(first, *common_args).stdout)
            second_summary = json.loads(run_generator(second, *common_args).stdout)

            for key in (
                "content_bytes",
                "disk_bytes",
                "files_written",
                "claude_sessions",
                "claude_resumed_sessions",
                "codex_rollouts",
                "codex_forked_rollouts",
                "usage_records",
            ):
                self.assertEqual(first_summary[key], second_summary[key], key)

    def test_different_seeds_produce_different_output(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            one = Path(directory) / "one"
            two = Path(directory) / "two"
            base_args = [
                "--max-bytes",
                "131072",
                "--days",
                "6",
                "--claude-sessions-per-day",
                "6",
                "--codex-rollouts-per-day",
                "6",
            ]

            run_generator(one, "--seed", "1", *base_args)
            run_generator(two, "--seed", "2", *base_args)

            self.assertNotEqual(read_tree(one), read_tree(two))


class SyntheticCorpusByteCapTest(unittest.TestCase):
    """The streamed corpus must never exceed --max-bytes, even at small sizes where a
    single session or rollout is a large fraction of the whole budget."""

    def test_small_cap_is_never_exceeded(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            out_dir = Path(directory) / "corpus"
            max_bytes = 65536
            result = run_generator(
                out_dir,
                "--seed",
                "7",
                "--max-bytes",
                str(max_bytes),
                "--days",
                "50",
                "--claude-sessions-per-day",
                "20",
                "--codex-rollouts-per-day",
                "20",
            )

            self.assertEqual(result.returncode, 0, result.stderr)
            summary = json.loads(result.stdout)
            actual_bytes = sum(path.stat().st_size for path in out_dir.rglob("*") if path.is_file())

            self.assertLessEqual(summary["content_bytes"], max_bytes)
            self.assertLessEqual(actual_bytes, max_bytes)
            # The cap should be put to good use, not abandoned after the first unit.
            self.assertGreater(summary["content_bytes"], max_bytes * 0.3)
            self.assertGreater(summary["usage_records"], 0)
            self.assertGreater(summary["files_written"], 0)

    def test_tiny_cap_still_produces_something_or_cleanly_nothing(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            out_dir = Path(directory) / "corpus"
            max_bytes = 4096
            result = run_generator(
                out_dir,
                "--seed",
                "3",
                "--max-bytes",
                str(max_bytes),
                "--days",
                "5",
                "--claude-sessions-per-day",
                "5",
                "--codex-rollouts-per-day",
                "5",
            )

            self.assertEqual(result.returncode, 0, result.stderr)
            summary = json.loads(result.stdout)
            self.assertLessEqual(summary["content_bytes"], max_bytes)

    def test_zero_max_bytes_is_unlimited_and_bounded_by_counts(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            out_dir = Path(directory) / "corpus"
            result = run_generator(
                out_dir,
                "--seed",
                "11",
                "--max-bytes",
                "0",
                "--days",
                "2",
                "--claude-sessions-per-day",
                "3",
                "--codex-rollouts-per-day",
                "3",
            )

            self.assertEqual(result.returncode, 0, result.stderr)
            summary = json.loads(result.stdout)
            self.assertIsNone(summary["max_bytes"])
            self.assertEqual(summary["claude_sessions"], 6)
            self.assertEqual(summary["codex_rollouts"], 6)


class SyntheticCorpusIndependenceTest(unittest.TestCase):
    """The raw-bytes independence measurement (scripts/measure-scale.py) relies on
    --content-padding-bytes changing corpus size without changing usage records."""

    def test_content_padding_changes_bytes_not_usage_records(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            base = Path(directory) / "base"
            padded = Path(directory) / "padded"
            common_args = [
                "--seed",
                "21",
                "--max-bytes",
                "0",
                "--days",
                "3",
                "--claude-sessions-per-day",
                "5",
                "--codex-rollouts-per-day",
                "5",
            ]

            base_summary = json.loads(
                run_generator(base, *common_args, "--content-padding-bytes", "0").stdout
            )
            padded_summary = json.loads(
                run_generator(padded, *common_args, "--content-padding-bytes", "2000").stdout
            )

            self.assertEqual(base_summary["usage_records"], padded_summary["usage_records"])
            self.assertEqual(base_summary["claude_sessions"], padded_summary["claude_sessions"])
            self.assertEqual(base_summary["codex_rollouts"], padded_summary["codex_rollouts"])
            self.assertGreater(padded_summary["content_bytes"], base_summary["content_bytes"])


class SyntheticCorpusValidationTest(unittest.TestCase):
    def test_zstd_fraction_requires_zstd_on_path(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            out_dir = Path(directory) / "corpus"
            empty_bin = Path(directory) / "empty-bin"
            empty_bin.mkdir()
            env = dict(os.environ)
            env["PATH"] = str(empty_bin)

            result = run_generator(
                out_dir,
                "--seed",
                "1",
                "--max-bytes",
                "65536",
                "--zstd-fraction",
                "0.5",
                env=env,
            )

            self.assertNotEqual(result.returncode, 0)
            self.assertIn("zstd", result.stderr.lower())
            self.assertFalse(out_dir.exists())

    def test_rejects_non_empty_output_directory(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            out_dir = Path(directory) / "corpus"
            out_dir.mkdir()
            (out_dir / "existing-file").write_text("occupied", encoding="utf-8")

            result = run_generator(out_dir, "--seed", "1", "--max-bytes", "65536")

            self.assertNotEqual(result.returncode, 0)

    def test_rejects_out_of_range_fraction(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            out_dir = Path(directory) / "corpus"

            result = run_generator(out_dir, "--seed", "1", "--resume-fraction", "1.5")

            self.assertNotEqual(result.returncode, 0)
            self.assertIn("resume-fraction", result.stderr)


if __name__ == "__main__":
    unittest.main()
