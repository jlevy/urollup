---
type: is
id: is-01m2ksy86tbsycpsr504v1sanf
title: Implement serve security controls
kind: task
status: open
priority: 1
version: 3
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - phase-2
dependencies:
  - type: blocks
    target: is-01m2ksya0k0wkq02e2gfws2ndh
  - type: blocks
    target: is-01m2ksyczqzy351xyfv1c2qx8r
parent_id: is-01m2ksy5yrxn48thxc311sqsv6
created_at: 2026-09-16T00:30:18.840Z
updated_at: 2026-09-16T00:30:23.725Z
---
Phase 2: make the loopback server safe against web pages and other local OS users, since loopback binding alone does not protect usage data. Design §7.3 and Decision 22.

Acceptance:
- Binds only 127.0.0.1, with no other address in the first release, on an OS-assigned port unless --port pins one, which fails if taken; the port is not a secret.
- A 256-bit per-launch token is printed as the only stdout line, `http://127.0.0.1:<port>/#token=<token>`, so it never reaches request lines, Referer headers or logs. The UI moves it to sessionStorage, clears the fragment and sends `Authorization: Bearer <token>` on every API request; the server compares it in constant time and sets no cookies; --open uses a user-only redirect file, keeping the token out of process arguments.
- Static assets load without the token; every API route answers 401 without a valid one.
- 403 unless Host is exactly 127.0.0.1:<port> or localhost:<port>; 403 when a present Origin is null or foreign or a present Sec-Fetch-Site is neither same-origin nor none; OPTIONS gets 403; no CORS headers and no JSONP. Snapshot refresh is the only POST, requires Content-Type: application/json and runs one rebuild at a time.
- API responses set Content-Type: application/json, X-Content-Type-Options: nosniff, Cache-Control: no-store and Cross-Origin-Resource-Policy: same-origin; pages set Content-Security-Policy `default-src 'self'; frame-ancestors 'none'` and Referrer-Policy: no-referrer.
- Raw-HTTP tests cover an attacker hostname, wrong port, missing Host, foreign and null Origin, cross-site Sec-Fetch-Site, OPTIONS, and missing, malformed and wrong tokens; no response carries Access-Control-Allow-*. A browser test confirms a second local origin cannot read responses.
