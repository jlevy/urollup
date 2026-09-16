# timestamp-precision

A negative case for ccusage 20.0.20, which accepts only 0 or 3 fractional digits and
drops other Claude records.
Design §2.2 accepts any RFC 3339 precision, and §4.3 stores UTC and buckets by 15-minute
UTC intervals.

- **Layout:** five responses stamped `23:59:58Z`, `23:59:59.123456Z`, `23:59:59.5Z`,
  `00:00:00.123456789Z` on the next day, and `20:00:01.25-04:00`, which is
  `2026-09-02T00:00:01.25Z`.
- **Reconciled:** all 5 requests count.
  Three fall on UTC day 2026-09-01 in the `23:45` bucket and two on 2026-09-02 in the
  `00:00` bucket; `expected.json` gives each request’s normalized UTC instant with its
  recorded precision.
- **Naive sum:** ccusage’s parser keeps only the first record: 1 request and 11 output
  tokens against 5 and 65.
- **Shapes:** ccusage’s
  [timestamp parser](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/crates/ccusage-core/src/date_utils.rs#L111-L161).
