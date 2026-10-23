---
type: is
id: is-01m5nfwqwjgaf1665db3fhvaan
title: "Alpha phase 1: honest coverage, robust ingestion, correct accounting and memory safety"
kind: epic
status: open
priority: 1
version: 32
spec_path: docs/project/specs/active/plan-2026-10-23-alpha-stabilization.md
labels:
  - alpha
  - milestone-0.1
dependencies:
  - type: blocks
    target: is-01m2ksd5yzhp73gb475pzbvvg6
  - type: blocks
    target: is-01m2ksd74mkre0zjh8rka36a4p
  - type: blocks
    target: is-01m2pkgva22qd452me3cdjc1fq
  - type: blocks
    target: is-01m2ke36qgfvdvnhw7c7v5esm6
  - type: blocks
    target: is-01m5nfw7st3tha6eybrx2nk9z6
parent_id: is-01m5nfw7st3tha6eybrx2nk9z6
child_order_hints:
  - is-01m5nfxvegp2z0h9q6fn9pwrcx
  - is-01m5nfxvv9myk74rnykeebse66
  - is-01m5nfxw6padvafyb4y618se1t
  - is-01m5nfxwhw20wwfxgqy8cfe5cd
  - is-01m5nfxwwzxmszq3kz8r8k5wfw
  - is-01m5nfxx893xn24gwwjrhn16cw
  - is-01m5nfxxm468fbch6p37hbth5x
  - is-01m5nfxy77yhgs0a66cbrvc6pn
  - is-01m5nfxyjt49g8xqk2czcpqn2w
  - is-01m5nfxyz4w0f63yyzebnx5fz8
  - is-01m5nfxzab58df2dg4am0kst14
  - is-01m5nfxznjh4z9tae9kf0nt982
  - is-01m5nfy00szzk1qqk1dywg67gy
  - is-01m5nfyw7nwmf1p3jp20m2d6dv
  - is-01m5nfywk9n5yftj5dp7qmmdz4
  - is-01m5nfywyvq7bnbe3cnd6j0w7d
  - is-01m5nfyxaa16gtt2xyd626xtzg
  - is-01m5nfyxnm3gdagxeffeatxg0w
  - is-01m5nfyy1pqxrjcefgeshyce05
  - is-01m5nfyyd7qwmcqwvraw10w44v
  - is-01m5nfyyrp7vnvb1agc577sy0a
  - is-01m5nfyz44g6tc2p07r7y1bej4
  - is-01m5nfyzfnv9tg6gjkhvjgbegb
  - is-01m5nfyztx3qcad3spte71qn8m
  - is-01m5nfz06mdr8wp3jexmbkxnjg
  - is-01m5nfz0j4w097pdj4cxe4te6z
created_at: 2026-10-23T23:00:17.936Z
updated_at: 2026-10-23T23:02:41.295Z
---
Phase 1 of the alpha stabilization plan: every loss is reported, one bad input never aborts or hangs a run, the known double counts and silent drops are fixed, and memory stays bounded. Children are the 2026-10-23 review findings; existing beads in scope block this epic. Must-fix versus fix-or-document classes are in the plan's Phase 1 section.
