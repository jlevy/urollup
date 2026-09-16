---
type: is
id: is-01m2ksw03jk2kpr3gzfh29r0qg
title: Implement the export allow-list and redaction profiles
kind: task
status: open
priority: 1
version: 2
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - phase-1
  - milestone-0.2
dependencies:
  - type: blocks
    target: is-01m2ksw5c3g28x3wk3g9hhfybr
parent_id: is-01m2ke45tasy262pas37jxwss5
created_at: 2026-09-16T00:29:05.009Z
updated_at: 2026-09-16T00:29:10.400Z
---
Milestone 0.2: the strict export strip policy and the three --redact profiles. Design §2.4, §5.5, §8.4 and Decision 19.

Acceptance:
- The per-dialect export policy is a versioned strict allow-list: only enumerated keys and paths keep values (types, IDs, timestamps, models, usage objects, stop reasons, tool names, limit fields and version fields, with path-like fields following the redaction profile); every other string, array or object value becomes a {type, bytes} stub whatever its length, so text under a key a new agent release adds never reaches an export.
- --redact paths (default) removes absolute paths, working directories and path-shaped locators and needs no key; names also replaces project names, account aliases and account identifiers with keyed HMAC-SHA-256 labels; native-ids also drops native ID fields and labels native IDs inside keys.
- The key comes from UROLLUP_REDACTION_KEY or a file named by --redaction-key-file; an opt-in profile with neither exits 2 with a message naming both. Artifacts record the profile and key fingerprint, never the key.
- Redaction never affects deduplication, which uses analytical IDs; project is always exported as a plain name resolved by the §2.1 rule, which names then labels.
- Grouping by a property any input labels requires every input to share a key fingerprint, else exit 2.
- Privacy tests: a leak fixture whose record carries text under an unknown key yields no plaintext in the summary, bundle or records table; validation diagnostics never echo values from redacted or stripped fields; default paths exports contain no absolute path, working directory or path-shaped locator; agent, dialect, model, effort, project, account alias, time buckets and tool categories group identically across two machines.
