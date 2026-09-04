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

/// Trait exposing the children index data needed by [`build`].
///
/// Decouples the tree-building logic from the concrete index implementations
pub(crate) trait ChildrenIndex {
    /// Return an iterator over the type strings of all contracts in the index.
    /// Each type must be yielded exactly once; no specific ordering is
    /// required, since the tree is keyed by [`BTreeMap`] at every level.
    fn child_types(&self) -> impl Iterator<Item = &str>;

    /// Returns an iterator over the unique contract hashes for the
    /// given type. The iterator must yield unique values and its
    /// `len()` must be O(1). Returns `None` if the type has no
    /// children.
    fn type_hashes(&self, ty: &str) -> Option<impl ExactSizeIterator<Item = &str>>;

    /// Iterates over `(slug, hash_iterator)` pairs for the given type.
    ///
    /// Both the slug references and hash references borrow from `&self`.
    fn type_slugs<'a>(
        &'a self,
        ty: &str,
    ) -> impl Iterator<Item = (&'a str, impl Iterator<Item = &'a str> + 'a)> + 'a;

    /// Looks up a child's [`RawContract`] by its hash.
    fn child_by_hash(&self, hash: &str) -> Option<&RawContract>;
}

/// Builds a [`ChildrenTree`] from children index data.
///
/// Reconstructs the nested tree format used in contract JSON serialization.
/// Types are split on `.` to create nested path segments (e.g., `sw.os` becomes
/// `{ "sw": { "os": ... } }`).
///
/// # Arguments
///
/// * `source` - Any type implementing [`ChildrenIndex`]
///
/// # Returns
///
/// A `ChildrenTree` representing the nested tree structure.
pub(crate) fn build(source: &impl ChildrenIndex) -> ChildrenTree {
    let mut root = BTreeMap::new();

    for kind in source.child_types() {
        // A type contributing no node must not leave empty branches
        // behind, so the node is built before the path is walked.
        let Some(node) = type_node(source, kind) else {
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
fn type_node(source: &impl ChildrenIndex, kind: &str) -> Option<ChildrenTree> {
    let mut hashes = source.type_hashes(kind)?;

    if hashes.len() == 1 {
        let contract = source.child_by_hash(hashes.next()?)?;
        return Some(ChildrenTree::Single(Box::new(contract.clone())));
    }

    let mut by_slug = BTreeMap::new();
    for (slug, hashes) in source.type_slugs(kind) {
        let contracts: Vec<RawContract> = hashes
            .filter_map(|hash| source.child_by_hash(hash).cloned())
            .collect();

        let node = match contracts.len() {
            0 => continue,
            1 => ChildrenTree::Single(Box::new(contracts.into_iter().next()?)),
            _ => ChildrenTree::Multiple(contracts),
        };
        by_slug.insert(slug.to_string(), node);
    }

    (!by_slug.is_empty()).then_some(ChildrenTree::Branch(by_slug))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::collections::{HashMap, HashSet};

    // -----------------------------------------------------------------------
    // Test implementation of WithChildrenIndex
    // -----------------------------------------------------------------------

    /// A simple test implementation of [`WithChildrenIndex`] backed by HashMaps.
    struct TestIndex {
        types: HashSet<String>,
        by_type: HashMap<String, HashSet<String>>,
        by_type_slug: HashMap<String, HashMap<String, HashSet<String>>>,
        contracts: HashMap<String, RawContract>,
    }

    impl ChildrenIndex for TestIndex {
        fn child_types(&self) -> impl Iterator<Item = &str> {
            self.types.iter().map(String::as_str)
        }

        fn type_hashes(&self, ty: &str) -> Option<impl ExactSizeIterator<Item = &str>> {
            self.by_type.get(ty).map(|s| s.iter().map(String::as_str))
        }

        fn type_slugs<'a>(
            &'a self,
            ty: &str,
        ) -> impl Iterator<Item = (&'a str, impl Iterator<Item = &'a str> + 'a)> + 'a {
            self.by_type_slug.get(ty).into_iter().flat_map(|slug_map| {
                slug_map
                    .iter()
                    .map(|(slug, hashes)| (slug.as_str(), hashes.iter().map(String::as_str)))
            })
        }

        fn child_by_hash(&self, hash: &str) -> Option<&RawContract> {
            self.contracts.get(hash)
        }
    }

    /// Helper to create a minimal [`RawContract`].
    fn raw_contract(type_: &str, slug: &str, version: Option<&str>) -> RawContract {
        let mut val = json!({ "type": type_, "slug": slug });
        if let Some(v) = version {
            val["version"] = json!(v);
        }
        serde_json::from_value(val).unwrap()
    }

    /// Helper to build an empty [`TestIndex`].
    fn empty_index() -> TestIndex {
        TestIndex {
            types: HashSet::new(),
            by_type: HashMap::new(),
            by_type_slug: HashMap::new(),
            contracts: HashMap::new(),
        }
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
        let index = empty_index();
        let result = build(&index);
        assert_eq!(result, ChildrenTree::Branch(BTreeMap::new()));
    }

    #[test]
    fn build_type_with_no_hashes() {
        // child_types returns a type, but type_hashes returns None for it.
        let index = TestIndex {
            types: HashSet::from(["sw.os".to_string()]),
            by_type: HashMap::new(), // no entry for sw.os
            by_type_slug: HashMap::new(),
            contracts: HashMap::new(),
        };
        let result = build(&index);
        assert_eq!(result, ChildrenTree::Branch(BTreeMap::new()));
    }

    #[test]
    fn build_single_child() {
        let c1 = raw_contract("sw.os", "debian", Some("wheezy"));
        let h1 = "hash1".to_string();

        let index = TestIndex {
            types: HashSet::from(["sw.os".to_string()]),
            by_type: HashMap::from([("sw.os".to_string(), HashSet::from([h1.clone()]))]),
            by_type_slug: HashMap::from([(
                "sw.os".to_string(),
                HashMap::from([("debian".to_string(), HashSet::from([h1.clone()]))]),
            )]),
            contracts: HashMap::from([(h1, c1.clone())]),
        };

        let result = build(&index);

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
        let h1 = "hash1".to_string();
        let h2 = "hash2".to_string();

        let index = TestIndex {
            types: HashSet::from(["sw.os".to_string(), "sw.blob".to_string()]),
            by_type: HashMap::from([
                ("sw.os".to_string(), HashSet::from([h1.clone()])),
                ("sw.blob".to_string(), HashSet::from([h2.clone()])),
            ]),
            by_type_slug: HashMap::from([
                (
                    "sw.os".to_string(),
                    HashMap::from([("debian".to_string(), HashSet::from([h1.clone()]))]),
                ),
                (
                    "sw.blob".to_string(),
                    HashMap::from([("nodejs".to_string(), HashSet::from([h2.clone()]))]),
                ),
            ]),
            contracts: HashMap::from([(h1, c1.clone()), (h2, c2.clone())]),
        };

        let result = build(&index);

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
        let h1 = "hash1".to_string();
        let h2 = "hash2".to_string();

        let index = TestIndex {
            types: HashSet::from(["sw.os".to_string()]),
            by_type: HashMap::from([(
                "sw.os".to_string(),
                HashSet::from([h1.clone(), h2.clone()]),
            )]),
            by_type_slug: HashMap::from([(
                "sw.os".to_string(),
                HashMap::from([
                    ("node.js".to_string(), HashSet::from([h1.clone()])),
                    ("debian.".to_string(), HashSet::from([h2.clone()])),
                ]),
            )]),
            contracts: HashMap::from([(h1, c1.clone()), (h2, c2.clone())]),
        };

        let tree = build(&index);
        let json = serde_json::to_value(&tree).unwrap();
        assert_eq!(json.pointer("/sw/os/node.js/slug").unwrap(), "node.js");
        assert_eq!(json.pointer("/sw/os/debian./slug").unwrap(), "debian.");

        let extracted = into_all(tree);
        assert_eq!(extracted.len(), 2);
        assert!(extracted.contains(&c1));
        assert!(extracted.contains(&c2));
    }

    #[test]
    fn build_single_child_hash_missing_from_contracts() {
        // Hash exists in by_type but not in contracts — silently skipped.
        let index = TestIndex {
            types: HashSet::from(["sw.os".to_string()]),
            by_type: HashMap::from([("sw.os".to_string(), HashSet::from(["gone".to_string()]))]),
            by_type_slug: HashMap::from([(
                "sw.os".to_string(),
                HashMap::from([("debian".to_string(), HashSet::from(["gone".to_string()]))]),
            )]),
            contracts: HashMap::new(),
        };
        let result = build(&index);
        assert!(into_all(result).is_empty());
    }

    #[test]
    fn build_multi_slug_all_hashes_missing() {
        // Two hashes in by_type so it takes the multi-child path, but
        // type_slugs yields hashes absent from contracts.
        let index = TestIndex {
            types: HashSet::from(["sw.os".to_string()]),
            by_type: HashMap::from([(
                "sw.os".to_string(),
                HashSet::from(["gone1".to_string(), "gone2".to_string()]),
            )]),
            by_type_slug: HashMap::from([(
                "sw.os".to_string(),
                HashMap::from([(
                    "debian".to_string(),
                    HashSet::from(["gone1".to_string(), "gone2".to_string()]),
                )]),
            )]),
            contracts: HashMap::new(),
        };
        let result = build(&index);
        assert!(into_all(result).is_empty());
    }

    #[test]
    fn build_multi_type_absent_from_slug_index() {
        // type_hashes has 2 entries but type_slugs yields nothing.
        let index = TestIndex {
            types: HashSet::from(["sw.os".to_string()]),
            by_type: HashMap::from([(
                "sw.os".to_string(),
                HashSet::from(["h1".to_string(), "h2".to_string()]),
            )]),
            by_type_slug: HashMap::new(),
            contracts: HashMap::from([
                (
                    "h1".to_string(),
                    raw_contract("sw.os", "debian", Some("wheezy")),
                ),
                (
                    "h2".to_string(),
                    raw_contract("sw.os", "fedora", Some("25")),
                ),
            ]),
        };
        let result = build(&index);
        assert!(into_all(result).is_empty());
    }

    #[test]
    fn build_same_type_different_slugs() {
        let c1 = raw_contract("sw.os", "debian", Some("wheezy"));
        let c2 = raw_contract("sw.os", "fedora", Some("25"));
        let h1 = "hash1".to_string();
        let h2 = "hash2".to_string();

        let index = TestIndex {
            types: HashSet::from(["sw.os".to_string()]),
            by_type: HashMap::from([(
                "sw.os".to_string(),
                HashSet::from([h1.clone(), h2.clone()]),
            )]),
            by_type_slug: HashMap::from([(
                "sw.os".to_string(),
                HashMap::from([
                    ("debian".to_string(), HashSet::from([h1.clone()])),
                    ("fedora".to_string(), HashSet::from([h2.clone()])),
                ]),
            )]),
            contracts: HashMap::from([(h1, c1.clone()), (h2, c2.clone())]),
        };

        let result = build(&index);

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
        let h1 = "hash1".to_string();
        let h2 = "hash2".to_string();

        let index = TestIndex {
            types: HashSet::from(["sw.os".to_string()]),
            by_type: HashMap::from([(
                "sw.os".to_string(),
                HashSet::from([h1.clone(), h2.clone()]),
            )]),
            by_type_slug: HashMap::from([(
                "sw.os".to_string(),
                HashMap::from([(
                    "debian".to_string(),
                    HashSet::from([h1.clone(), h2.clone()]),
                )]),
            )]),
            contracts: HashMap::from([(h1, c1.clone()), (h2, c2.clone())]),
        };

        let result = build(&index);

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
        let h1 = "hash1".to_string();
        let h2 = "hash2".to_string();

        let index = TestIndex {
            types: HashSet::from(["sw.os".to_string()]),
            by_type: HashMap::from([(
                "sw.os".to_string(),
                HashSet::from([h1.clone(), h2.clone()]),
            )]),
            by_type_slug: HashMap::from([(
                "sw.os".to_string(),
                HashMap::from([(
                    "debian".to_string(),
                    HashSet::from([h1.clone(), h2.clone()]),
                )]),
            )]),
            contracts: HashMap::from([(h1, c1.clone()), (h2, c2.clone())]),
        };

        let result = build(&index);
        let json = serde_json::to_value(&result).unwrap();
        let arr = json.pointer("/sw/os/debian").expect("should have debian");
        assert!(arr.is_array());
        assert_eq!(arr.as_array().unwrap().len(), 2);
    }

    // -----------------------------------------------------------------------
    // Overlapping types
    // -----------------------------------------------------------------------

    // -----------------------------------------------------------------------
    // Round-trip: build → into_all
    // -----------------------------------------------------------------------

    #[test]
    fn round_trip_build_then_into_all() {
        let c1 = raw_contract("sw.os", "debian", Some("wheezy"));
        let c2 = raw_contract("sw.blob", "nodejs", Some("4.8.0"));
        let h1 = "hash1".to_string();
        let h2 = "hash2".to_string();

        let index = TestIndex {
            types: HashSet::from(["sw.os".to_string(), "sw.blob".to_string()]),
            by_type: HashMap::from([
                ("sw.os".to_string(), HashSet::from([h1.clone()])),
                ("sw.blob".to_string(), HashSet::from([h2.clone()])),
            ]),
            by_type_slug: HashMap::from([
                (
                    "sw.os".to_string(),
                    HashMap::from([("debian".to_string(), HashSet::from([h1.clone()]))]),
                ),
                (
                    "sw.blob".to_string(),
                    HashMap::from([("nodejs".to_string(), HashSet::from([h2.clone()]))]),
                ),
            ]),
            contracts: HashMap::from([(h1, c1.clone()), (h2, c2.clone())]),
        };

        let tree = build(&index);
        let extracted = into_all(tree);
        assert_eq!(extracted.len(), 2);
        assert!(extracted.contains(&c1));
        assert!(extracted.contains(&c2));
    }

    #[test]
    fn round_trip_multi_version_slug() {
        let c1 = raw_contract("sw.os", "debian", Some("wheezy"));
        let c2 = raw_contract("sw.os", "debian", Some("jessie"));
        let h1 = "hash1".to_string();
        let h2 = "hash2".to_string();

        let index = TestIndex {
            types: HashSet::from(["sw.os".to_string()]),
            by_type: HashMap::from([(
                "sw.os".to_string(),
                HashSet::from([h1.clone(), h2.clone()]),
            )]),
            by_type_slug: HashMap::from([(
                "sw.os".to_string(),
                HashMap::from([(
                    "debian".to_string(),
                    HashSet::from([h1.clone(), h2.clone()]),
                )]),
            )]),
            contracts: HashMap::from([(h1, c1.clone()), (h2, c2.clone())]),
        };

        let tree = build(&index);
        let extracted = into_all(tree);
        assert_eq!(extracted.len(), 2);
        assert!(extracted.contains(&c1));
        assert!(extracted.contains(&c2));
    }
}
