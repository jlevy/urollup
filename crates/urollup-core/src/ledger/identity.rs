//! Analytical IDs: deterministic identifiers derived from recorded keys (design §3.6).
//!
//! An ID has the form `<prefix>-v<version>-<digest>`, such as
//! `req-v1-74jwmxh9ngbvdmk5zgvcrtgszc`. The digest is the first 128 bits of SHA-256 over
//! the RFC 8785 canonical JSON of one flat array:
//!
//! ```text
//! [prefix token, identity version, key kind, component 1, component 2, ...]
//! ```
//!
//! for example `["req",1,"provider-response","anthropic","msg_01"]`. The 128 bits are read
//! as a big-endian unsigned integer and written as 26 lowercase Crockford base32 digits
//! (alphabet `0123456789abcdefghjkmnpqrstvwxyz`), left-padded like a ULID, so the first
//! digit is at most `7` and string order equals digest order.
//!
//! Keys are stored beside their IDs ([`StoredIdentity`]) so an ID can be re-derived and
//! checked, and an [`IdentityRegistry`] refuses two different keys that produce one ID
//! ([`IdentityError::Collision`]). A key component replaced by redaction
//! ([`KeyComponent::Redacted`]) keeps its stored ID but can never be re-derived.

use std::collections::BTreeMap;
use std::fmt;
use std::str::FromStr;

use sha2::{Digest as _, Sha256};

use super::canonical_json::{CanonicalJsonError, CanonicalValue, to_canonical_json};

/// Number of base32 digits in an ID digest: 128 bits in 5-bit digits, rounded up.
pub const DIGEST_DIGITS: usize = 26;

const CROCKFORD_LOWER: &[u8; 32] = b"0123456789abcdefghjkmnpqrstvwxyz";

/// The entity kind an analytical ID names, written as its prefix.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum IdPrefix {
    /// `src-`: a source artifact.
    Source,
    /// `thr-`: a thread.
    Thread,
    /// `req-`: a logical request and its response.
    Request,
    /// `act-`: a tool action.
    Action,
    /// `rpt-`: a report.
    Report,
}

impl IdPrefix {
    /// Every prefix, in declaration order.
    pub const ALL: [Self; 5] =
        [Self::Source, Self::Thread, Self::Request, Self::Action, Self::Report];

    /// The token written before the first hyphen, which is also the first element of the
    /// digested key array.
    pub const fn token(self) -> &'static str {
        match self {
            Self::Source => "src",
            Self::Thread => "thr",
            Self::Request => "req",
            Self::Action => "act",
            Self::Report => "rpt",
        }
    }

    fn from_token(token: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|prefix| prefix.token() == token)
    }
}

impl fmt::Display for IdPrefix {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.token())
    }
}

/// An identity contract version: the rules that turn a key into an ID.
///
/// Only version 1 exists. Re-derivation under another version will add variants here.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum IdentityVersion {
    /// Version 1, the derivation this module documents.
    V1,
}

impl IdentityVersion {
    /// The version number, written as `v<number>` in IDs and as an integer in key arrays.
    pub const fn number(self) -> u32 {
        match self {
            Self::V1 => 1,
        }
    }

    fn from_number(number: u32) -> Option<Self> {
        (number == 1).then_some(Self::V1)
    }
}

/// A validated analytical ID.
///
/// The ID stores its prefix, version and 128-bit digest rather than its text, so IDs and
/// the evidence references that carry them never allocate. Ordering equals the order of
/// the ID text, which the design's "lowest ID" tie-breaks use.
///
/// The digest is kept as big-endian bytes rather than a `u128`, whose 16-byte alignment
/// would pad every ID, and every row that embeds one, to twice its data size.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct AnalyticalId {
    prefix: IdPrefix,
    version: IdentityVersion,
    digest: [u8; 16],
}

const _: () = assert!(std::mem::size_of::<AnalyticalId>() == 17);
const _: () = assert!(std::mem::size_of::<Option<AnalyticalId>>() == 17);

impl Ord for AnalyticalId {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Text order: the prefix token, then `v<number>`, then 26 digits, whose text order
        // equals digest order. Version numbers are single digits, so numeric order is text
        // order.
        self.prefix
            .token()
            .cmp(other.prefix.token())
            .then_with(|| self.version.number().cmp(&other.version.number()))
            .then_with(|| self.digest.cmp(&other.digest))
    }
}

impl PartialOrd for AnalyticalId {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl AnalyticalId {
    /// Parses and validates an ID string.
    pub fn parse(text: &str) -> Result<Self, IdParseError> {
        let invalid = || IdParseError { text: text.to_owned() };
        let mut parts = text.splitn(3, '-');
        let (Some(prefix), Some(version), Some(digest)) =
            (parts.next(), parts.next(), parts.next())
        else {
            return Err(invalid());
        };
        let prefix = IdPrefix::from_token(prefix).ok_or_else(invalid)?;
        let number = version.strip_prefix('v').ok_or_else(invalid)?;
        if number.starts_with('0') || !number.bytes().all(|b| b.is_ascii_digit()) {
            return Err(invalid());
        }
        let version = number
            .parse::<u32>()
            .ok()
            .and_then(IdentityVersion::from_number)
            .ok_or_else(invalid)?;
        let digits = digest.as_bytes();
        let well_formed = digits.len() == DIGEST_DIGITS
            && digits.first().is_some_and(|first| (b'0'..=b'7').contains(first));
        if !well_formed {
            return Err(invalid());
        }
        let mut value = 0_u128;
        for digit in digits {
            let index = CROCKFORD_LOWER.iter().position(|b| b == digit).ok_or_else(invalid)?;
            value = (value << 5) | u128::try_from(index).map_err(|_| invalid())?;
        }
        Ok(Self { prefix, version, digest: value.to_be_bytes() })
    }

    /// The entity kind this ID names.
    pub const fn prefix(&self) -> IdPrefix {
        self.prefix
    }
}

impl fmt::Display for AnalyticalId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let digits = crockford_digits(u128::from_be_bytes(self.digest));
        let digits = std::str::from_utf8(&digits).map_err(|_| fmt::Error)?;
        write!(f, "{}-v{}-{digits}", self.prefix.token(), self.version.number())
    }
}

impl FromStr for AnalyticalId {
    type Err = IdParseError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::parse(text)
    }
}

/// A string that is not a well-formed analytical ID.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[error("not a valid analytical ID: {text:?}")]
pub struct IdParseError {
    /// The rejected text.
    pub text: String,
}

/// One component of an identity key.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum KeyComponent {
    /// An unknown component, digested as JSON `null`.
    Null,
    /// An integer component, such as a native sequence number.
    Integer(i64),
    /// A text component: a native ID verbatim, a registry token or a digest.
    Text(String),
    /// A component removed by redaction. Its key keeps the stored ID, which cannot be
    /// re-derived.
    Redacted,
}

impl KeyComponent {
    /// A text component.
    pub fn text(value: impl Into<String>) -> Self {
        Self::Text(value.into())
    }
}

/// The recorded key an analytical ID derives from.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct IdentityKey {
    /// The entity kind.
    pub prefix: IdPrefix,
    /// The identity contract version the key was built under.
    pub version: IdentityVersion,
    /// The key kind, a registry token such as `provider-response`.
    pub kind: String,
    /// The components in the order the key kind declares.
    pub components: Vec<KeyComponent>,
}

impl IdentityKey {
    /// A version 1 key.
    pub fn new(prefix: IdPrefix, kind: impl Into<String>, components: Vec<KeyComponent>) -> Self {
        Self { prefix, version: IdentityVersion::V1, kind: kind.into(), components }
    }

    /// The RFC 8785 canonical JSON array this key digests under its own version.
    pub fn canonical_json(&self) -> Result<String, IdentityError> {
        self.canonical_json_for(self.version)
    }

    fn canonical_json_for(&self, version: IdentityVersion) -> Result<String, IdentityError> {
        let mut array = Vec::with_capacity(self.components.len().saturating_add(3));
        array.push(CanonicalValue::String(self.prefix.token().to_owned()));
        array.push(CanonicalValue::Integer(i64::from(version.number())));
        array.push(CanonicalValue::String(self.kind.clone()));
        for component in &self.components {
            array.push(match component {
                KeyComponent::Null => CanonicalValue::Null,
                KeyComponent::Integer(value) => CanonicalValue::Integer(*value),
                KeyComponent::Text(value) => CanonicalValue::String(value.clone()),
                KeyComponent::Redacted => {
                    return Err(IdentityError::Redacted { key: Box::new(self.clone()) });
                }
            });
        }
        to_canonical_json(&CanonicalValue::Array(array)).map_err(IdentityError::NotCanonical)
    }

    /// Derives this key's ID under its own version.
    pub fn derive_id(&self) -> Result<AnalyticalId, IdentityError> {
        self.rederive(self.version)
    }

    /// Derives this key's ID with 64 further digest bits that tell apart two different keys
    /// sharing that ID, so a caller can detect collisions without storing the key.
    pub fn derive_digest(&self) -> Result<(AnalyticalId, u64), IdentityError> {
        let canonical = self.canonical_json_for(self.version)?;
        let full = Sha256::digest(canonical.as_bytes());
        let mut id = [0u8; 16];
        id.copy_from_slice(&full[..16]);
        let mut check = [0u8; 8];
        check.copy_from_slice(&full[16..24]);
        Ok((id_from_digest(self.prefix, self.version, &id), u64::from_be_bytes(check)))
    }

    /// Derives this key's ID under `version`, as a reader supporting several identity
    /// versions does when merging inputs written under different ones.
    pub fn rederive(&self, version: IdentityVersion) -> Result<AnalyticalId, IdentityError> {
        let canonical = self.canonical_json_for(version)?;
        Ok(id_from_digest(self.prefix, version, &sha256_128(canonical.as_bytes())))
    }

    /// Whether any component was removed by redaction.
    pub fn is_redacted(&self) -> bool {
        self.components.contains(&KeyComponent::Redacted)
    }
}

/// An ID stored beside the key it was derived from.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct StoredIdentity {
    /// The stored ID.
    pub id: AnalyticalId,
    /// The stored key, possibly redacted.
    pub key: IdentityKey,
}

impl StoredIdentity {
    /// Derives `key`'s ID and stores it with the key.
    pub fn derive(key: IdentityKey) -> Result<Self, IdentityError> {
        Ok(Self { id: key.derive_id()?, key })
    }

    /// Checks that the stored key re-derives the stored ID.
    ///
    /// A redacted key cannot be checked and returns [`IdentityError::Redacted`].
    pub fn verify(&self) -> Result<(), IdentityError> {
        let derived = self.key.derive_id()?;
        if derived == self.id {
            Ok(())
        } else {
            Err(IdentityError::Mismatch { stored: self.id.clone(), derived })
        }
    }
}

/// Why an identity could not be derived, re-derived or registered.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum IdentityError {
    /// Two different keys produce one ID.
    #[error(
        "identity collision: {id} derives from two different keys ({existing:?} and {incoming:?})"
    )]
    Collision {
        /// The shared ID.
        id: AnalyticalId,
        /// The key registered first.
        existing: Box<IdentityKey>,
        /// The different key that produced the same ID.
        incoming: Box<IdentityKey>,
    },
    /// A stored key does not re-derive its stored ID.
    #[error("stored identity {stored} does not match its key, which derives {derived}")]
    Mismatch {
        /// The stored ID.
        stored: AnalyticalId,
        /// The ID the stored key derives.
        derived: AnalyticalId,
    },
    /// A key with a redacted component cannot be derived.
    #[error("key {key:?} has a redacted component, so its ID cannot be re-derived")]
    Redacted {
        /// The redacted key.
        key: Box<IdentityKey>,
    },
    /// A key component has no canonical JSON form.
    #[error("key has no canonical form: {0}")]
    NotCanonical(CanonicalJsonError),
    /// Two different keys derive one ID, detected by their further digest bits.
    #[error("identity collision: two different keys derive {id}")]
    DigestCollision {
        /// The shared ID.
        id: AnalyticalId,
    },
}

/// IDs seen in one run, with the key each came from, so a second key producing a known ID
/// is caught.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct IdentityRegistry {
    keys: BTreeMap<AnalyticalId, IdentityKey>,
}

impl IdentityRegistry {
    /// An empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Derives `key`'s ID and registers the pair.
    pub fn derive(&mut self, key: &IdentityKey) -> Result<AnalyticalId, IdentityError> {
        let id = key.derive_id()?;
        self.register(id.clone(), key)?;
        Ok(id)
    }

    /// Registers an ID read from an artifact with its stored key.
    ///
    /// A derivable key must re-derive the stored ID. A redacted key is accepted as stored,
    /// since it cannot be checked, but still collides with a different registered key.
    pub fn register_stored(&mut self, stored: &StoredIdentity) -> Result<(), IdentityError> {
        if !stored.key.is_redacted() {
            stored.verify()?;
        }
        self.register(stored.id.clone(), &stored.key)
    }

    /// The key registered for `id`.
    pub fn key(&self, id: &AnalyticalId) -> Option<&IdentityKey> {
        self.keys.get(id)
    }

    /// Number of registered IDs.
    pub fn len(&self) -> usize {
        self.keys.len()
    }

    /// Whether no ID is registered.
    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }

    fn register(&mut self, id: AnalyticalId, key: &IdentityKey) -> Result<(), IdentityError> {
        match self.keys.get(&id) {
            Some(existing) if existing == key => Ok(()),
            Some(existing) => Err(IdentityError::Collision {
                id,
                existing: Box::new(existing.clone()),
                incoming: Box::new(key.clone()),
            }),
            None => {
                self.keys.insert(id, key.clone());
                Ok(())
            }
        }
    }
}

/// The first 128 bits of SHA-256 over `bytes`.
pub(crate) fn sha256_128(bytes: &[u8]) -> [u8; 16] {
    let full = Sha256::digest(bytes);
    let mut truncated = [0u8; 16];
    truncated.copy_from_slice(&full[..16]);
    truncated
}

/// 128 bits as 26 lowercase Crockford base32 digits, most significant first.
fn crockford_digits(mut value: u128) -> [u8; DIGEST_DIGITS] {
    let mut digits = [b'0'; DIGEST_DIGITS];
    for slot in digits.iter_mut().rev() {
        // The mask keeps the index below 32.
        *slot = CROCKFORD_LOWER[usize::try_from(value & 0x1f).unwrap_or(0)];
        value >>= 5;
    }
    digits
}

/// Writes 128 bits as 26 lowercase Crockford base32 digits, most significant first.
pub(crate) fn crockford_base32_128(bits: &[u8; 16]) -> String {
    crockford_digits(u128::from_be_bytes(*bits)).iter().map(|&digit| char::from(digit)).collect()
}

fn id_from_digest(prefix: IdPrefix, version: IdentityVersion, digest: &[u8; 16]) -> AnalyticalId {
    AnalyticalId { prefix, version, digest: *digest }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::{
        AnalyticalId, IdPrefix, IdentityError, IdentityKey, IdentityRegistry, IdentityVersion,
        KeyComponent, StoredIdentity, crockford_base32_128, id_from_digest,
    };

    pub(crate) fn response_key(provider: &str, response: &str) -> IdentityKey {
        IdentityKey::new(
            IdPrefix::Request,
            "provider-response",
            vec![KeyComponent::text(provider), KeyComponent::text(response)],
        )
    }

    #[test]
    fn the_key_array_is_prefix_version_kind_then_components() {
        let key = IdentityKey::new(
            IdPrefix::Request,
            "provider-response",
            vec![KeyComponent::text("anthropic"), KeyComponent::Null, KeyComponent::Integer(3)],
        );
        assert_eq!(
            key.canonical_json().unwrap(),
            r#"["req",1,"provider-response","anthropic",null,3]"#
        );
    }

    #[test]
    fn derives_a_stable_well_formed_id() {
        let id = response_key("anthropic", "msg_01").derive_id().unwrap();
        // Pinned, and checked independently with Python hashlib: a change here changes
        // every stored request ID under identity v1.
        assert_eq!(id.to_string(), "req-v1-74jwmxh9ngbvdmk5zgvcrtgszc");
        assert_eq!(AnalyticalId::parse(&id.to_string()).unwrap(), id);
        assert_eq!(id.prefix(), IdPrefix::Request);
    }

    proptest::proptest! {
        #[test]
        fn id_order_and_text_round_trip_match_the_text_form(
            left_digest in proptest::prelude::any::<[u8; 16]>(),
            right_digest in proptest::prelude::any::<[u8; 16]>(),
            left_prefix in 0_usize..IdPrefix::ALL.len(),
            right_prefix in 0_usize..IdPrefix::ALL.len(),
        ) {
            let left = id_from_digest(IdPrefix::ALL[left_prefix], IdentityVersion::V1, &left_digest);
            let right =
                id_from_digest(IdPrefix::ALL[right_prefix], IdentityVersion::V1, &right_digest);
            proptest::prop_assert_eq!(left.cmp(&right), left.to_string().cmp(&right.to_string()));
            proptest::prop_assert_eq!(AnalyticalId::parse(&left.to_string()).unwrap(), left);
        }
    }

    #[test]
    fn base32_is_big_endian_and_left_padded() {
        assert_eq!(crockford_base32_128(&[0; 16]), "00000000000000000000000000");
        assert_eq!(crockford_base32_128(&[0xff; 16]), "7zzzzzzzzzzzzzzzzzzzzzzzzz");
        let mut one = [0u8; 16];
        one[15] = 1;
        assert_eq!(crockford_base32_128(&one), "00000000000000000000000001");
        let mut high = [0u8; 16];
        high[0] = 0x80;
        assert_eq!(crockford_base32_128(&high), "40000000000000000000000000");
    }

    #[test]
    fn different_keys_derive_different_ids() {
        let a = response_key("anthropic", "msg_01").derive_id().unwrap();
        let b = response_key("anthropic", "msg_02").derive_id().unwrap();
        let c = response_key("openai", "msg_01").derive_id().unwrap();
        assert_ne!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn component_boundaries_are_part_of_the_digest() {
        let split = IdentityKey::new(
            IdPrefix::Request,
            "k",
            vec![KeyComponent::text("ab"), KeyComponent::text("c")],
        );
        let joined = IdentityKey::new(
            IdPrefix::Request,
            "k",
            vec![KeyComponent::text("a"), KeyComponent::text("bc")],
        );
        assert_ne!(split.derive_id().unwrap(), joined.derive_id().unwrap());
    }

    #[test]
    fn a_stored_key_rederives_its_id() {
        let stored = StoredIdentity::derive(response_key("anthropic", "msg_01")).unwrap();
        stored.verify().unwrap();
        assert_eq!(stored.key.rederive(IdentityVersion::V1).unwrap(), stored.id);
    }

    #[test]
    fn a_stored_id_that_its_key_does_not_derive_is_a_mismatch() {
        let stored = StoredIdentity {
            id: response_key("anthropic", "msg_01").derive_id().unwrap(),
            key: response_key("anthropic", "msg_02"),
        };
        assert!(matches!(stored.verify(), Err(IdentityError::Mismatch { .. })));
        assert!(matches!(
            IdentityRegistry::new().register_stored(&stored),
            Err(IdentityError::Mismatch { .. })
        ));
    }

    #[test]
    fn a_redacted_key_cannot_be_rederived() {
        let key = IdentityKey::new(
            IdPrefix::Source,
            "root-relative",
            vec![KeyComponent::text("local"), KeyComponent::Redacted],
        );
        assert!(key.is_redacted());
        assert!(matches!(key.derive_id(), Err(IdentityError::Redacted { .. })));
    }

    #[test]
    fn two_keys_with_one_id_are_a_collision() {
        let mut registry = IdentityRegistry::new();
        let id = registry.derive(&response_key("anthropic", "msg_01")).unwrap();
        // Inject the collision: a redacted stored key claiming the same ID cannot be
        // re-derived, so only the registry can notice that its key differs.
        let impostor = StoredIdentity {
            id: id.clone(),
            key: IdentityKey::new(
                IdPrefix::Request,
                "provider-response",
                vec![KeyComponent::Redacted],
            ),
        };
        let error = registry.register_stored(&impostor).unwrap_err();
        assert!(matches!(error, IdentityError::Collision { id: ref shared, .. } if *shared == id));
    }

    #[test]
    fn a_forced_digest_collision_between_derived_keys_is_caught() {
        let mut registry = IdentityRegistry::new();
        let shared = id_from_digest(IdPrefix::Request, IdentityVersion::V1, &[7; 16]);
        registry.register(shared.clone(), &response_key("anthropic", "msg_01")).unwrap();
        registry.register(shared.clone(), &response_key("anthropic", "msg_01")).unwrap();
        let error = registry.register(shared, &response_key("anthropic", "msg_02")).unwrap_err();
        assert!(matches!(error, IdentityError::Collision { .. }));
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn rejects_malformed_id_strings() {
        let good = response_key("anthropic", "msg_01").derive_id().unwrap();
        let good_text = good.to_string();
        let digest = &good_text[7..];
        for bad in [
            String::new(),
            "req-v1".to_owned(),
            format!("xyz-v1-{digest}"),
            format!("req-v2-{digest}"),
            format!("req-v01-{digest}"),
            format!("req-1-{digest}"),
            format!("req-v1-{}", &digest[1..]),
            format!("req-v1-{}u", &digest[1..]),
            format!("req-v1-8{}", &digest[1..]),
            format!("req-v1-{}", digest.to_uppercase()),
        ] {
            assert!(AnalyticalId::parse(&bad).is_err(), "accepted {bad:?}");
        }
    }
}
