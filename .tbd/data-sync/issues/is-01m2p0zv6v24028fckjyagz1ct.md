---
type: is
id: is-01m2p0zv6v24028fckjyagz1ct
title: Bound large-corpus memory during log ingestion
kind: bug
status: closed
priority: 0
version: 6
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - milestone-0.1
  - performance
  - testing
  - memory
dependencies: []
parent_id: is-01m2ke36qgfvdvnhw7c7v5esm6
created_at: 2026-09-16T21:11:59.962Z
updated_at: 2026-09-19T07:40:01.512Z
closed_at: 2026-09-19T07:40:01.511Z
close_reason: "Temporary 512 MiB input guard, session-family preselection and RSS watchdog shipped on PR #8 (4568059, efa5ef8). Whole-history replacement is uro-o6x5."
resolution: null
duplicate_of: null
---
A consented local all-log acceptance run caused severe memory pressure and the Codex desktop task restarted. Remove raw-payload retention in both adapters, restrict default Codex discovery to rollout directories, reduce avoidable reconciliation duplication, preserve manifest and fixture behavior, and add selection-aware family discovery so --current and exact selections parse only the relevant family before applying a temporary capacity guard. Validate with bounded real and synthetic corpora. Multi-gigabyte --all streaming/spill is tracked separately by uro-o6x5.

## Notes

2026-09-16: raw Codex and Claude payload retention removed; default Codex discovery narrowed; reconciliation identity duplication reduced. Core/CLI tests pass. 128 MiB skipped Codex stress case used about 3.8 MiB RSS; a 204 MiB bounded Codex slice used about 97,760 KiB; a 472,936,448-byte Claude project used about 443,904 KiB. The full approximately 15 GB corpus was not rerun. CLI currently fails before ingestion above a conservative 512 MiB estimated-decoded-input bound. Selection-aware family discovery is in progress; bounded all-log streaming remains uro-o6x5.

2026-09-16 (evening): Forensics corrected. Second incident at 14:11:34 PT was not a QA command: backticks around urollup daily --all inside a double-quoted tbd create --description made bash run the pre-budget global binary over the default corpus; macOS JetsamEvent-2026-09-16-142753 (snapshot 14:12:52 after its 901 s timeDelta) shows a 23.2 GB footprint (4.2 GB resident, 19.0 GB compressed) after 79 s and 88 jettisoned daemons. Budgets and preflight committed in 4568059 on PR #8; global developer binary reinstalled from that build. Guard verified on the real default corpus: daily --all, sessions --all and report --all --group-by project,model exit 1 with the safety-limit error in 0.4-1.4 s at about 11 MiB peak RSS; report --current exits 0 at about 11 MiB. Full-corpus aggregation remains uro-o6x5. Details in docs/project/qa/qa-report-2026-09-16-milestone-0.1.md.
