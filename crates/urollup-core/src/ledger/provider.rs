//! Versioned vendor inference for report grouping.
//!
//! Provider is a request facet distinct from the coding-agent surface. Claude Code and
//! Codex each have one observed vendor. Cursor multiplexes vendors, so its provider is
//! inferred from the model attributed to the measurement. The same table will classify a
//! Gemini CLI model as `google` whether that model arrived through Cursor or a later
//! Gemini adapter.

use crate::ledger::entities::Basis;
use crate::selection::Agent;

/// Registry version of the catalog-family table. Bump when a mapping rule changes.
pub const PROVIDER_MAP_VERSION: &str = "2026-09-20";

/// Infers the usage vendor for one request from its agent and attributed model.
///
/// `model` is the native label attached to the measurement (Cursor picker/catalog id,
/// Claude served id, Codex requested id). It is never the agent token.
pub fn provider_for(agent: Agent, model: Option<&str>) -> Basis<&'static str> {
    match agent {
        Agent::Claude => Basis::Observed("anthropic"),
        Agent::Codex => Basis::Observed("openai"),
        Agent::Pi => Basis::Unknown,
        Agent::Cursor => infer_catalog_provider(model),
    }
}

/// Maps a catalog or picker model family to a vendor token.
///
/// Auto (`default`) hides the served model, so its provider stays unknown. Fable stays
/// unmapped until a `claude-fable-*` or `fable-*` value is locked.
pub fn infer_catalog_provider(model: Option<&str>) -> Basis<&'static str> {
    let Some(model) = model.filter(|model| !model.is_empty()) else {
        return Basis::Unknown;
    };
    let normalized = model.strip_prefix("cursor-").unwrap_or(model).to_ascii_lowercase();
    if normalized == "default" || normalized == "auto" {
        return Basis::Unknown;
    }
    if normalized.contains("fable") {
        return Basis::Unknown;
    }
    if normalized.starts_with("grok") || normalized.starts_with("composer") {
        return Basis::Inferred("cursor");
    }
    if normalized.starts_with("claude") {
        return Basis::Inferred("anthropic");
    }
    if normalized.starts_with("gpt")
        || normalized.starts_with("o1")
        || normalized.starts_with("o3")
        || normalized.starts_with("o4")
    {
        return Basis::Inferred("openai");
    }
    if normalized.starts_with("gemini") {
        return Basis::Inferred("google");
    }
    if normalized.starts_with("kimi") {
        return Basis::Inferred("moonshot");
    }
    if normalized.starts_with("glm") {
        return Basis::Inferred("zai");
    }
    Basis::Unknown
}

#[cfg(test)]
mod tests {
    use super::{infer_catalog_provider, provider_for};
    use crate::ledger::entities::Basis;
    use crate::selection::Agent;

    #[test]
    fn single_vendor_agents_are_observed() {
        assert_eq!(
            provider_for(Agent::Claude, Some("claude-opus-4-5")),
            Basis::Observed("anthropic")
        );
        assert_eq!(provider_for(Agent::Codex, Some("gpt-5")), Basis::Observed("openai"));
        assert_eq!(provider_for(Agent::Pi, Some("any")), Basis::Unknown);
    }

    #[test]
    fn cursor_maps_catalog_families_and_leaves_auto_unknown() {
        assert_eq!(
            infer_catalog_provider(Some("cursor-grok-4.6-xhigh-fast")),
            Basis::Inferred("cursor")
        );
        assert_eq!(infer_catalog_provider(Some("composer-2.5-fast")), Basis::Inferred("cursor"));
        assert_eq!(
            infer_catalog_provider(Some("claude-4.5-opus-high-thinking")),
            Basis::Inferred("anthropic")
        );
        assert_eq!(infer_catalog_provider(Some("gpt-5.1-codex-high")), Basis::Inferred("openai"));
        assert_eq!(infer_catalog_provider(Some("gemini-3-pro")), Basis::Inferred("google"));
        assert_eq!(infer_catalog_provider(Some("kimi-k2-instruct")), Basis::Inferred("moonshot"));
        assert_eq!(infer_catalog_provider(Some("glm-5.2")), Basis::Inferred("zai"));
        assert_eq!(infer_catalog_provider(Some("default")), Basis::Unknown);
        assert_eq!(infer_catalog_provider(Some("claude-fable-max")), Basis::Unknown);
        assert_eq!(infer_catalog_provider(None), Basis::Unknown);
    }

    #[test]
    fn gemini_family_is_google_whether_or_not_the_agent_is_cursor() {
        assert_eq!(infer_catalog_provider(Some("gemini-3-pro-preview")), Basis::Inferred("google"));
        assert_eq!(provider_for(Agent::Cursor, Some("gemini-3-pro")), Basis::Inferred("google"));
    }
}
