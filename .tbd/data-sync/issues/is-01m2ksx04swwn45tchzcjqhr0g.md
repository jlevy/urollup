---
type: is
id: is-01m2ksx04swwn45tchzcjqhr0g
title: Add --prices overrides, prices.yaml and pricing-basis reporting
kind: task
status: open
priority: 1
version: 3
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - phase-1
  - milestone-0.4
dependencies:
  - type: blocks
    target: is-01m2ksx6f9w2rep42kwatz76k5
  - type: blocks
    target: is-01m2ksz8r23b3kmymq8g06c90a
parent_id: is-01m2ke55tfcy92ct96d9446j5p
created_at: 2026-09-16T00:29:37.797Z
updated_at: 2026-09-16T00:30:52.161Z
---
Milestone 0.4: user rate overrides and the recorded pricing basis. Design §4.5 and §2.1 (config directory).

Acceptance:
- The repeatable --prices option reads YAML files under the same urollup:PriceTable/v1 contract; when none is passed, prices.yaml in the platform config directory is read.
- Override rows take precedence over bundled rows for the dates they cover; overlapping override rows across files are rejected like overlaps within one table; amounts they price are labeled configured rates, not list prices.
- Reports record the pricing basis: bundled table version and review date plus each override file's fingerprint, carried in the normalized QuerySpec and in JSON output.
- Pricing never makes network requests, in any code path.
