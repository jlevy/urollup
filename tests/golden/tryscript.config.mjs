// Project configuration for the golden corpus, loaded because scripts/run-golden.mjs runs
// tryscript from this directory.
//
// It refuses to run outside the harness, and names the few values that legitimately vary
// between machines and runs, so sessions elide exactly those and nothing else
// (tests/golden/README.md, Normalization).

const harness = process.env.GOLDEN_HOME;
if (!harness || !process.env.UROLLUP_BIN) {
  throw new Error(
    "run the golden corpus through `make golden` or `node scripts/run-golden.mjs`, which build the hermetic environment; a direct tryscript run inherits your HOME and agent session",
  );
}

/** A literal directory prefix that matches with either path separator. */
function directory(value) {
  return value
    .split(/[\\/]/)
    .map((part) => part.replace(/[.*+?^${}()|[\]\\]/g, "\\$&"))
    .join("[\\\\/]");
}

export default {
  patterns: {
    // The hermetic HOME and the fixture root are absolute and differ on every run.
    GOLDEN_HOME: directory(process.env.GOLDEN_HOME),
    GOLDEN_FIXTURES: directory(process.env.GOLDEN_FIXTURES ?? "golden-fixtures-unset"),
    // Wall-clock instants, such as when a report was generated. Instants derived from
    // fixture records are stable and stay literal.
    NOW: "\\d{4}-\\d{2}-\\d{2}T\\d{2}:\\d{2}:\\d{2}(?:\\.\\d+)?Z",
    // Measured elapsed time.
    ELAPSED: "\\d+(?:\\.\\d+)?\\s?(?:ns|µs|us|ms|s)",
    // The package version outside `urollup --version`, which asserts it literally once.
    UROLLUP_VERSION: "\\d+\\.\\d+\\.\\d+(?:-[0-9A-Za-z.-]+)?",
  },
};
