---
type: is
id: is-01m2ksz7d707rz38cfe8gh2xbj
title: Run the 0.1.0 release checklist and shadow-mode validation
kind: task
status: open
priority: 2
version: 3
spec_path: docs/project/specs/active/plan-2026-09-16-first-release-publishing.md
labels:
  - release
dependencies: []
parent_id: is-01m2ksyxh3c3qvgg8qhp5fgbsx
created_at: 2026-09-16T00:30:50.788Z
updated_at: 2026-09-16T18:22:58.900Z
---
Final product-acceptance gate for the first public alpha.\n\nAcceptance:\n- Milestone 0.1 local acceptance, including the privacy-tested aggregate mode, pinned-ccusage diff and G1, is complete.\n- make check and the credential-free release rehearsal pass on the exact candidate commit.\n- The archive, wheel and Cargo package smoke tests pass from empty environments, and the complete artifact manifest is reviewed.\n- urollup runs in shadow mode against retained logs and existing reports; supported 0.1 reports reconcile and every difference is explained without recording private content, paths or identifiers.\n- The checklist is limited to the 0.1 command surface. Later Phase 1 feature-matrix coverage and provider-export validation remain separate pre-1.0 work.
