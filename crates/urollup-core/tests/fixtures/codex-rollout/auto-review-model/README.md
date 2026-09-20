# auto-review-model

Tests that placeholder model names such as `codex-auto-review` stay as observed with a
`requested` basis, and that a service tier is taken only from the thread’s own
`thread_settings_applied` (design §3.1).

- **Parent:** a `gpt-5.2-codex` turn whose settings event sets `service_tier: priority`.
- **Guardian review:** a `guardian_review` thread with source
  `{"subagent": {"other": "guardian"}}` and `parent_thread_id`; its settings event omits
  `service_tier` and its `turn_context` requests `codex-auto-review`.
- **Reconciled:** 2 requests and 10,760 tokens; the review keeps model
  `codex-auto-review`, has an unknown tier, and links to the parent’s root turn.
- **Naive:** ccusage substitutes a dated fallback model for the placeholder, and
  inheriting the parent’s tier would price the review as priority; tokens match either
  way.
- **Shapes:** ccusage’s
  [placeholder handling](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/codex/src/parser.rs#L583-L627)
  and
  [tier notes](https://github.com/ccusage/ccusage/blob/bd7f89b469aee5635fb2e6722dd6d70f2d113ac1/rust/adapters/codex/src/README.md#L21-L28);
  Codex
  [guardian sessions](https://github.com/openai/codex/blob/6b9826e3aa83b1a5947db50f4332cb9c65f1b340/codex-rs/core/src/codex_delegate.rs#L82-L116).
