---
type: is
id: is-01m5nfxw6padvafyb4y618se1t
title: Carry coverage, unresolved usage and partial reasons in daily, sessions and selected reports
kind: bug
status: open
priority: 1
version: 2
spec_path: docs/project/specs/active/plan-2026-10-23-alpha-stabilization.md
labels:
  - coverage
  - alpha
  - milestone-0.1
dependencies:
  - type: blocks
    target: is-01m5nfxwhw20wwfxgqy8cfe5cd
parent_id: is-01m5nfwqwjgaf1665db3fhvaan
created_at: 2026-10-23T23:00:55.125Z
updated_at: 2026-10-23T23:02:37.713Z
---
Ledger review findings 1, 2, 5 and 8 (2026-10-23). (1) DailyDocument and SessionsDocument (query.rs:333-358) have no coverage, complete, unresolved, possible or requests-without-usage fields, so daily and sessions present partial data as complete-looking numbers, contrary to design §6.4. (2) Under a session selection, aggregate.rs:246-264 never fills totals.unresolved, SelectionTotals has no unresolved measure, and Completeness::Partial(reasons) collapses to a bool, so --session can print 'partial' with every counter zero. (3) Selection-mode copies_excluded and subject-less diagnostics are summed over the whole ledger (aggregate.rs:292-322). (4) Requests with no known owner drop out of every selection without a count. Fix: one coverage block (complete, reasons, gap counts, unresolved and possible side totals, requests without usage) computed per selection in all three documents, plus a per-row partial flag on sessions rows; regression tests replacing the bare !complete assertion at aggregate.rs:705-711. Related: uro-1vg5, uro-akgu.
