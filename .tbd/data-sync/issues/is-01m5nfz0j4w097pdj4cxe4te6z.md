---
type: is
id: is-01m5nfz0j4w097pdj4cxe4te6z
title: Look up Claude quota-limit owners by key instead of scanning every thread
kind: task
status: open
priority: 3
version: 1
spec_path: docs/project/specs/active/plan-2026-10-23-alpha-stabilization.md
labels:
  - performance
  - alpha
  - milestone-0.1
dependencies: []
parent_id: is-01m5nfwqwjgaf1665db3fhvaan
created_at: 2026-10-23T23:01:32.355Z
updated_at: 2026-10-23T23:01:32.355Z
---
append_limits (claude_project.rs:1652-1657) derives an identity and scans every thread ID linearly for each record with quotaLimits, so cost grows with records x threads (synthetic: about 1.2 s of extra CPU at 3,000 sessions and 180,000 quota records). Fix: ids.get(&record.native_owner()).
