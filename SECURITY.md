# Security Policy

## Supported Versions

Before the first public release, only the current `main` branch receives security fixes.
After `0.1.0`, the latest released minor series and the current development branch are
supported. Older `0.x` lines may receive a fix when the change is low risk, but they are
not a standing compatibility promise.

## Reporting a Vulnerability

Use GitHub’s private vulnerability-reporting form for this repository.
Include the affected version or commit, platform, impact, and the smallest reproduction
you can share safely.
Do not open a public issue for a vulnerability that has not been coordinated.

The maintainer will acknowledge a report, assess severity and affected artifacts, and
coordinate a fix and disclosure timeline with the reporter.

urollup reads coding-agent session logs, which can contain prompts, file paths,
credentials and other private data.
Never attach real session logs, usage summaries or bundles to a report; reduce the
reproduction to a synthetic or sanitized log.
Registry credentials, tokens, private paths and customer data must never appear in a
public advisory or test fixture.

Supply-chain policy for dependencies and tooling is in
[SUPPLY-CHAIN-SECURITY.md](SUPPLY-CHAIN-SECURITY.md).

<!-- This document follows common-doc-guidelines.md.
See github.com/jlevy/practical-prose and review guidelines before editing.
-->
