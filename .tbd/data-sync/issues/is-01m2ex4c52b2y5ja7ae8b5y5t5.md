---
type: is
id: is-01m2ex4c52b2y5ja7ae8b5y5t5
title: Plan follow-ups from identity and ownership design
kind: task
status: closed
priority: 2
version: 6
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
delegate: claude-code@spud10.local
labels:
  - plan-spec
dependencies:
  - type: blocks
    target: is-01m2ewe2g3vqcckn9qbntjk5aq
parent_id: is-01m2ew7x8t5sh3h5997e0rz0t2
hold: null
hold_until: null
created_at: 2026-09-14T02:49:53.055Z
updated_at: 2026-09-14T03:53:55.885Z
started_at: 2026-09-14T03:12:38.397Z
closed_at: 2026-09-14T03:53:55.884Z
close_reason: "Applied all cross-section follow-ups: identity keys and HMAC redaction in summaries/bundles, stable account identifiers, --annotation-set and --strict, web UI coverage badges, ID equality/re-derivation/collision and raw/summary/bundle equivalence tests, Phase 3 own-store input, single --source flag, ccusage link, contract gate script."
resolution: null
duplicate_of: null
---
Cross-section follow-ups from wave-one agents. Identity/ownership: bundles/summaries carry identity keys with deterministic redaction; source manifest needs a stable account identifier, not only an alias; CLI annotation-set flag and strict mode on nonzero unresolved; web UI badges for ambiguous/unresolved/possible; Testing Strategy cross-machine/merge-order ID equality, version re-derivation and collision tests; decide on provider receipt import phase item. Summary format: Testing Strategy should say raw/summary/bundle equivalence plus unresolved-overlap cases instead of raw-versus-bundle/database; add a Phase 3 checklist item for reading urollup's own store snapshots; the src- identity key uses a root-relative locator, so confirm analytical IDs never embed names or paths that redaction must remove. Baseline: contract checks run from a tested script, not Makefile shell loops; confirm JSONL exports end with a completion record; confirm the committed web bundle location in the Web UI section. Sources/workflows: confirm the --scope descendants default for session selections in the ledger/accounting text; unify --source versus --input naming in bundle examples; update plan References ccusage link to github.com/ccusage/ccusage.
