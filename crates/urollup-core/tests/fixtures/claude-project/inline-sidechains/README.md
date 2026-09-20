# inline-sidechains

Tests the legacy Claude Code layout in which consecutive `isSidechain: true` turns live
inside the main transcript and carry no subagent or spawn ID (design §3.2).

- **Layout:** two separate sidechain runs interrupt three main-thread requests.
  The first run has two requests, and the second has one.
- **Identity:** each run becomes one child thread keyed by a fallback digest of its
  first complete record.
  The main thread has two inferred `inline-sidechain` edges.
- **Ownership:** the main thread owns 3 requests, the first sidechain owns 2 and the
  second owns 1. Descendant scope over the main thread therefore holds all 6.
- **Naive grouping:** treating every line as part of the main thread preserves the grand
  token total but loses both child threads and their attribution.
- **Shape source:** agentfdr’s pinned
  [legacy-sidechain reader](https://github.com/kamihork/agentfdr/blob/e0904bf8791f90916fa8db2ce702df93a7caee90/src/subagents.js#L3-L229)
  and
  [tests](https://github.com/kamihork/agentfdr/blob/e0904bf8791f90916fa8db2ce702df93a7caee90/test/subagents.test.js#L21-L127).
  The values and identifiers here are synthetic because no retained local transcript had
  this older layout.

<!-- This document follows common-doc-guidelines.md.
See github.com/jlevy/practical-prose and review guidelines before editing.
-->
