//! Core types for the contrato contract system.
//!
//! Defines the data structures used to represent contracts, matchers,
//! requirements, and assets. All types implement serde Serialize/Deserialize
//! for JSON round-trip fidelity.

use std::collections::HashMap;
use std::fmt;
use std::sync::OnceLock;

use serde::de;
use serde::ser::SerializeMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{Map, Value};

use crate::children_tree::ChildrenTree;
use crate::hash::hash_object;
use crate::object_set::Identifiable;

/// Type constant for universe contracts (collection of all available contracts).
pub const UNIVERSE: &str = "meta.universe";

/// A contract type string (e.g., `sw.os`, `hw.device-type`).
///
/// Type strings identify the category of a contract. They use dot-separated
/// namespacing (e.g., `hw.device-type`, `arch.sw`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ContractType(String);

impl ContractType {
    /// Creates a new contract type from a string.
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    /// Returns the type as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ContractType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// A contract slug identifier (e.g., `debian`, `raspberry-pi`).
///
/// Slugs uniquely identify a contract within its type.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Slug(String);

impl Slug {
    /// Creates a new slug from a string.
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    /// Returns the slug as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Slug {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Internal representation of a version: either valid semver or a plain
/// identifier (e.g., `wheezy`, `jessie`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum VersionInner {
    /// Full three-component semver parsed directly (e.g. `"1.2.3"`).
    Semver(semver::Version),
    /// Partial semver padded with `.0` components (e.g. `"2.31"` ->
    /// `"2.31.0"`). The original string is kept for serialization.
    PartialSemver {
        parsed: semver::Version,
        original: String,
    },
    Identifier(String),
}

/// A contract version (e.g., `1.0.0`, `2.31`, `wheezy`).
///
/// Construction tries strict semver first (`MAJOR.MINOR.PATCH`). If that
/// fails, it pads the string with `.0` components (so `"2.31"` becomes
/// `"2.31.0"` and `"1"` becomes `"1.0.0"`). This allows partial versions
/// to participate in semver range comparisons while preserving the
/// original string for serialization. Falls back to a plain identifier
/// if padding doesn't produce valid semver either.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Version(VersionInner);

impl Version {
    /// Creates a new version by parsing the string. Tries strict semver
    /// first, then pads with `.0` components, and falls back to a plain
    /// identifier.
    pub fn new(s: impl Into<String>) -> Self {
        let s = s.into();
        if let Ok(v) = semver::Version::parse(&s) {
            return Self(VersionInner::Semver(v));
        }
        // Pad partial versions: "2.31" -> "2.31.0", "1" -> "1.0.0"
        let dot_count = s.chars().filter(|&c| c == '.').count();
        let padded = match dot_count {
            0 => format!("{s}.0.0"),
            1 => format!("{s}.0"),
            _ => return Self(VersionInner::Identifier(s)),
        };
        match semver::Version::parse(&padded) {
            Ok(parsed) => Self(VersionInner::PartialSemver {
                parsed,
                original: s,
            }),
            Err(_) => Self(VersionInner::Identifier(s)),
        }
    }

    /// Returns `true` if this version was parsed as semver (including
    /// partial versions that were padded).
    pub fn is_semver(&self) -> bool {
        matches!(
            self.0,
            VersionInner::Semver(_) | VersionInner::PartialSemver { .. }
        )
    }

    /// Returns the parsed semver value, if any.
    fn as_semver(&self) -> Option<&semver::Version> {
        match &self.0 {
            VersionInner::Semver(v) | VersionInner::PartialSemver { parsed: v, .. } => Some(v),
            VersionInner::Identifier(_) => None,
        }
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0 {
            VersionInner::Semver(v) => write!(f, "{v}"),
            VersionInner::PartialSemver { original, .. } => f.write_str(original),
            VersionInner::Identifier(s) => f.write_str(s),
        }
    }
}

impl Serialize for Version {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for Version {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Ok(Version::new(s))
    }
}

/// Internal representation of a version requirement: either a valid semver
/// range or a plain identifier used for exact equality matching.
#[derive(Debug, Clone, PartialEq, Eq)]
enum VersionReqInner {
    SemverRange(semver::VersionReq),
    Identifier(String),
}

/// A version requirement or range (e.g., `>=1.0.0`, `^2.3`, `wheezy`).
///
/// Deserialization tries semver range first; if that fails, stores as an
/// identifier. Matching semantics depend on the variant: semver ranges
/// use `satisfies()`, identifiers use exact string equality.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionReq(VersionReqInner);

impl VersionReq {
    /// Creates a new version requirement by parsing the string. If it is a
    /// valid semver range, it is stored as such; otherwise as an identifier.
    pub fn new(s: impl Into<String>) -> Self {
        let s = s.into();
        match semver::VersionReq::parse(&s) {
            Ok(r) => Self(VersionReqInner::SemverRange(r)),
            Err(_) => Self(VersionReqInner::Identifier(s)),
        }
    }

    /// Returns `true` if this was parsed as a valid semver range.
    pub fn is_semver_range(&self) -> bool {
        matches!(self.0, VersionReqInner::SemverRange(_))
    }

    /// Returns `true` if `target` satisfies this requirement.
    ///
    /// The allocation-free fast paths are:
    /// - **Semver range × semver version**: delegate to
    ///   [`semver::VersionReq::matches`] on the already-parsed inner
    ///   values — the common case on the validation hot path.
    /// - **Identifier × identifier**: direct string equality on the
    ///   stored inner strings — no allocation, no re-parse.
    ///
    /// The mismatched cases (identifier target against a semver
    /// range, or vice versa) fall back to comparing the two sides'
    /// `Display` output. This allocates, but it is the rare path —
    /// the contract corpus either uses semver throughout or
    /// identifier strings throughout.
    pub fn matches(&self, target: &Version) -> bool {
        // Fast path: both sides are semver (including padded partial versions).
        if let (Some(v), VersionReqInner::SemverRange(r)) = (target.as_semver(), &self.0) {
            return r.matches(v);
        }
        // Fast path: both sides are plain identifiers.
        if let (VersionInner::Identifier(tv), VersionReqInner::Identifier(rv)) =
            (&target.0, &self.0)
        {
            return tv == rv;
        }
        // Mismatched cases: fall back to string comparison.
        target.to_string() == self.to_string()
    }
}

impl fmt::Display for VersionReq {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0 {
            VersionReqInner::SemverRange(r) => write!(f, "{r}"),
            VersionReqInner::Identifier(s) => f.write_str(s),
        }
    }
}

impl Serialize for VersionReq {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for VersionReq {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Ok(VersionReq::new(s))
    }
}

/// An asset attached to a contract, with a URL and optional metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Asset {
    /// The URL where the asset can be retrieved.
    pub url: String,

    /// Optional human-readable name for the asset.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Optional checksum for integrity verification.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checksum: Option<String>,

    /// Optional checksum algorithm (e.g., `"sha256"`).
    #[serde(rename = "checksumType", skip_serializing_if = "Option::is_none")]
    pub checksum_type: Option<String>,
}

/// A matcher that references contracts by type and optional additional criteria.
///
/// Used both as requirement targets (what a contract needs) and as capability
/// declarations (what a contract provides). Per the CUE spec, additional matching
/// criteria should be placed in `data`, not as top-level fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContractMatcher {
    /// The contract type to match against.
    #[serde(rename = "type")]
    pub kind: ContractType,

    /// Optional slug to match.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slug: Option<Slug>,

    /// Optional version or semver range to match.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<VersionReq>,

    /// Optional structured data for deep matching.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,

    /// Lazily computed deterministic hash of this matcher.
    ///
    /// Populated on first call to [`Self::hash`] and shared by the
    /// [`Identifiable`] impl (used by [`ObjectSet`](crate::object_set::ObjectSet)
    /// deduplication in the requirements index) and by the
    /// [`Matcher`](crate::matcher::Matcher) impl (used by the
    /// [`MatcherCache`](crate::matcher_cache::MatcherCache) key on the
    /// search hot path). Both paths share one serialization + SHA-256
    /// per unique matcher — without this cache, `find_children` pays the
    /// full hashing cost twice per cache operation (once on `get`, once
    /// on `insert`).
    ///
    /// Excluded from serde so round-tripping a matcher through JSON
    /// yields an identical canonical form. Cloning a matcher copies
    /// whatever cached value the source had (via `OnceLock::clone`);
    /// two clones of the same matcher may independently populate their
    /// own cells without affecting one another.
    #[serde(skip)]
    hash: OnceLock<String>,
}

impl PartialEq for ContractMatcher {
    /// Compares two matchers by their typed fields, ignoring the
    /// cached hash cell.
    ///
    /// Two matchers with identical `kind`, `slug`, `version`, and
    /// `data` are equal regardless of whether either side has
    /// populated its hash cache — equality is a property of the
    /// canonical matcher, not of the caching state.
    fn eq(&self, other: &Self) -> bool {
        self.kind == other.kind
            && self.slug == other.slug
            && self.version == other.version
            && self.data == other.data
    }
}

impl ContractMatcher {
    /// Creates a new matcher with the given fields and an empty hash
    /// cache.
    ///
    /// Provided as a single construction point so callers do not have
    /// to know that `hash` is an implementation detail. The cache
    /// starts empty; the first call to [`Self::hash`] populates it.
    pub fn new(
        kind: ContractType,
        slug: Option<Slug>,
        version: Option<VersionReq>,
        data: Option<Value>,
    ) -> Self {
        Self {
            kind,
            slug,
            version,
            data,
            hash: OnceLock::new(),
        }
    }

    /// Returns the cached deterministic hash of this matcher,
    /// computing it on first call.
    ///
    /// The hash is a SHA-256 digest of the matcher's canonical JSON
    /// form — the same digest the requirements index uses to
    /// deduplicate matchers inside an
    /// [`ObjectSet`](crate::object_set::ObjectSet) and the same
    /// digest `find_children` uses to key the
    /// [`MatcherCache`](crate::matcher_cache::MatcherCache). Both
    /// code paths route through this method, so a matcher that is
    /// both registered as a requirement and used as a search key
    /// pays exactly one serialization + hashing cost across its
    /// lifetime.
    pub(crate) fn hash(&self) -> &str {
        self.hash.get_or_init(|| {
            hash_object(
                &serde_json::to_value(self).expect("ContractMatcher must serialize to JSON"),
            )
        })
    }
}

/// A contract requirement — either a direct match or a boolean operation
/// over a flat list of simple matchers.
///
/// Requirements express what a contract needs. They can be:
/// - A simple match: `{"type": "hw.device-type", "slug": "rpi"}`
/// - A disjunction: `{"or": [{"type": "hw.device-type", "slug": "rpi"}, ...]}`
/// - A negation: `{"not": [{"type": "sw.os", "slug": "windows"}]}`
///
/// The CUE schema that this type mirrors allows only one level of boolean
/// nesting: the items inside an `or` / `not` are always simple matchers,
/// never further boolean operations. That constraint is enforced here at
/// the type level — `Or` / `Not` carry `Vec<ContractMatcher>`, not
/// `Vec<ContractRequirement>`. Attempting to deserialize a nested
/// `{"or": [{"or": [...]}]}` shape will fail with a serde error because
/// the inner object has no `type` field.
#[derive(Debug, Clone, PartialEq)]
pub enum ContractRequirement {
    /// A direct matcher requirement.
    Match(ContractMatcher),
    /// At least one of the inner matchers must be satisfied.
    Or(Vec<ContractMatcher>),
    /// None of the inner matchers must be satisfied.
    Not(Vec<ContractMatcher>),
}

impl Serialize for ContractRequirement {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            ContractRequirement::Match(matcher) => matcher.serialize(serializer),
            ContractRequirement::Or(items) => {
                let mut map = serializer.serialize_map(Some(1))?;
                map.serialize_entry("or", items)?;
                map.end()
            }
            ContractRequirement::Not(items) => {
                let mut map = serializer.serialize_map(Some(1))?;
                map.serialize_entry("not", items)?;
                map.end()
            }
        }
    }
}

impl<'de> Deserialize<'de> for ContractRequirement {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = Value::deserialize(deserializer)?;
        deserialize_requirement_from_value(value).map_err(de::Error::custom)
    }
}

/// Deserializes a `ContractRequirement` from a `serde_json::Value`.
///
/// Inspects the JSON object for `"or"` or `"not"` keys to determine the
/// variant; the inner items are deserialized as [`ContractMatcher`]s so
/// nested boolean operations fail with a clear serde error. Falls back to
/// `Match` when neither discriminator is present.
fn deserialize_requirement_from_value(value: Value) -> Result<ContractRequirement, String> {
    let obj = value
        .as_object()
        .ok_or_else(|| "requirement must be a JSON object".to_string())?;

    if let Some(or_val) = obj.get("or") {
        let items: Vec<ContractMatcher> =
            serde_json::from_value(or_val.clone()).map_err(|e| format!("'or' items: {e}"))?;
        return Ok(ContractRequirement::Or(items));
    }

    if let Some(not_val) = obj.get("not") {
        let items: Vec<ContractMatcher> =
            serde_json::from_value(not_val.clone()).map_err(|e| format!("'not' items: {e}"))?;
        return Ok(ContractRequirement::Not(items));
    }

    let matcher: ContractMatcher = serde_json::from_value(value).map_err(|e| e.to_string())?;
    Ok(ContractRequirement::Match(matcher))
}

/// Identity for [`ContractMatcher`] used by [`ObjectSet`](crate::object_set::ObjectSet).
///
/// Delegates to the cached [`ContractMatcher::hash`] accessor so that
/// `ObjectSet` deduplication (on `register_matcher`) and the search
/// cache key (on `find_children`) share a single memoized SHA-256 per
/// matcher instance.
impl Identifiable for ContractMatcher {
    fn id(&self) -> String {
        self.hash().to_string()
    }
}

/// Identity for [`ContractRequirement`] used by [`ObjectSet`](crate::object_set::ObjectSet).
///
/// The ID is a deterministic SHA-256 of the requirement's canonical JSON form.
/// `Match`, `Or`, and `Not` variants serialize to distinct JSON shapes so they
/// never collide; two requirements with the same variant and the same inner
/// matchers share an ID and deduplicate inside the compiled requirements set.
impl Identifiable for ContractRequirement {
    fn id(&self) -> String {
        hash_object(
            &serde_json::to_value(self).expect("ContractRequirement must serialize to JSON"),
        )
    }
}

/// Contract metadata fields without a type identifier.
///
/// Used for variant definitions that get deep-merged with a base contract
/// during expansion. The `type` and `slug` come from the base contract.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PartialContract {
    /// Contract slug.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slug: Option<Slug>,

    /// Semver-compliant version string.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<Version>,

    /// Human-readable name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Human-readable description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Alternative slugs for this contract.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub aliases: Vec<Slug>,

    /// Free-form data specific to the contract type.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,

    /// Named assets attached to this contract.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub assets: HashMap<String, Asset>,

    /// Requirements that must be satisfied for this contract.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub requires: Vec<ContractRequirement>,

    /// Capabilities this contract provides to other contracts.
    /// At construction time these are converted into child contracts,
    /// so this field is only used during deserialization.
    #[serde(default, skip_serializing)]
    pub(crate) provides: Vec<ContractCapability>,

    /// Nested variants (recursive expansion).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub variants: Vec<PartialContract>,

    /// Children contracts stored as a nested tree (`{type: {slug: contract}}`).
    ///
    /// Deserialized into a strongly typed [`ChildrenTree`] enum. Conversion
    /// between this tree format and flat contract lists is handled by the
    /// [`children_tree`](crate::children_tree) module.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<ChildrenTree>,
}

/// A capability declaration specifying what a contract provides.
///
/// Combines a required contract type with a [`PartialContract`] for the
/// remaining fields (slug, version, data, etc.). Only used during
/// deserialization — at construction time these are converted into
/// child contracts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct ContractCapability {
    /// The contract type of the provided capability.
    #[serde(rename = "type")]
    pub kind: ContractType,

    /// Shared contract fields (slug, version, data, etc.).
    #[serde(flatten)]
    pub body: PartialContract,
}

/// The raw contract data as deserialized from JSON.
///
/// A full contract has a required `kind` (`type` on JSON), an optional `canonical_slug`, shared
/// fields via [`PartialContract`], and a catch-all `extra` for round-trip
/// fidelity of unknown top-level fields.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct RawContract {
    /// The contract type (e.g., `sw.os`, `hw.device-type`).
    #[serde(rename = "type")]
    pub kind: ContractType,

    /// Maps alias slugs back to the canonical slug.
    #[serde(rename = "canonicalSlug", skip_serializing_if = "Option::is_none")]
    pub canonical_slug: Option<Slug>,

    /// Shared contract fields (slug, version, name, data, requires, etc.).
    #[serde(flatten)]
    pub body: PartialContract,

    /// Additional fields not captured above, preserved for round-trip fidelity.
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn deserialize_minimal_contract() {
        let json = json!({
            "type": "arch.sw",
            "name": "armv7hf",
            "slug": "armv7hf"
        });

        let contract: RawContract = serde_json::from_value(json).unwrap();
        assert_eq!(contract.kind.as_str(), "arch.sw");
        assert_eq!(contract.body.slug.as_ref().unwrap().as_str(), "armv7hf");
        assert_eq!(contract.body.name.as_deref(), Some("armv7hf"));
        assert_eq!(contract.body.version, None);
        assert_eq!(contract.body.description, None);
        assert!(contract.body.aliases.is_empty());
        assert_eq!(contract.canonical_slug, None);
        assert_eq!(contract.body.data, None);
        assert!(contract.body.assets.is_empty());
        assert!(contract.body.requires.is_empty());
        assert!(contract.body.provides.is_empty());
        assert_eq!(contract.body.children, None);
        assert!(contract.body.variants.is_empty());
        assert!(contract.extra.is_empty());
    }

    #[test]
    fn round_trip_minimal_contract() {
        let input = json!({
            "type": "arch.sw",
            "name": "armv7hf",
            "slug": "armv7hf"
        });

        let contract: RawContract = serde_json::from_value(input.clone()).unwrap();
        let output = serde_json::to_value(&contract).unwrap();
        assert_eq!(input, output);
    }

    #[test]
    fn deserialize_contract_with_version() {
        let json = json!({
            "type": "sw.os",
            "name": "Debian Wheezy",
            "version": "wheezy",
            "slug": "debian"
        });

        let contract: RawContract = serde_json::from_value(json).unwrap();
        assert_eq!(contract.kind.as_str(), "sw.os");
        assert_eq!(contract.body.slug.as_ref().unwrap().as_str(), "debian");
        assert_eq!(
            contract.body.version.as_ref().unwrap().to_string(),
            "wheezy"
        );
        assert_eq!(contract.body.name.as_deref(), Some("Debian Wheezy"));
    }

    #[test]
    fn round_trip_contract_with_version() {
        let input = json!({
            "type": "sw.os",
            "name": "Debian Wheezy",
            "version": "wheezy",
            "slug": "debian"
        });

        let contract: RawContract = serde_json::from_value(input.clone()).unwrap();
        let output = serde_json::to_value(&contract).unwrap();
        assert_eq!(input, output);
    }

    #[test]
    fn version_semver_parsing() {
        let v = Version::new("1.2.3");
        assert!(v.is_semver());
        assert_eq!(v.to_string(), "1.2.3");

        let v = Version::new("wheezy");
        assert!(!v.is_semver());
        assert_eq!(v.to_string(), "wheezy");
    }

    #[test]
    fn version_partial_semver_two_components() {
        let v = Version::new("2.31");
        assert!(v.is_semver());
        assert_eq!(v.to_string(), "2.31");
        assert!(VersionReq::new(">=2.17").matches(&v));
        assert!(!VersionReq::new(">=3.0").matches(&v));
    }

    #[test]
    fn version_partial_semver_one_component() {
        let v = Version::new("5");
        assert!(v.is_semver());
        assert_eq!(v.to_string(), "5");
        assert!(VersionReq::new(">=4").matches(&v));
        assert!(!VersionReq::new(">=6").matches(&v));
    }

    #[test]
    fn version_partial_semver_not_a_number() {
        let v = Version::new("abc.def");
        assert!(!v.is_semver());
        assert_eq!(v.to_string(), "abc.def");
    }

    #[test]
    fn version_req_semver_parsing() {
        let vr = VersionReq::new(">=1.0.0");
        assert!(vr.is_semver_range());
        assert_eq!(vr.to_string(), ">=1.0.0");

        let vr = VersionReq::new("wheezy");
        assert!(!vr.is_semver_range());
        assert_eq!(vr.to_string(), "wheezy");
    }

    #[test]
    fn version_req_matches_semver_range_satisfies_semver_version() {
        // Allocation-free fast path: both sides parse as semver, so
        // `matches` dispatches through `semver::VersionReq::matches`
        // on the parsed inner values.
        let target = Version::new("1.2.3");
        assert!(VersionReq::new(">=1.0.0").matches(&target));
        assert!(VersionReq::new("^1.2.0").matches(&target));
        assert!(VersionReq::new("=1.2.3").matches(&target));
    }

    #[test]
    fn version_req_matches_semver_range_rejects_out_of_range() {
        let target = Version::new("0.9.0");
        assert!(!VersionReq::new(">=1.0.0").matches(&target));
    }

    #[test]
    fn version_req_matches_identifier_equality() {
        // Allocation-free fast path: both sides are identifiers, so
        // `matches` compares the stored inner strings directly.
        assert!(VersionReq::new("wheezy").matches(&Version::new("wheezy")));
        assert!(!VersionReq::new("wheezy").matches(&Version::new("jessie")));
    }

    #[test]
    fn version_req_matches_identifier_target_against_semver_range_fallback() {
        // Mismatched case: identifier Version + semver-range
        // VersionReq. The fallback compares `Display` outputs, so
        // "wheezy" is compared against ">=1.0.0" and returns false.
        assert!(!VersionReq::new(">=1.0.0").matches(&Version::new("wheezy")));
    }

    #[test]
    fn version_req_matches_semver_target_against_identifier_req_fallback() {
        // Mismatched case the other way: semver Version + identifier
        // VersionReq. The fallback compares "1.0.0" against the
        // identifier string — they differ.
        assert!(!VersionReq::new("wheezy").matches(&Version::new("1.0.0")));
    }

    #[test]
    fn deserialize_contract_with_simple_requires() {
        let json = json!({
            "type": "sw.stack",
            "slug": "nodejs",
            "requires": [
                {
                    "type": "hw.device-type",
                    "slug": "raspberry-pi"
                }
            ]
        });

        let contract: RawContract = serde_json::from_value(json).unwrap();
        assert_eq!(contract.body.requires.len(), 1);
        match &contract.body.requires[0] {
            ContractRequirement::Match(m) => {
                assert_eq!(m.kind.as_str(), "hw.device-type");
                assert_eq!(m.slug.as_ref().unwrap().as_str(), "raspberry-pi");
            }
            _ => panic!("expected Match variant"),
        }
    }

    #[test]
    fn deserialize_contract_rejects_extra_requirement_fields() {
        let json = json!({
            "type": "sw.stack",
            "slug": "nodejs",
            "requires": [
                {
                    "type": "hw.device-type",
                    "slug": "raspberry-pi",
                    "name": "raspberry-pi",
                }
            ]
        });

        let err = serde_json::from_value::<RawContract>(json).unwrap_err();
        assert_eq!(
            err.to_string(),
            "unknown field `name`, expected one of `type`, `slug`, `version`, `data`",
        );
    }

    #[test]
    fn round_trip_contract_with_simple_requires() {
        let input = json!({
            "type": "sw.stack",
            "slug": "nodejs",
            "requires": [
                {
                    "type": "hw.device-type",
                    "slug": "raspberry-pi"
                }
            ]
        });

        let contract: RawContract = serde_json::from_value(input.clone()).unwrap();
        let output = serde_json::to_value(&contract).unwrap();
        assert_eq!(input, output);
    }

    #[test]
    fn deserialize_requires_with_or_operation() {
        let json = json!({
            "type": "sw.os",
            "slug": "test",
            "requires": [
                {
                    "or": [
                        { "type": "hw.device-type", "slug": "raspberry-pi" },
                        { "type": "hw.device-type", "slug": "raspberry-pi2" }
                    ]
                }
            ]
        });

        let contract: RawContract = serde_json::from_value(json).unwrap();
        assert_eq!(contract.body.requires.len(), 1);
        match &contract.body.requires[0] {
            ContractRequirement::Or(items) => {
                assert_eq!(items.len(), 2);
                assert_eq!(items[0].slug.as_ref().unwrap().as_str(), "raspberry-pi");
                assert_eq!(items[1].slug.as_ref().unwrap().as_str(), "raspberry-pi2");
            }
            _ => panic!("expected Or variant"),
        }
    }

    #[test]
    fn round_trip_requires_with_or() {
        let input = json!({
            "type": "sw.os",
            "slug": "test",
            "requires": [
                {
                    "or": [
                        { "type": "hw.device-type", "slug": "raspberry-pi" },
                        { "type": "hw.device-type", "slug": "raspberry-pi2" }
                    ]
                }
            ]
        });

        let contract: RawContract = serde_json::from_value(input.clone()).unwrap();
        let output = serde_json::to_value(&contract).unwrap();
        assert_eq!(input, output);
    }

    #[test]
    fn deserialize_requires_with_not_operation() {
        let json = json!({
            "type": "sw.os",
            "slug": "test",
            "requires": [
                {
                    "not": [
                        { "type": "sw.os", "slug": "windows" }
                    ]
                }
            ]
        });

        let contract: RawContract = serde_json::from_value(json).unwrap();
        assert_eq!(contract.body.requires.len(), 1);
        match &contract.body.requires[0] {
            ContractRequirement::Not(items) => {
                assert_eq!(items.len(), 1);
                assert_eq!(items[0].slug.as_ref().unwrap().as_str(), "windows");
            }
            _ => panic!("expected Not variant"),
        }
    }

    #[test]
    fn round_trip_requires_with_not() {
        let input = json!({
            "type": "sw.os",
            "slug": "test",
            "requires": [
                {
                    "not": [
                        { "type": "sw.os", "slug": "windows" }
                    ]
                }
            ]
        });

        let contract: RawContract = serde_json::from_value(input.clone()).unwrap();
        let output = serde_json::to_value(&contract).unwrap();
        assert_eq!(input, output);
    }

    #[test]
    fn deserialize_requires_with_empty_or() {
        let json = json!({
            "type": "sw.os",
            "slug": "test",
            "requires": [{ "or": [] }]
        });

        let contract: RawContract = serde_json::from_value(json).unwrap();
        match &contract.body.requires[0] {
            ContractRequirement::Or(items) => assert!(items.is_empty()),
            _ => panic!("expected Or variant"),
        }
    }

    #[test]
    fn deserialize_requires_with_empty_not() {
        let json = json!({
            "type": "sw.os",
            "slug": "test",
            "requires": [{ "not": [] }]
        });

        let contract: RawContract = serde_json::from_value(json).unwrap();
        match &contract.body.requires[0] {
            ContractRequirement::Not(items) => assert!(items.is_empty()),
            _ => panic!("expected Not variant"),
        }
    }

    #[test]
    fn deserialize_matcher_with_nested_data() {
        let json = json!({
            "type": "arch.sw",
            "slug": "aarch64",
            "requires": [
                {
                    "type": "hw.device-type",
                    "data": { "arch": "aarch64" }
                }
            ]
        });

        let contract: RawContract = serde_json::from_value(json).unwrap();
        match &contract.body.requires[0] {
            ContractRequirement::Match(m) => {
                assert_eq!(m.kind.as_str(), "hw.device-type");
                assert_eq!(m.data.as_ref().unwrap(), &json!({"arch": "aarch64"}));
            }
            _ => panic!("expected Match variant"),
        }
    }

    #[test]
    fn round_trip_matcher_with_nested_data() {
        let input = json!({
            "type": "arch.sw",
            "slug": "aarch64",
            "requires": [
                {
                    "type": "hw.device-type",
                    "data": { "arch": "aarch64" }
                }
            ]
        });

        let contract: RawContract = serde_json::from_value(input.clone()).unwrap();
        let output = serde_json::to_value(&contract).unwrap();
        assert_eq!(input, output);
    }

    #[test]
    fn contract_matcher_hash_is_stable_and_structural() {
        // Two structurally-identical matchers built independently
        // must produce the same hash — that is the deduplication
        // invariant the requirements index relies on. Two matchers
        // that differ in any field must produce different hashes.
        let a = ContractMatcher::new(
            ContractType::new("hw.device-type"),
            Some(Slug::new("raspberry-pi")),
            None,
            None,
        );
        let b = ContractMatcher::new(
            ContractType::new("hw.device-type"),
            Some(Slug::new("raspberry-pi")),
            None,
            None,
        );
        let c = ContractMatcher::new(
            ContractType::new("hw.device-type"),
            Some(Slug::new("raspberry-pi2")),
            None,
            None,
        );
        assert_eq!(a.hash(), b.hash());
        assert_ne!(a.hash(), c.hash());
    }

    #[test]
    fn contract_matcher_hash_is_cached_across_calls() {
        // The second call to `hash()` must return the same slice
        // as the first — populating the OnceLock once, not twice.
        // Observing identical pointer addresses is the tightest
        // available proxy for "the inner String was reused".
        let m = ContractMatcher::new(
            ContractType::new("hw.device-type"),
            Some(Slug::new("raspberry-pi")),
            None,
            None,
        );
        let first = m.hash() as *const str;
        let second = m.hash() as *const str;
        assert_eq!(first, second, "hash cell must serve the same slice");
    }

    #[test]
    fn contract_matcher_partial_eq_ignores_cached_hash() {
        // Populating the cache on one matcher but not the other
        // must not affect structural equality.
        let a = ContractMatcher::new(
            ContractType::new("hw.device-type"),
            Some(Slug::new("raspberry-pi")),
            None,
            None,
        );
        let b = ContractMatcher::new(
            ContractType::new("hw.device-type"),
            Some(Slug::new("raspberry-pi")),
            None,
            None,
        );
        let _ = a.hash(); // populate a's cell, leave b's empty
        assert_eq!(a, b);
    }

    #[test]
    fn contract_matcher_roundtrip_hash_field_is_skipped() {
        // The `hash` field must not appear in serialized output so
        // matchers remain round-trip clean through JSON. Verifying
        // via serializing a minimal matcher directly.
        let m = ContractMatcher::new(
            ContractType::new("hw.device-type"),
            Some(Slug::new("raspberry-pi")),
            None,
            None,
        );
        let _ = m.hash(); // populate the cache before serializing
        let output = serde_json::to_value(&m).unwrap();
        assert_eq!(
            output,
            json!({"type": "hw.device-type", "slug": "raspberry-pi"})
        );
    }

    #[test]
    fn deserialize_contract_with_provides() {
        let json = json!({
            "type": "meta.context",
            "slug": "test",
            "provides": [
                {
                    "type": "sw.os",
                    "slug": "debian",
                    "version": "wheezy"
                },
                {
                    "type": "arch.sw",
                    "slug": "amd64",
                    "version": "1"
                }
            ]
        });

        let contract: RawContract = serde_json::from_value(json).unwrap();
        assert_eq!(contract.body.provides.len(), 2);
        assert_eq!(contract.body.provides[0].kind.as_str(), "sw.os");
        assert_eq!(
            contract.body.provides[0]
                .body
                .slug
                .as_ref()
                .unwrap()
                .as_str(),
            "debian"
        );
        assert_eq!(
            contract.body.provides[0]
                .body
                .version
                .as_ref()
                .unwrap()
                .to_string(),
            "wheezy"
        );
        assert_eq!(contract.body.provides[1].kind.as_str(), "arch.sw");
    }

    #[test]
    fn provides_deserialized_but_not_serialized() {
        let input = json!({
            "type": "meta.context",
            "slug": "test",
            "provides": [
                {
                    "type": "sw.os",
                    "slug": "debian",
                    "version": "wheezy"
                }
            ]
        });

        let contract: RawContract = serde_json::from_value(input).unwrap();
        assert_eq!(contract.body.provides.len(), 1);

        let output = serde_json::to_value(&contract).unwrap();
        assert!(output.get("provides").is_none());
    }

    #[test]
    fn deserialize_contract_with_aliases() {
        let json = json!({
            "type": "hw.device-type",
            "name": "Raspberry Pi",
            "slug": "raspberrypi",
            "aliases": ["rpi", "raspberry-pi"]
        });

        let contract: RawContract = serde_json::from_value(json).unwrap();
        assert_eq!(contract.body.aliases.len(), 2);
        assert_eq!(contract.body.aliases[0].as_str(), "rpi");
        assert_eq!(contract.body.aliases[1].as_str(), "raspberry-pi");
    }

    #[test]
    fn round_trip_contract_with_aliases() {
        let input = json!({
            "type": "hw.device-type",
            "name": "Raspberry Pi",
            "slug": "raspberrypi",
            "aliases": ["rpi", "raspberry-pi"]
        });

        let contract: RawContract = serde_json::from_value(input.clone()).unwrap();
        let output = serde_json::to_value(&contract).unwrap();
        assert_eq!(input, output);
    }

    #[test]
    fn deserialize_contract_with_children_tree() {
        let json = json!({
            "type": "meta.context",
            "slug": "test",
            "children": {
                "arch": {
                    "sw": {
                        "type": "arch.sw",
                        "name": "armv7hf",
                        "slug": "armv7hf"
                    }
                }
            }
        });

        let contract: RawContract = serde_json::from_value(json).unwrap();
        assert!(contract.body.children.is_some());
        let children = contract.body.children.as_ref().unwrap();
        match children {
            crate::children_tree::ChildrenTree::Branch(map) => {
                assert!(map.contains_key("arch"));
            }
            _ => panic!("expected Branch"),
        }
    }

    #[test]
    fn round_trip_contract_with_children_tree() {
        let input = json!({
            "type": "meta.context",
            "slug": "test",
            "children": {
                "arch": {
                    "sw": {
                        "type": "arch.sw",
                        "name": "armv7hf",
                        "slug": "armv7hf"
                    }
                }
            }
        });

        let contract: RawContract = serde_json::from_value(input.clone()).unwrap();
        let output = serde_json::to_value(&contract).unwrap();
        assert_eq!(input, output);
    }

    #[test]
    fn deserialize_contract_with_multi_children_tree() {
        let json = json!({
            "type": "meta.context",
            "slug": "test",
            "children": {
                "arch": {
                    "sw": {
                        "armv7hf": {
                            "type": "arch.sw",
                            "name": "armv7hf",
                            "slug": "armv7hf"
                        },
                        "armel": {
                            "type": "arch.sw",
                            "name": "armel",
                            "slug": "armel"
                        }
                    }
                }
            }
        });

        let contract: RawContract = serde_json::from_value(json).unwrap();
        let children = contract.body.children.as_ref().unwrap();
        match children {
            crate::children_tree::ChildrenTree::Branch(root) => match root.get("arch").unwrap() {
                crate::children_tree::ChildrenTree::Branch(arch) => match arch.get("sw").unwrap() {
                    crate::children_tree::ChildrenTree::Branch(sw) => {
                        assert!(sw.contains_key("armv7hf"));
                        assert!(sw.contains_key("armel"));
                    }
                    _ => panic!("expected Branch at sw"),
                },
                _ => panic!("expected Branch at arch"),
            },
            _ => panic!("expected Branch at root"),
        }
    }

    #[test]
    fn deserialize_contract_with_variants() {
        let json = json!({
            "type": "blob",
            "slug": "nodejs_{{data.arch}}",
            "name": "Node.js",
            "data": { "libc": "musl-libc" },
            "variants": [
                {
                    "data": { "arch": "amd64" },
                    "requires": [{ "type": "arch.sw", "slug": "amd64" }]
                },
                {
                    "data": { "arch": "i386" },
                    "requires": [{ "type": "arch.sw", "slug": "i386" }]
                }
            ]
        });

        let contract: RawContract = serde_json::from_value(json).unwrap();
        assert_eq!(contract.body.variants.len(), 2);
        assert_eq!(
            contract.body.variants[0].data,
            Some(json!({"arch": "amd64"}))
        );
        assert_eq!(contract.body.variants[0].requires.len(), 1);
        assert_eq!(contract.body.data, Some(json!({"libc": "musl-libc"})));
    }

    #[test]
    fn round_trip_contract_with_variants() {
        let input = json!({
            "type": "blob",
            "slug": "nodejs_{{data.arch}}",
            "name": "Node.js",
            "data": { "libc": "musl-libc" },
            "variants": [
                {
                    "data": { "arch": "amd64" },
                    "requires": [{ "type": "arch.sw", "slug": "amd64" }]
                }
            ]
        });

        let contract: RawContract = serde_json::from_value(input.clone()).unwrap();
        let output = serde_json::to_value(&contract).unwrap();
        assert_eq!(input, output);
    }

    #[test]
    fn deserialize_nested_variants() {
        let json = json!({
            "type": "blob",
            "slug": "nodejs",
            "variants": [
                {
                    "data": { "arch": "amd64" },
                    "variants": [
                        { "version": "6.3.0" },
                        { "version": "6.4.0" }
                    ]
                }
            ]
        });

        let contract: RawContract = serde_json::from_value(json).unwrap();
        assert_eq!(contract.body.variants.len(), 1);
        assert_eq!(contract.body.variants[0].variants.len(), 2);
        assert_eq!(
            contract.body.variants[0].variants[0]
                .version
                .as_ref()
                .unwrap()
                .to_string(),
            "6.3.0"
        );
    }

    #[test]
    fn deserialize_contract_with_extra_fields() {
        let json = json!({
            "type": "hw.device-type",
            "slug": "am571x-evm",
            "name": "AM571X-EVM",
            "arch": "armv7hf"
        });

        let contract: RawContract = serde_json::from_value(json).unwrap();
        assert_eq!(contract.kind.as_str(), "hw.device-type");
        assert_eq!(contract.extra.get("arch").unwrap(), "armv7hf");
    }

    #[test]
    fn round_trip_contract_with_extra_fields() {
        let input = json!({
            "type": "hw.device-type",
            "slug": "am571x-evm",
            "name": "AM571X-EVM",
            "arch": "armv7hf"
        });

        let contract: RawContract = serde_json::from_value(input.clone()).unwrap();
        let output = serde_json::to_value(&contract).unwrap();
        assert_eq!(input, output);
    }

    #[test]
    fn deserialize_contract_with_assets() {
        let json = json!({
            "type": "sw.blob",
            "slug": "firmware",
            "assets": {
                "binary": {
                    "url": "https://example.com/firmware.bin",
                    "checksum": "abc123",
                    "checksumType": "sha256"
                }
            }
        });

        let contract: RawContract = serde_json::from_value(json).unwrap();
        assert_eq!(contract.body.assets.len(), 1);
        let binary = &contract.body.assets["binary"];
        assert_eq!(binary.url, "https://example.com/firmware.bin");
        assert_eq!(binary.checksum.as_deref(), Some("abc123"));
        assert_eq!(binary.checksum_type.as_deref(), Some("sha256"));
    }

    #[test]
    fn round_trip_contract_with_assets() {
        let input = json!({
            "type": "sw.blob",
            "slug": "firmware",
            "assets": {
                "binary": {
                    "url": "https://example.com/firmware.bin",
                    "checksum": "abc123",
                    "checksumType": "sha256"
                }
            }
        });

        let contract: RawContract = serde_json::from_value(input.clone()).unwrap();
        let output = serde_json::to_value(&contract).unwrap();
        assert_eq!(input, output);
    }

    #[test]
    fn deserialize_contract_with_canonical_slug() {
        let json = json!({
            "type": "hw.device-type",
            "slug": "rpi",
            "canonicalSlug": "raspberrypi"
        });

        let contract: RawContract = serde_json::from_value(json).unwrap();
        assert_eq!(contract.body.slug.as_ref().unwrap().as_str(), "rpi");
        assert_eq!(
            contract.canonical_slug.as_ref().unwrap().as_str(),
            "raspberrypi"
        );
    }

    #[test]
    fn deserialize_contract_with_mixed_requires() {
        let json = json!({
            "type": "sw.stack",
            "slug": "test",
            "requires": [
                { "type": "hw.device-type", "slug": "intel-nuc" },
                { "type": "arch.sw", "slug": "amd64" },
                {
                    "or": [
                        { "type": "sw.os", "slug": "debian" },
                        { "type": "sw.os", "slug": "ubuntu" }
                    ]
                },
                {
                    "not": [
                        { "type": "sw.os", "slug": "windows" }
                    ]
                }
            ]
        });

        let contract: RawContract = serde_json::from_value(json).unwrap();
        assert_eq!(contract.body.requires.len(), 4);
        assert!(matches!(
            &contract.body.requires[0],
            ContractRequirement::Match(_)
        ));
        assert!(matches!(
            &contract.body.requires[1],
            ContractRequirement::Match(_)
        ));
        assert!(matches!(
            &contract.body.requires[2],
            ContractRequirement::Or(_)
        ));
        assert!(matches!(
            &contract.body.requires[3],
            ContractRequirement::Not(_)
        ));
    }

    #[test]
    fn round_trip_contract_with_mixed_requires() {
        let input = json!({
            "type": "sw.stack",
            "slug": "test",
            "requires": [
                { "type": "hw.device-type", "slug": "intel-nuc" },
                {
                    "or": [
                        { "type": "sw.os", "slug": "debian" },
                        { "type": "sw.os", "slug": "ubuntu" }
                    ]
                },
                {
                    "not": [
                        { "type": "sw.os", "slug": "windows" }
                    ]
                }
            ]
        });

        let contract: RawContract = serde_json::from_value(input.clone()).unwrap();
        let output = serde_json::to_value(&contract).unwrap();
        assert_eq!(input, output);
    }

    #[test]
    fn deserialize_real_universe_contract() {
        let json = json!({
            "slug": "aarch64",
            "type": "arch.sw",
            "name": "aarch64",
            "requires": [
                {
                    "type": "hw.device-type",
                    "data": {
                        "arch": "aarch64"
                    }
                }
            ]
        });

        let contract: RawContract = serde_json::from_value(json).unwrap();
        assert_eq!(contract.kind.as_str(), "arch.sw");
        assert_eq!(contract.body.slug.as_ref().unwrap().as_str(), "aarch64");
        match &contract.body.requires[0] {
            ContractRequirement::Match(m) => {
                assert_eq!(m.kind.as_str(), "hw.device-type");
                assert_eq!(m.data.as_ref().unwrap(), &json!({"arch": "aarch64"}));
            }
            _ => panic!("expected Match variant"),
        }
    }

    #[test]
    fn deserialize_only_type_required() {
        let json = json!({"type": "test"});
        let contract: RawContract = serde_json::from_value(json).unwrap();
        assert_eq!(contract.kind.as_str(), "test");
        assert_eq!(contract.body.slug, None);
    }

    #[test]
    fn deserialize_fails_without_type() {
        let json = json!({"slug": "test"});
        let result: Result<RawContract, _> = serde_json::from_value(json);
        assert!(result.is_err());
    }

    #[test]
    fn type_constants_are_correct() {
        assert_eq!(UNIVERSE, "meta.universe");
    }

    #[test]
    fn contract_type_display() {
        let ct = ContractType::new("sw.os");
        assert_eq!(format!("{ct}"), "sw.os");
    }

    #[test]
    fn slug_display() {
        let s = Slug::new("debian");
        assert_eq!(format!("{s}"), "debian");
    }

    #[test]
    fn version_display() {
        let v = Version::new("1.2.3");
        assert_eq!(format!("{v}"), "1.2.3");
    }

    #[test]
    fn version_req_display() {
        let vr = VersionReq::new(">=1.0.0");
        assert_eq!(format!("{vr}"), ">=1.0.0");
    }

    #[test]
    fn round_trip_full_contract() {
        let input = json!({
            "type": "sw.os",
            "slug": "debian",
            "version": "wheezy",
            "name": "Debian Wheezy",
            "description": "A Linux distribution",
            "aliases": ["deb-wheezy"],
            "canonicalSlug": "debian",
            "data": {
                "libc": "glibc",
                "release": 7
            },
            "assets": {
                "image": {
                    "url": "https://example.com/debian.img",
                    "name": "Debian Image"
                }
            },
            "requires": [
                { "type": "arch.sw", "slug": "amd64" },
                {
                    "or": [
                        { "type": "hw.device-type", "slug": "nuc" },
                        { "type": "hw.device-type", "slug": "pc" }
                    ]
                }
            ],
            "provides": [
                { "type": "sw.runtime", "slug": "glibc", "version": "^2.19" }
            ],
            "children": {
                "sw": {
                    "package": {
                        "type": "sw.package",
                        "slug": "apt",
                        "version": "1.0"
                    }
                }
            },
            "variants": [
                { "version": "jessie" }
            ],
            "custom_field": "custom_value",
            "nested_custom": { "a": 1, "b": [2, 3] }
        });

        let contract: RawContract = serde_json::from_value(input.clone()).unwrap();
        // Verify provides was deserialized
        assert_eq!(contract.body.provides.len(), 1);

        let output = serde_json::to_value(&contract).unwrap();
        // provides is not serialized — it becomes children at construction time
        let mut expected = input;
        expected.as_object_mut().unwrap().remove("provides");
        assert_eq!(expected, output);
    }
}
