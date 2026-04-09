//! Core types for the contrato contract system.
//!
//! Defines the data structures used to represent contracts, matchers,
//! requirements, and assets. All types implement serde Serialize/Deserialize
//! for JSON round-trip fidelity.

use std::collections::HashMap;
use std::fmt;

use serde::de;
use serde::ser::SerializeMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{Map, Value};

/// Type constant for universe contracts (collection of all available contracts).
pub const UNIVERSE: &str = "meta.universe";

/// A contract type string (e.g., `sw.os`, `hw.device-type`).
///
/// Type strings identify the category of a contract. They use dot-separated
/// namespacing (e.g., `hw.device-type`, `arch.sw`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
    Semver(semver::Version),
    Identifier(String),
}

/// A contract version (e.g., `1.0.0`, `wheezy`).
///
/// Deserialization tries semver first; if that fails, stores as an identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Version(VersionInner);

impl Version {
    /// Creates a new version by parsing the string. If it is valid semver, it
    /// is stored as such; otherwise it is stored as a plain identifier.
    pub fn new(s: impl Into<String>) -> Self {
        let s = s.into();
        match semver::Version::parse(&s) {
            Ok(v) => Self(VersionInner::Semver(v)),
            Err(_) => Self(VersionInner::Identifier(s)),
        }
    }

    /// Returns `true` if this version was parsed as valid semver.
    pub fn is_semver(&self) -> bool {
        matches!(self.0, VersionInner::Semver(_))
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0 {
            VersionInner::Semver(v) => write!(f, "{v}"),
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
}

/// A contract requirement — either a direct match or a boolean operation.
///
/// Requirements express what a contract needs. They can be:
/// - A simple match: `{"type": "hw.device-type", "slug": "rpi"}`
/// - A disjunction: `{"or": [{"type": "hw.device-type", "slug": "rpi"}, ...]}`
/// - A negation: `{"not": [{"type": "sw.os", "slug": "windows"}]}`
#[derive(Debug, Clone, PartialEq)]
pub enum ContractRequirement {
    /// A direct matcher requirement.
    Match(ContractMatcher),
    /// At least one of the inner requirements must be satisfied.
    Or(Vec<ContractRequirement>),
    /// None of the inner requirements must be satisfied.
    Not(Vec<ContractRequirement>),
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
/// variant. Falls back to `Match` if neither is present.
fn deserialize_requirement_from_value(value: Value) -> Result<ContractRequirement, String> {
    let obj = value
        .as_object()
        .ok_or_else(|| "requirement must be a JSON object".to_string())?;

    if let Some(or_val) = obj.get("or") {
        let arr = or_val
            .as_array()
            .ok_or_else(|| "'or' must be an array".to_string())?;
        let items: Result<Vec<ContractRequirement>, String> = arr
            .iter()
            .map(|v| deserialize_requirement_from_value(v.clone()))
            .collect();
        return Ok(ContractRequirement::Or(items?));
    }

    if let Some(not_val) = obj.get("not") {
        let arr = not_val
            .as_array()
            .ok_or_else(|| "'not' must be an array".to_string())?;
        let items: Result<Vec<ContractRequirement>, String> = arr
            .iter()
            .map(|v| deserialize_requirement_from_value(v.clone()))
            .collect();
        return Ok(ContractRequirement::Not(items?));
    }

    let matcher: ContractMatcher = serde_json::from_value(value).map_err(|e| e.to_string())?;
    Ok(ContractRequirement::Match(matcher))
}

/// Contract metadata fields without a type identifier.
///
/// Used for variant definitions that get deep-merged with a base contract
/// during expansion. The `type` and `slug` come from the base contract.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub provides: Vec<ContractCapability>,

    /// Nested variants (recursive expansion).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub variants: Vec<PartialContract>,

    /// Children contracts.
    // TODO: The CUE spec defines children as `[...#Contract]` (a flat array),
    // but the TS implementation uses a nested tree format `{type: {slug: contract}}`.
    // Keeping as `Value` until children_tree.rs (Phase 7) handles the conversion.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<Value>,
}

/// A capability declaration specifying what a contract provides.
///
/// Combines a required contract type with a [`PartialContract`] for the
/// remaining fields (slug, version, data, etc.).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContractCapability {
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
    fn version_req_semver_parsing() {
        let vr = VersionReq::new(">=1.0.0");
        assert!(vr.is_semver_range());
        assert_eq!(vr.to_string(), ">=1.0.0");

        let vr = VersionReq::new("wheezy");
        assert!(!vr.is_semver_range());
        assert_eq!(vr.to_string(), "wheezy");
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
                match &items[0] {
                    ContractRequirement::Match(m) => {
                        assert_eq!(m.slug.as_ref().unwrap().as_str(), "raspberry-pi");
                    }
                    _ => panic!("expected Match inside Or"),
                }
                match &items[1] {
                    ContractRequirement::Match(m) => {
                        assert_eq!(m.slug.as_ref().unwrap().as_str(), "raspberry-pi2");
                    }
                    _ => panic!("expected Match inside Or"),
                }
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
                match &items[0] {
                    ContractRequirement::Match(m) => {
                        assert_eq!(m.slug.as_ref().unwrap().as_str(), "windows");
                    }
                    _ => panic!("expected Match inside Not"),
                }
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
    fn round_trip_contract_with_provides() {
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

        let contract: RawContract = serde_json::from_value(input.clone()).unwrap();
        let output = serde_json::to_value(&contract).unwrap();
        assert_eq!(input, output);
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
        assert!(children.is_object());
        assert!(children.get("arch").is_some());
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
        let sw = children.get("arch").unwrap().get("sw").unwrap();
        assert!(sw.get("armv7hf").is_some());
        assert!(sw.get("armel").is_some());
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
        let output = serde_json::to_value(&contract).unwrap();
        assert_eq!(input, output);
    }
}
