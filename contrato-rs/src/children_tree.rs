//! Serialize and deserialize the nested children tree format.
//!
//! Contracts store their children in a nested tree structure keyed by type and
//! slug. This module converts between that tree format and flat collections of
//! contract data.
//!
//! # Tree Format
//!
//! The tree nests contracts by their dotted type path. Types like `sw.os` become
//! nested objects `{ "sw": { "os": ... } }`.
//!
//! - **Single child of a type**: stored directly at the type path.
//!   `{ "sw": { "os": { "type": "sw.os", "slug": "debian", ... } } }`
//! - **Multiple children of a type**: nested one level deeper by slug.
//!   `{ "sw": { "os": { "debian": { ... }, "fedora": { ... } } } }`
//! - **Multiple children with the same slug**: stored as an array.
//!   `{ "sw": { "os": { "debian": [{ ... }, { ... }] } } }`

use std::collections::BTreeMap;
use std::fmt;

use serde::de;
use serde::ser::SerializeMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{Map, Value};

use crate::index::ContractIndex;
use crate::types::RawContract;

/// A strongly typed representation of the nested children tree.
///
/// The tree structure mirrors the JSON format used in contract serialization:
/// intermediate nodes map path segments to subtrees, while leaf nodes hold
/// one or more [`RawContract`] values.
#[derive(Debug, Clone, PartialEq)]
pub enum ChildrenTree {
    /// An intermediate node mapping keys (type path segments or slugs) to subtrees.
    Branch(BTreeMap<String, ChildrenTree>),
    /// A leaf containing a single contract.
    Single(Box<RawContract>),
    /// A leaf containing multiple contracts at the same tree position
    /// (e.g., different versions or variants of the same slug).
    Multiple(Vec<RawContract>),
}

impl Serialize for ChildrenTree {
    /// Serializes directly to the target format without an intermediate `Value`.
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            ChildrenTree::Branch(map) => {
                let mut ser_map = serializer.serialize_map(Some(map.len()))?;
                for (key, value) in map {
                    ser_map.serialize_entry(key, value)?;
                }
                ser_map.end()
            }
            ChildrenTree::Single(contract) => contract.serialize(serializer),
            ChildrenTree::Multiple(contracts) => contracts.serialize(serializer),
        }
    }
}

/// Visitor that deserializes a [`ChildrenTree`] from JSON.
///
/// - Arrays are deserialized directly as `Vec<RawContract>` (no `Value` buffering).
/// - Objects must be buffered into a `Map` to inspect the `slug` field before
///   deciding whether the object is a contract leaf or a branch node.
struct ChildrenTreeVisitor;

impl<'de> de::Visitor<'de> for ChildrenTreeVisitor {
    type Value = ChildrenTree;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a children tree (JSON object or array)")
    }

    fn visit_seq<A: de::SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let mut contracts = Vec::new();
        while let Some(contract) = seq.next_element::<RawContract>()? {
            contracts.push(contract);
        }
        Ok(ChildrenTree::Multiple(contracts))
    }

    fn visit_map<A: de::MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        // Buffer entries so we can identify whether the object is a contract or a sub-tree
        let mut entries = Map::new();
        while let Some((key, value)) = map.next_entry::<String, Value>()? {
            entries.insert(key, value);
        }

        // if entries contains a `type` key, assume it's a contract, otherwise treat it as
        // a sub-tree
        if let Some(kind) = entries.get("type")
            && matches!(kind, Value::String(s) if !s.is_empty())
        {
            let contract: RawContract =
                serde_json::from_value(Value::Object(entries)).map_err(de::Error::custom)?;
            Ok(ChildrenTree::Single(Box::new(contract)))
        } else {
            let mut branch = BTreeMap::new();
            for (key, val) in entries {
                let child: ChildrenTree = serde_json::from_value(val).map_err(de::Error::custom)?;
                branch.insert(key, child);
            }
            Ok(ChildrenTree::Branch(branch))
        }
    }
}

impl<'de> Deserialize<'de> for ChildrenTree {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(ChildrenTreeVisitor)
    }
}

/// Extracts all contracts from a [`ChildrenTree`], consuming it.
///
/// Recursively walks the tree and moves every [`RawContract`] found at leaf
/// positions into a flat vector.
pub(crate) fn into_all(tree: ChildrenTree) -> Vec<RawContract> {
    let mut out = Vec::new();
    collect_into(tree, &mut out);
    out
}

/// Recursive helper that accumulates contracts into a single `Vec`, avoiding
/// intermediate allocations from `flat_map` + `collect` at each branch level.
fn collect_into(tree: ChildrenTree, out: &mut Vec<RawContract>) {
    match tree {
        ChildrenTree::Branch(map) => {
            for child in map.into_values() {
                collect_into(child, out);
            }
        }
        ChildrenTree::Single(contract) => out.push(*contract),
        ChildrenTree::Multiple(contracts) => out.extend(contracts),
    }
}

/// Builds a [`ChildrenTree`] from a contract index.
///
/// Reconstructs the nested tree format used in contract JSON serialization.
/// Types are split on `.` to create nested path segments (e.g., `sw.os` becomes
/// `{ "sw": { "os": ... } }`).
///
/// # Returns
///
/// A `ChildrenTree` representing the nested tree structure.
pub(crate) fn build(index: &ContractIndex) -> ChildrenTree {
    let mut root = BTreeMap::new();

    for kind in index.types() {
        // A type contributing no node must not leave empty branches
        // behind, so the node is built before the path is walked.
        let Some(node) = type_node(index, kind) else {
            continue;
        };

        // tree conditions (no mising slug, no overlapping types), are enforced by the index, but we
        // add debug assertions here in case they don't
        let mut level = &mut root;
        let mut segments = kind.split('.').peekable();
        while let Some(segment) = segments.next() {
            if segments.peek().is_none() {
                // if this is the last segment of the type, insert the node at this position in the tree
                if let Some(shadowed) = level.insert(segment.to_string(), node) {
                    // if there was another node in the same position, there is an overlap missed
                    // by the index
                    debug_assert!(false, "child type '{kind}' replaced {shadowed:?}");
                }
                break;
            }

            // for every intermediate segment create a new branch if none exists
            let entry = level
                .entry(segment.to_string())
                .or_insert_with(|| ChildrenTree::Branch(BTreeMap::new()));
            let ChildrenTree::Branch(subtree) = entry else {
                // if there is a leaf in the current entry, there is also a path overlap, another
                // leaf node was already inserted
                debug_assert!(false, "child type '{kind}' nests under a leaf");
                break;
            };

            // continue going down the new subtree
            level = subtree;
        }
    }

    ChildrenTree::Branch(root)
}

/// Builds the node holding every child of `kind`, or `None` when the index
/// holds none.
///
/// A lone child sits at the type's own key; siblings nest one level deeper,
/// keyed by slug and by every alias.
fn type_node(index: &ContractIndex, kind: &str) -> Option<ChildrenTree> {
    // find all contracts for the given type
    let mut hashes = index.hashes_by_type(kind);
    let first = hashes.next()?;

    if hashes.next().is_none() {
        // if there is only one contract, then no nesting is needed
        let contract = index.get(first)?;
        return Some(ChildrenTree::Single(Box::new(contract.raw().clone())));
    }

    let mut by_slug = BTreeMap::new();
    for slug in index.slugs_by_type(kind) {
        // find all contracts for each slug indexed per type
        let mut contracts: Vec<RawContract> = index
            .hashes_by_type_slug(kind, slug)
            .filter_map(|hash| index.get(hash))
            .map(|contract| contract.raw().clone())
            .collect();

        let node = if contracts.len() == 1 {
            // if there is only one contract, put it immediately under the branch type
            ChildrenTree::Single(Box::new(contracts.pop()?))
        } else {
            // otherwise put it under an array
            ChildrenTree::Multiple(contracts)
        };

        // insert the new node under the slug
        by_slug.insert(slug.to_string(), node);
    }

    Some(ChildrenTree::Branch(by_slug))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract::Contract;
    use serde_json::json;
    use std::collections::HashSet;

    /// Helper to create a minimal [`RawContract`].
    fn raw_contract(type_: &str, slug: &str, version: Option<&str>) -> RawContract {
        let mut val = json!({ "type": type_, "slug": slug });
        if let Some(v) = version {
            val["version"] = json!(v);
        }
        serde_json::from_value(val).unwrap()
    }

    /// Builds a [`ContractIndex`] holding the given contracts.
    fn index_of(contracts: &[RawContract]) -> ContractIndex {
        let contracts = contracts
            .iter()
            .cloned()
            .map(Contract::new)
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

        let mut index = ContractIndex::default();
        index.insert_all(contracts).unwrap();
        index
    }

    // -----------------------------------------------------------------------
    // ChildrenTree serde tests
    // -----------------------------------------------------------------------

    #[test]
    fn serde_round_trip_single() {
        let input = json!({
            "arch": {
                "sw": {
                    "type": "arch.sw",
                    "slug": "armv7hf"
                }
            }
        });
        let tree: ChildrenTree = serde_json::from_value(input.clone()).unwrap();

        // Verify the variant structure, not just the round-trip.
        match &tree {
            ChildrenTree::Branch(root) => match root.get("arch").unwrap() {
                ChildrenTree::Branch(arch) => {
                    assert!(matches!(arch.get("sw").unwrap(), ChildrenTree::Single(_)));
                }
                _ => panic!("expected Branch at arch"),
            },
            _ => panic!("expected Branch at root"),
        }

        let output = serde_json::to_value(&tree).unwrap();
        assert_eq!(input, output);
    }

    #[test]
    fn serde_round_trip_multiple_slugs() {
        let input = json!({
            "sw": {
                "os": {
                    "debian": { "type": "sw.os", "slug": "debian", "version": "wheezy" },
                    "fedora": { "type": "sw.os", "slug": "fedora", "version": "25" }
                }
            }
        });
        let tree: ChildrenTree = serde_json::from_value(input.clone()).unwrap();
        let output = serde_json::to_value(&tree).unwrap();
        assert_eq!(input, output);
    }

    #[test]
    fn serde_round_trip_array() {
        let input = json!({
            "sw": {
                "os": {
                    "debian": [
                        { "type": "sw.os", "slug": "debian", "version": "wheezy" },
                        { "type": "sw.os", "slug": "debian", "version": "jessie" }
                    ]
                }
            }
        });
        let tree: ChildrenTree = serde_json::from_value(input.clone()).unwrap();
        let output = serde_json::to_value(&tree).unwrap();
        assert_eq!(input, output);
    }

    #[test]
    fn deserialize_rejects_bare_number() {
        let result = serde_json::from_value::<ChildrenTree>(json!(42));
        assert!(result.is_err());
    }

    #[test]
    fn deserialize_rejects_bare_string() {
        let result = serde_json::from_value::<ChildrenTree>(json!("not a tree"));
        assert!(result.is_err());
    }

    #[test]
    fn deserialize_rejects_null() {
        let result = serde_json::from_value::<ChildrenTree>(Value::Null);
        assert!(result.is_err());
    }

    #[test]
    fn deserialize_rejects_bare_bool() {
        let result = serde_json::from_value::<ChildrenTree>(json!(true));
        assert!(result.is_err());
    }

    #[test]
    fn deserialize_empty_array() {
        let tree: ChildrenTree = serde_json::from_value(json!([])).unwrap();
        assert_eq!(tree, ChildrenTree::Multiple(vec![]));
        assert!(into_all(tree).is_empty());
    }

    // -----------------------------------------------------------------------
    // into_all tests
    // -----------------------------------------------------------------------

    #[test]
    fn into_all_empty_tree() {
        let tree = ChildrenTree::Branch(BTreeMap::new());
        assert!(into_all(tree).is_empty());
    }

    #[test]
    fn into_all_root_is_single() {
        let c = raw_contract("sw.os", "debian", Some("wheezy"));
        let tree = ChildrenTree::Single(Box::new(c.clone()));
        let result = into_all(tree);
        assert_eq!(result, vec![c]);
    }

    #[test]
    fn into_all_root_is_multiple() {
        let c1 = raw_contract("sw.os", "debian", Some("wheezy"));
        let c2 = raw_contract("sw.os", "debian", Some("jessie"));
        let tree = ChildrenTree::Multiple(vec![c1.clone(), c2.clone()]);
        let result = into_all(tree);
        assert_eq!(result, vec![c1, c2]);
    }

    #[test]
    fn into_all_single_contract() {
        let tree: ChildrenTree = serde_json::from_value(json!({
            "sw": {
                "os": {
                    "type": "sw.os",
                    "slug": "debian",
                    "version": "wheezy"
                }
            }
        }))
        .unwrap();

        let result = into_all(tree);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].body.slug.as_ref().unwrap().as_str(), "debian");
    }

    #[test]
    fn into_all_multiple_types() {
        let tree: ChildrenTree = serde_json::from_value(json!({
            "sw": {
                "os": { "type": "sw.os", "slug": "debian", "version": "wheezy" },
                "blob": { "type": "sw.blob", "slug": "nodejs", "version": "4.8.0" }
            }
        }))
        .unwrap();

        let result = into_all(tree);
        assert_eq!(result.len(), 2);
        let slugs: HashSet<&str> = result
            .iter()
            .map(|c| c.body.slug.as_ref().unwrap().as_str())
            .collect();
        assert!(slugs.contains("debian"));
        assert!(slugs.contains("nodejs"));
    }

    #[test]
    fn into_all_nested_by_slug() {
        let tree: ChildrenTree = serde_json::from_value(json!({
            "sw": {
                "os": {
                    "debian": { "type": "sw.os", "slug": "debian", "version": "wheezy" },
                    "fedora": { "type": "sw.os", "slug": "fedora", "version": "25" }
                }
            }
        }))
        .unwrap();

        let result = into_all(tree);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn into_all_array_of_same_slug() {
        let tree: ChildrenTree = serde_json::from_value(json!({
            "sw": {
                "os": {
                    "debian": [
                        { "type": "sw.os", "slug": "debian", "version": "wheezy" },
                        { "type": "sw.os", "slug": "debian", "version": "jessie" }
                    ]
                }
            }
        }))
        .unwrap();

        let result = into_all(tree);
        assert_eq!(result.len(), 2);
        let versions: HashSet<String> = result
            .iter()
            .map(|c| c.body.version.as_ref().unwrap().to_string())
            .collect();
        assert!(versions.contains("wheezy"));
        assert!(versions.contains("jessie"));
    }

    // -----------------------------------------------------------------------
    // build tests
    // -----------------------------------------------------------------------

    #[test]
    fn build_empty_index() {
        let result = build(&ContractIndex::default());
        assert_eq!(result, ChildrenTree::Branch(BTreeMap::new()));
    }

    #[test]
    fn build_single_child() {
        let c1 = raw_contract("sw.os", "debian", Some("wheezy"));
        let result = build(&index_of(std::slice::from_ref(&c1)));

        // Verify full tree shape: sw -> os -> Single(contract)
        let json = serde_json::to_value(&result).unwrap();
        assert_eq!(json.pointer("/sw/os/slug").unwrap(), "debian");

        let extracted = into_all(result);
        assert_eq!(extracted.len(), 1);
        assert_eq!(extracted[0], c1);
    }

    #[test]
    fn build_two_different_types() {
        let c1 = raw_contract("sw.os", "debian", Some("wheezy"));
        let c2 = raw_contract("sw.blob", "nodejs", Some("4.8.0"));
        let result = build(&index_of(&[c1.clone(), c2.clone()]));

        // Both types share the "sw" prefix, so they should be siblings.
        let json = serde_json::to_value(&result).unwrap();
        assert_eq!(json.pointer("/sw/os/slug").unwrap(), "debian");
        assert_eq!(json.pointer("/sw/blob/slug").unwrap(), "nodejs");

        let extracted = into_all(result);
        assert_eq!(extracted.len(), 2);
        assert!(extracted.contains(&c1));
        assert!(extracted.contains(&c2));
    }

    /// A slug is one key however many dots it contains, so `node.js` must
    /// not nest as `node` -> `js`.
    #[test]
    fn build_keeps_a_dotted_slug_as_a_single_key() {
        let c1 = raw_contract("sw.os", "node.js", None);
        let c2 = raw_contract("sw.os", "debian.", None);
        let tree = build(&index_of(&[c1.clone(), c2.clone()]));

        let json = serde_json::to_value(&tree).unwrap();
        assert_eq!(json.pointer("/sw/os/node.js/slug").unwrap(), "node.js");
        assert_eq!(json.pointer("/sw/os/debian./slug").unwrap(), "debian.");

        let extracted = into_all(tree);
        assert_eq!(extracted.len(), 2);
        assert!(extracted.contains(&c1));
        assert!(extracted.contains(&c2));
    }

    #[test]
    fn build_same_type_different_slugs() {
        let c1 = raw_contract("sw.os", "debian", Some("wheezy"));
        let c2 = raw_contract("sw.os", "fedora", Some("25"));
        let result = build(&index_of(&[c1.clone(), c2.clone()]));

        // Verify tree shape: sw -> os -> {debian, fedora}
        let json = serde_json::to_value(&result).unwrap();
        assert!(json.pointer("/sw/os/debian").is_some());
        assert!(json.pointer("/sw/os/fedora").is_some());

        let extracted = into_all(result);
        assert_eq!(extracted.len(), 2);
        assert!(extracted.contains(&c1));
        assert!(extracted.contains(&c2));
    }

    #[test]
    fn build_multiple_versions_same_slug() {
        let c1 = raw_contract("sw.os", "debian", Some("wheezy"));
        let c2 = raw_contract("sw.os", "debian", Some("jessie"));
        let result = build(&index_of(&[c1.clone(), c2.clone()]));

        // Verify the debian node is Multiple
        let json = serde_json::to_value(&result).unwrap();
        let arr = json.pointer("/sw/os/debian").expect("should have debian");
        assert!(arr.is_array());
        assert_eq!(arr.as_array().unwrap().len(), 2);

        let extracted = into_all(result);
        assert_eq!(extracted.len(), 2);
        assert!(extracted.contains(&c1));
        assert!(extracted.contains(&c2));
    }

    #[test]
    fn build_variants_same_slug_and_version() {
        let c1: RawContract = serde_json::from_value(json!({
            "type": "sw.os",
            "slug": "debian",
            "version": "wheezy",
            "requires": [{ "type": "arch.sw", "slug": "amd64" }]
        }))
        .unwrap();
        let c2: RawContract = serde_json::from_value(json!({
            "type": "sw.os",
            "slug": "debian",
            "version": "wheezy",
            "requires": [{ "type": "arch.sw", "slug": "armv7hf" }]
        }))
        .unwrap();

        let result = build(&index_of(&[c1, c2]));
        let json = serde_json::to_value(&result).unwrap();
        let arr = json.pointer("/sw/os/debian").expect("should have debian");
        assert!(arr.is_array());
        assert_eq!(arr.as_array().unwrap().len(), 2);
    }

    /// Aliases are keyed alongside the canonical slug, so a child reachable
    /// under two names appears at both keys.
    #[test]
    fn build_keys_a_child_under_each_alias() {
        let aliased: RawContract = serde_json::from_value(json!({
            "type": "sw.os",
            "slug": "debian",
            "aliases": ["deb"]
        }))
        .unwrap();
        let other = raw_contract("sw.os", "fedora", None);

        let contracts = Contract::build(aliased).unwrap();
        let mut index = ContractIndex::default();
        index.insert_all(contracts).unwrap();
        index
            .insert_all(vec![Contract::new(other).unwrap()])
            .unwrap();

        let json = serde_json::to_value(build(&index)).unwrap();
        assert_eq!(json.pointer("/sw/os/deb/slug").unwrap(), "deb");
        assert_eq!(json.pointer("/sw/os/debian/slug").unwrap(), "debian");
        assert_eq!(json.pointer("/sw/os/fedora/slug").unwrap(), "fedora");
    }

    // -----------------------------------------------------------------------
    // Round-trip: build -> into_all
    // -----------------------------------------------------------------------

    #[test]
    fn round_trip_build_then_into_all() {
        let c1 = raw_contract("sw.os", "debian", Some("wheezy"));
        let c2 = raw_contract("sw.blob", "nodejs", Some("4.8.0"));

        let extracted = into_all(build(&index_of(&[c1.clone(), c2.clone()])));
        assert_eq!(extracted.len(), 2);
        assert!(extracted.contains(&c1));
        assert!(extracted.contains(&c2));
    }

    #[test]
    fn round_trip_multi_version_slug() {
        let c1 = raw_contract("sw.os", "debian", Some("wheezy"));
        let c2 = raw_contract("sw.os", "debian", Some("jessie"));

        let extracted = into_all(build(&index_of(&[c1.clone(), c2.clone()])));
        assert_eq!(extracted.len(), 2);
        assert!(extracted.contains(&c1));
        assert!(extracted.contains(&c2));
    }
}
