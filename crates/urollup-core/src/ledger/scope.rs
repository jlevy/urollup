//! Key kinds, key scope and identity basis (design §3.6, Key Scope and Identity Basis).
//!
//! An adapter declares a [`KeySpec`] for every key kind it builds: the ID prefix, the
//! precedence of the kind among that prefix's keys, whether the kind is a native or a
//! fallback key, the uniqueness scope of the native ID, and the role of each component.
//! [`KeySpec::validate`] enforces the scope rules the design states, so a spec that would
//! mint unstable or over-scoped IDs fails when it is declared rather than when data is
//! merged:
//!
//! - A key includes only its scope's namespace components: a provider-, agent- or
//!   account-scoped kind names its namespace, and only an account-scoped kind has an
//!   account component (the stable account identifier, never a display alias).
//! - A lineage-scoped kind, such as a Pi entry ID, includes the lineage root's ID; a
//!   thread- or response-scoped kind includes its parent's ID.
//! - There is no component role for byte offsets or file-relative ordinals. A fallback key
//!   may use a position only inside a response (an `act-` call's position), and otherwise
//!   needs a digest of recorded content.
//!
//! The one key that uses a record offset, the artifact-local key of an ambiguous
//! observation, is built by [`artifact_local_key`] and has the `ambiguous` basis.

use super::canonical_json::MAX_SAFE_INTEGER;
use super::identity::{AnalyticalId, IdPrefix, IdentityKey, KeyComponent};

/// How strongly an identity is established. The derived order puts the strongest first.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum IdentityBasis {
    /// A native ID with every required namespace component present.
    Native,
    /// A declared fallback key applies.
    Fallback,
    /// No usable key, or a shared key whose observations disagree.
    Ambiguous,
}

impl IdentityBasis {
    /// The contract token: `native`, `fallback` or `ambiguous`.
    pub const fn token(self) -> &'static str {
        match self {
            Self::Native => "native",
            Self::Fallback => "fallback",
            Self::Ambiguous => "ambiguous",
        }
    }
}

/// The uniqueness scope of a native ID kind, which decides the namespace components its
/// key carries.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum IdScope {
    /// Unique within the issuing provider, such as a provider response ID; not scoped to
    /// host, source environment or account, so copies merge wherever they were collected.
    Provider,
    /// Unique within one agent's namespace, such as a native session ID.
    Agent,
    /// Unique only within one account; the key carries the stable account identifier.
    Account,
    /// Unique only within one lineage, such as a Pi entry ID; the key carries the lineage
    /// root's ID.
    Lineage,
    /// Unique within one thread; the key carries the thread's ID.
    Thread,
    /// Unique within one response; the key carries the request's ID.
    Response,
    /// A source artifact key: environment, dialect, locator and first-record digest.
    Source,
}

/// What a key component is, which decides the values it accepts.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ComponentRole {
    /// A registry token naming a provider or agent namespace, such as `anthropic`.
    Namespace,
    /// A stable account identifier from the source manifest.
    Account,
    /// A native ID, entered verbatim.
    NativeId,
    /// A native sequence number or a position within the declared scope.
    Sequence,
    /// The analytical ID of a lineage root thread.
    LineageRoot,
    /// The analytical ID of the containing thread or request.
    Parent,
    /// A digest of recorded content, such as revision-invariant fields.
    Digest,
    /// A registry token naming the source environment, such as `local`.
    Environment,
    /// A dialect registry token, such as `codex-rollout`.
    Dialect,
    /// A source locator: root-relative, or stable when the dialect declares one.
    Locator,
}

/// One declared component of a key kind.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ComponentSlot {
    /// The component's name, for diagnostics.
    pub name: &'static str,
    /// The component's role.
    pub role: ComponentRole,
    /// Whether the key is unusable when the component is unknown. An optional component
    /// that is unknown is digested as `null`.
    pub required: bool,
}

impl ComponentSlot {
    /// A required component.
    pub const fn required(name: &'static str, role: ComponentRole) -> Self {
        Self { name, role, required: true }
    }

    /// An optional component, digested as `null` when unknown.
    pub const fn optional(name: &'static str, role: ComponentRole) -> Self {
        Self { name, role, required: false }
    }
}

/// An adapter's declaration of one key kind.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct KeySpec {
    /// The entity the key identifies.
    pub prefix: IdPrefix,
    /// The key kind token digested into the ID, such as `provider-response`.
    pub kind: &'static str,
    /// Rank among this prefix's key kinds; 0 is the highest precedence.
    pub precedence: u8,
    /// `Native` or `Fallback`; `Ambiguous` is never declared.
    pub basis: IdentityBasis,
    /// The native ID's uniqueness scope.
    pub scope: IdScope,
    /// The components, in digest order.
    pub slots: &'static [ComponentSlot],
}

/// A key built from a spec, carrying the rank its linked set is resolved by.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ScopedKey {
    /// Precedence of the key's kind; 0 is the highest.
    pub precedence: u8,
    /// The basis the key establishes.
    pub basis: IdentityBasis,
    /// The key itself.
    pub key: IdentityKey,
}

/// A key spec or key value that breaks the scope rules.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum KeyScopeError {
    /// The spec itself is invalid.
    #[error("key kind {kind:?} is invalid: {reason}")]
    InvalidSpec {
        /// The key kind.
        kind: &'static str,
        /// Which rule it breaks.
        reason: &'static str,
    },
    /// Two specs share a prefix and kind, or a prefix and precedence.
    #[error("key kinds {first:?} and {second:?} for {prefix} share a kind or a precedence")]
    DuplicateSpec {
        /// The shared prefix.
        prefix: IdPrefix,
        /// The first kind.
        first: &'static str,
        /// The second kind.
        second: &'static str,
    },
    /// The number of values differs from the declared slots.
    #[error("key kind {kind:?} declares {expected} components, got {actual}")]
    ComponentCount {
        /// The key kind.
        kind: &'static str,
        /// Declared slots.
        expected: usize,
        /// Values given.
        actual: usize,
    },
    /// A required component is unknown, so this key does not apply.
    #[error("key kind {kind:?} is missing required component {slot:?}")]
    MissingComponent {
        /// The key kind.
        kind: &'static str,
        /// The missing slot.
        slot: &'static str,
    },
    /// A component value does not fit its role.
    #[error("key kind {kind:?} component {slot:?} is not a valid {role:?} value")]
    InvalidComponent {
        /// The key kind.
        kind: &'static str,
        /// The slot.
        slot: &'static str,
        /// The slot's role.
        role: ComponentRole,
    },
}

impl KeySpec {
    /// Checks the spec against the scope rules in the module documentation.
    pub fn validate(&self) -> Result<(), KeyScopeError> {
        let invalid = |reason| Err(KeyScopeError::InvalidSpec { kind: self.kind, reason });
        if !is_registry_token(self.kind) {
            return invalid("the kind must be a lowercase registry token");
        }
        if self.basis == IdentityBasis::Ambiguous {
            return invalid("only native and fallback key kinds can be declared");
        }
        let has = |role| self.slots.iter().any(|slot| slot.role == role);
        let has_required = |role| self.slots.iter().any(|slot| slot.role == role && slot.required);

        let needs_namespace =
            matches!(self.scope, IdScope::Provider | IdScope::Agent | IdScope::Account);
        if needs_namespace && !has_required(ComponentRole::Namespace) {
            return invalid("provider, agent and account scopes need a required namespace");
        }
        if has(ComponentRole::Account) != (self.scope == IdScope::Account) {
            return invalid("an account component belongs only to an account-scoped kind");
        }
        if self.scope == IdScope::Account && !has_required(ComponentRole::Account) {
            return invalid("an account-scoped kind needs a required account component");
        }
        if has(ComponentRole::LineageRoot) != (self.scope == IdScope::Lineage) {
            return invalid("a lineage root belongs only to, and is needed by, a lineage scope");
        }
        if self.scope == IdScope::Lineage && !has_required(ComponentRole::LineageRoot) {
            return invalid("a lineage-scoped kind needs a required lineage root");
        }
        let parent_scoped = matches!(self.scope, IdScope::Thread | IdScope::Response);
        if parent_scoped && !has_required(ComponentRole::Parent) {
            return invalid("thread and response scopes need a required parent ID");
        }
        let source_roles =
            [ComponentRole::Environment, ComponentRole::Dialect, ComponentRole::Locator];
        if self.scope == IdScope::Source {
            if self.prefix != IdPrefix::Source {
                return invalid("the source scope is only for src- keys");
            }
            if !source_roles.into_iter().chain([ComponentRole::Digest]).all(has_required) {
                return invalid("a source key needs environment, dialect, locator and digest");
            }
        } else if source_roles.into_iter().any(has) {
            return invalid("environment, dialect and locator belong only to source keys");
        }
        match self.basis {
            IdentityBasis::Native => {
                if self.scope != IdScope::Source
                    && !has_required(ComponentRole::NativeId)
                    && !has_required(ComponentRole::Sequence)
                {
                    return invalid("a native key needs a required native ID or sequence");
                }
            }
            IdentityBasis::Fallback => {
                if has(ComponentRole::NativeId) {
                    return invalid("a fallback key cannot carry a native ID");
                }
                if has(ComponentRole::Sequence) && self.scope != IdScope::Response {
                    return invalid(
                        "a fallback key uses a position only within a response, never a file ordinal",
                    );
                }
                if !has_required(ComponentRole::Digest) && !has_required(ComponentRole::Sequence) {
                    return invalid("a fallback key needs a required digest or response position");
                }
            }
            IdentityBasis::Ambiguous => {
                return invalid("only native and fallback key kinds can be declared");
            }
        }
        Ok(())
    }

    /// Builds this kind's key from component values in slot order.
    ///
    /// [`KeyScopeError::MissingComponent`] means the key does not apply to the record and
    /// the adapter should try its next key kind.
    pub fn key(&self, values: Vec<KeyComponent>) -> Result<ScopedKey, KeyScopeError> {
        if values.len() != self.slots.len() {
            return Err(KeyScopeError::ComponentCount {
                kind: self.kind,
                expected: self.slots.len(),
                actual: values.len(),
            });
        }
        for (slot, value) in self.slots.iter().zip(&values) {
            if *value == KeyComponent::Null {
                if slot.required {
                    return Err(KeyScopeError::MissingComponent {
                        kind: self.kind,
                        slot: slot.name,
                    });
                }
                continue;
            }
            if !component_fits(slot.role, self.scope, value) {
                return Err(KeyScopeError::InvalidComponent {
                    kind: self.kind,
                    slot: slot.name,
                    role: slot.role,
                });
            }
        }
        Ok(ScopedKey {
            precedence: self.precedence,
            basis: self.basis,
            key: IdentityKey::new(self.prefix, self.kind, values),
        })
    }
}

/// Checks a set of specs together: each must be valid, and no two for one prefix may
/// share a kind or a precedence.
pub fn validate_specs(specs: &[KeySpec]) -> Result<(), KeyScopeError> {
    for (index, spec) in specs.iter().enumerate() {
        spec.validate()?;
        for other in specs.iter().skip(index.saturating_add(1)) {
            if spec.prefix == other.prefix
                && (spec.kind == other.kind || spec.precedence == other.precedence)
            {
                return Err(KeyScopeError::DuplicateSpec {
                    prefix: spec.prefix,
                    first: spec.kind,
                    second: other.kind,
                });
            }
        }
    }
    Ok(())
}

/// The key kind token of artifact-local keys.
pub const ARTIFACT_LOCAL_KIND: &str = "artifact-local";

/// The artifact-local key of an ambiguous observation: its `src-` ID and record offset, so
/// only a re-read of the same record merges with it (design §3.6).
///
/// Returns `None` for an offset beyond 2^53 − 1 bytes, which no canonical key can carry.
pub fn artifact_local_key(
    prefix: IdPrefix,
    source: &AnalyticalId,
    offset: u64,
) -> Option<ScopedKey> {
    let offset = i64::try_from(offset).ok().filter(|value| *value <= MAX_SAFE_INTEGER)?;
    Some(ScopedKey {
        precedence: u8::MAX,
        basis: IdentityBasis::Ambiguous,
        key: IdentityKey::new(
            prefix,
            ARTIFACT_LOCAL_KIND,
            vec![KeyComponent::text(source.as_str()), KeyComponent::Integer(offset)],
        ),
    })
}

/// The basis a set of usable keys establishes: the strongest among them, or ambiguous
/// when there is none.
pub fn identity_basis(keys: &[ScopedKey]) -> IdentityBasis {
    keys.iter().map(|key| key.basis).min().unwrap_or(IdentityBasis::Ambiguous)
}

/// Whether `text` is a registry token: lowercase ASCII letters, digits and inner hyphens.
pub fn is_registry_token(text: &str) -> bool {
    !text.is_empty()
        && !text.starts_with('-')
        && !text.ends_with('-')
        && text.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

fn component_fits(role: ComponentRole, scope: IdScope, value: &KeyComponent) -> bool {
    match (role, value) {
        (
            ComponentRole::Namespace | ComponentRole::Environment | ComponentRole::Dialect,
            KeyComponent::Text(text),
        ) => is_registry_token(text),
        (
            ComponentRole::Account
            | ComponentRole::NativeId
            | ComponentRole::Digest
            | ComponentRole::Locator,
            KeyComponent::Text(text),
        ) => !text.is_empty(),
        (ComponentRole::Sequence, KeyComponent::Integer(number)) => {
            (0..=MAX_SAFE_INTEGER).contains(number)
        }
        (ComponentRole::LineageRoot, KeyComponent::Text(text)) => {
            AnalyticalId::parse(text).is_ok_and(|id| id.prefix() == IdPrefix::Thread)
        }
        (ComponentRole::Parent, KeyComponent::Text(text)) => {
            let expected =
                if scope == IdScope::Response { IdPrefix::Request } else { IdPrefix::Thread };
            AnalyticalId::parse(text).is_ok_and(|id| id.prefix() == expected)
        }
        (
            ComponentRole::Namespace
            | ComponentRole::Environment
            | ComponentRole::Dialect
            | ComponentRole::Account
            | ComponentRole::NativeId
            | ComponentRole::Digest
            | ComponentRole::Locator
            | ComponentRole::Sequence
            | ComponentRole::LineageRoot
            | ComponentRole::Parent,
            KeyComponent::Null
            | KeyComponent::Integer(_)
            | KeyComponent::Text(_)
            | KeyComponent::Redacted,
        ) => false,
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::{
        ComponentRole, ComponentSlot, IdScope, IdentityBasis, KeyScopeError, KeySpec,
        artifact_local_key, identity_basis, validate_specs,
    };
    use crate::ledger::identity::{AnalyticalId, IdPrefix, KeyComponent};

    const NS: ComponentRole = ComponentRole::Namespace;

    pub(crate) const PROVIDER_RESPONSE: KeySpec = KeySpec {
        prefix: IdPrefix::Request,
        kind: "provider-response",
        precedence: 0,
        basis: IdentityBasis::Native,
        scope: IdScope::Provider,
        slots: &[
            ComponentSlot::required("provider", NS),
            ComponentSlot::required("response_id", ComponentRole::NativeId),
        ],
    };

    pub(crate) const THREAD_DIGEST: KeySpec = KeySpec {
        prefix: IdPrefix::Request,
        kind: "thread-invariant-digest",
        precedence: 3,
        basis: IdentityBasis::Fallback,
        scope: IdScope::Thread,
        slots: &[
            ComponentSlot::required("thread", ComponentRole::Parent),
            ComponentSlot::required("invariant_digest", ComponentRole::Digest),
        ],
    };

    const PI_ENTRY: KeySpec = KeySpec {
        prefix: IdPrefix::Request,
        kind: "pi-entry",
        precedence: 2,
        basis: IdentityBasis::Native,
        scope: IdScope::Lineage,
        slots: &[
            ComponentSlot::required("lineage_root", ComponentRole::LineageRoot),
            ComponentSlot::required("entry_id", ComponentRole::NativeId),
        ],
    };

    fn thread_id() -> AnalyticalId {
        crate::ledger::identity::IdentityKey::new(
            IdPrefix::Thread,
            "agent-session",
            vec![KeyComponent::text("claude"), KeyComponent::text("s1")],
        )
        .derive_id()
        .unwrap()
    }

    fn spec_error(spec: KeySpec) -> &'static str {
        match spec.validate() {
            Err(KeyScopeError::InvalidSpec { reason, .. }) => reason,
            other => unreachable!("expected an invalid spec, got {other:?}"),
        }
    }

    #[test]
    fn well_formed_specs_validate_together() {
        validate_specs(&[PROVIDER_RESPONSE, PI_ENTRY, THREAD_DIGEST]).unwrap();
    }

    #[test]
    fn two_kinds_cannot_share_a_precedence() {
        let clash = KeySpec { kind: "other", ..PROVIDER_RESPONSE };
        assert!(matches!(
            validate_specs(&[PROVIDER_RESPONSE, clash]),
            Err(KeyScopeError::DuplicateSpec { .. })
        ));
    }

    #[test]
    fn account_components_only_in_account_scope() {
        const SLOTS: &[ComponentSlot] = &[
            ComponentSlot::required("provider", NS),
            ComponentSlot::required("account", ComponentRole::Account),
            ComponentSlot::required("id", ComponentRole::NativeId),
        ];
        let spec = KeySpec { slots: SLOTS, ..PROVIDER_RESPONSE };
        assert!(spec_error(spec).contains("account"));
        KeySpec { scope: IdScope::Account, ..spec }.validate().unwrap();
        let unkeyed_account = KeySpec { scope: IdScope::Account, ..PROVIDER_RESPONSE };
        assert!(spec_error(unkeyed_account).contains("account"));
    }

    #[test]
    fn provider_scope_needs_its_namespace() {
        const SLOTS: &[ComponentSlot] =
            &[ComponentSlot::required("response_id", ComponentRole::NativeId)];
        let spec = KeySpec { slots: SLOTS, ..PROVIDER_RESPONSE };
        assert!(spec_error(spec).contains("namespace"));
    }

    #[test]
    fn lineage_scoped_native_ids_need_the_lineage_root() {
        const SLOTS: &[ComponentSlot] =
            &[ComponentSlot::required("entry_id", ComponentRole::NativeId)];
        let bare_entry = KeySpec { slots: SLOTS, ..PI_ENTRY };
        assert!(spec_error(bare_entry).contains("lineage"));
    }

    #[test]
    fn fallback_keys_never_use_file_ordinals() {
        const ORDINAL: &[ComponentSlot] = &[
            ComponentSlot::required("thread", ComponentRole::Parent),
            ComponentSlot::required("line", ComponentRole::Sequence),
        ];
        const POSITION: &[ComponentSlot] = &[
            ComponentSlot::required("request", ComponentRole::Parent),
            ComponentSlot::required("position", ComponentRole::Sequence),
        ];
        let ordinal = KeySpec { slots: ORDINAL, ..THREAD_DIGEST };
        assert!(spec_error(ordinal).contains("file ordinal"));
        let position = KeySpec {
            prefix: IdPrefix::Action,
            kind: "request-position",
            scope: IdScope::Response,
            slots: POSITION,
            ..THREAD_DIGEST
        };
        position.validate().unwrap();
    }

    #[test]
    fn a_missing_required_component_means_the_key_does_not_apply() {
        let error = PROVIDER_RESPONSE
            .key(vec![KeyComponent::text("anthropic"), KeyComponent::Null])
            .unwrap_err();
        assert_eq!(
            error,
            KeyScopeError::MissingComponent { kind: "provider-response", slot: "response_id" }
        );
    }

    #[test]
    fn component_values_must_fit_their_roles() {
        let bad_namespace =
            PROVIDER_RESPONSE.key(vec![KeyComponent::text("Anthropic"), KeyComponent::text("m")]);
        assert!(matches!(bad_namespace, Err(KeyScopeError::InvalidComponent { .. })));
        let wrong_parent = THREAD_DIGEST.key(vec![
            KeyComponent::text("req-v1-00000000000000000000000000"),
            KeyComponent::text("d"),
        ]);
        assert!(matches!(wrong_parent, Err(KeyScopeError::InvalidComponent { .. })));
        let redacted =
            PROVIDER_RESPONSE.key(vec![KeyComponent::text("anthropic"), KeyComponent::Redacted]);
        assert!(matches!(redacted, Err(KeyScopeError::InvalidComponent { .. })));
    }

    #[test]
    fn basis_is_native_then_fallback_then_ambiguous() {
        let native = PROVIDER_RESPONSE
            .key(vec![KeyComponent::text("anthropic"), KeyComponent::text("msg_1")])
            .unwrap();
        let fallback = THREAD_DIGEST
            .key(vec![KeyComponent::text(thread_id().as_str()), KeyComponent::text("abc")])
            .unwrap();
        assert_eq!(identity_basis(&[fallback.clone(), native]), IdentityBasis::Native);
        assert_eq!(identity_basis(&[fallback]), IdentityBasis::Fallback);
        assert_eq!(identity_basis(&[]), IdentityBasis::Ambiguous);
    }

    #[test]
    fn artifact_local_keys_are_ambiguous_and_offset_scoped() {
        let source = crate::ledger::identity::IdentityKey::new(
            IdPrefix::Source,
            "root-relative",
            vec![KeyComponent::text("x")],
        )
        .derive_id()
        .unwrap();
        let first = artifact_local_key(IdPrefix::Request, &source, 10).unwrap();
        let reread = artifact_local_key(IdPrefix::Request, &source, 10).unwrap();
        let other = artifact_local_key(IdPrefix::Request, &source, 11).unwrap();
        assert_eq!(first.basis, IdentityBasis::Ambiguous);
        assert_eq!(first.key.derive_id().unwrap(), reread.key.derive_id().unwrap());
        assert_ne!(first.key.derive_id().unwrap(), other.key.derive_id().unwrap());
        assert!(artifact_local_key(IdPrefix::Request, &source, u64::MAX).is_none());
    }
}
