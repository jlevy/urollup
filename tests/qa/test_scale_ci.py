"""The scale workload must execute in CI, beyond testing its decision helpers."""

import re
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


def check_scale_job(workflow: str) -> None:
    """Check the deliberately explicit scale job contract without a YAML dependency."""
    match = re.search(r"^  scale:\n(?P<body>(?:[ \t].*\n|\n)*?)(?=^  \S|\Z)", workflow, re.MULTILINE)
    if match is None:
        raise ValueError("CI must execute the scale workload in a dedicated scale job")
    body = match.group("body")
    required = [
        "    needs: supply-chain",
        "    runs-on: ${{ matrix.os }}",
        "        os: [ubuntu-24.04, macos-15]",
        "        shell: bash",
        "        run: |",
        "          set -o pipefail",
        "          mkdir -p target",
        "          make scale-gate 2>&1 | tee target/scale-gate.log",
        "        if: always()",
        "          name: scale-gate-${{ matrix.os }}",
        "          path: target/scale-gate.log",
        "          if-no-files-found: error",
    ]
    for line in required:
        if line not in body.splitlines():
            raise ValueError(f"scale job contract missing: {line.strip()}")
    # Only artifact publication may be conditional; never silently skip the workload
    # or convert its failure into a green job.
    conditions = re.findall(r"^[ \t]+if:.*$", body, re.MULTILINE)
    if conditions != ["        if: always()"] or "continue-on-error:" in body:
        raise ValueError("scale workload must run unconditionally and propagate failure")
    if not re.search(r"^        uses: actions/upload-artifact@[a-f0-9]{40} ", body, re.MULTILINE):
        raise ValueError("scale job must archive its output with a pinned action")


class ScaleCiContractTest(unittest.TestCase):
    def test_scale_workload_is_live_and_fail_closed(self) -> None:
        workflow = (ROOT / ".github/workflows/ci.yml").read_text()
        check_scale_job(workflow)
        mutations = [
            ("  scale:\n", "  unused-scale:\n"),
            ("          make scale-gate 2>&1", "          make qa-tool-tests 2>&1"),
            ("          set -o pipefail\n", ""),
            ("          path: target/scale-gate.log", "          path: target/missing.log"),
            ("    name: Synthetic scale", "    if: false\n    name: Synthetic scale"),
        ]
        for before, after in mutations:
            with self.subTest(mutation=before):
                self.assertIn(before, workflow)
                with self.assertRaises(ValueError):
                    check_scale_job(workflow.replace(before, after))


if __name__ == "__main__":
    unittest.main()
