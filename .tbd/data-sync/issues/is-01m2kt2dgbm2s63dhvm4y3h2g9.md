---
type: is
id: is-01m2kt2dgbm2s63dhvm4y3h2g9
title: Add --prices overrides, prices.yaml and pricing-basis reporting
kind: task
status: closed
priority: 1
version: 4
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - phase-1
  - milestone-0.4
dependencies:
  - type: blocks
    target: is-01m2kt2f8c62j2vaw2k4swfaj6
  - type: blocks
    target: is-01m2kt3aw32tw7x1p9q5f893yg
parent_id: is-01m2ke55tfcy92ct96d9446j5p
created_at: 2026-09-16T00:32:35.337Z
updated_at: 2026-09-16T03:01:54.980Z
closed_at: 2026-09-16T03:01:54.979Z
close_reason: Created twice when the bead-mapping run was interrupted and rerun; uro-640p is the original.
resolution: duplicate
duplicate_of: is-01m2ksx04swwn45tchzcjqhr0g
---
Milestone 0.4: user rate overrides and the recorded pricing basis. Design §4.5 and §2.1 (config directory).

Acceptance:
- The repeatable --prices option reads YAML files under the same urollup:PriceTable/v1 contract; when none is passed, prices.yaml in the platform config directory is read.
- Override rows take precedence over bundled rows for the dates they cover; overlapping override rows across files are rejected like overlaps within one table; amounts they price are labeled configured rates, not list prices.
- Reports record the pricing basis: bundled table version and review date plus each override file's fingerprint, carried in the normalized QuerySpec and in JSON output.
- Pricing never makes network requests, in any code path.
