# Gate Probes

Each `make check` gate has a committed violation here that it must reject.
`make gate-proofs` runs `scripts/prove-gates.mjs`, which for every probe in
[probes.json](probes.json) copies the repository into a scratch directory, applies the
probe’s edits, runs the gate’s make target exactly as `make check` does, and requires a
nonzero exit whose output matches the probe’s expected diagnostic.

- **Probe files** live in `files/` with a `.probe` suffix, so no formatter, linter or
  build ever picks them up in the real tree.
- **Edits** append, create, replace, delete or substitute text.
  A substitution must match exactly once, so a probe that has gone stale fails the proof
  rather than proving a gate against an unchanged tree.
- **Coverage** is enforced: every prerequisite of `make check` needs a probe or a reason
  under `unprobed`, and `scripts/prove-gates.test.mjs` checks the committed manifest
  against the Makefile.
- **Isolation:** probes share `target/gate-proofs` for compiled dependencies and link
  the installed `node_modules`; they pass `NPM=false` so no probe can reinstall
  packages.

Run one probe while developing with `node scripts/prove-gates.mjs --only <id>`, list
them with `--list`, and keep a failing probe’s scratch copy with `--keep`.

<!-- This document follows common-doc-guidelines.md.
See github.com/jlevy/practical-prose and review guidelines before editing.
-->
