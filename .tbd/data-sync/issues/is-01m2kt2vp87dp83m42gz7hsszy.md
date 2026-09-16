---
type: is
id: is-01m2kt2vp87dp83m42gz7hsszy
title: Implement serve security controls
kind: task
status: closed
priority: 1
version: 4
spec_path: docs/project/specs/active/plan-2026-09-13-urollup-cli-and-web.md
labels:
  - phase-2
dependencies:
  - type: blocks
    target: is-01m2kt2wg2jfgxxa1hz8e16brv
  - type: blocks
    target: is-01m2kt2x6kabafha6zx0gfk0m7
parent_id: is-01m2kt2tyeqvt9kyp7fp7f22zk
created_at: 2026-09-16T00:32:49.863Z
updated_at: 2026-09-16T03:01:56.408Z
closed_at: 2026-09-16T03:01:56.405Z
close_reason: Created twice when the bead-mapping run was interrupted and rerun; uro-vrxt is the original.
resolution: duplicate
duplicate_of: is-01m2ksy86tbsycpsr504v1sanf
---
Phase 2: make the loopback server safe against web pages and other local OS users, since loopback binding alone does not protect usage data. Design §7.3 and Decision 22.

Acceptance:
- Binds only 127.0.0.1, with no other address in the first release, on an OS-assigned port unless --port pins one, which fails if taken; the port is not a secret.
- A 256-bit per-launch token is printed as the only stdout line, `http://127.0.0.1:<port>/#token=<token>`, so it never reaches request lines, Referer headers or logs. The UI moves it to sessionStorage, clears the fragment and sends `Authorization: Bearer <token>` on every API request; the server compares it in constant time and sets no cookies; --open uses a user-only redirect file, keeping the token out of process arguments.
- Static assets load without the token; every API route answers 401 without a valid one.
- 403 unless Host is exactly 127.0.0.1:<port> or localhost:<port>; 403 when a present Origin is null or foreign or a present Sec-Fetch-Site is neither same-origin nor none; OPTIONS gets 403; no CORS headers and no JSONP. Snapshot refresh is the only POST, requires Content-Type: application/json and runs one rebuild at a time.
- API responses set Content-Type: application/json, X-Content-Type-Options: nosniff, Cache-Control: no-store and Cross-Origin-Resource-Policy: same-origin; pages set Content-Security-Policy `default-src 'self'; frame-ancestors 'none'` and Referrer-Policy: no-referrer.
- Raw-HTTP tests cover an attacker hostname, wrong port, missing Host, foreign and null Origin, cross-site Sec-Fetch-Site, OPTIONS, and missing, malformed and wrong tokens; no response carries Access-Control-Allow-*. A browser test confirms a second local origin cannot read responses.
