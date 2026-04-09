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

/// Error produced when [`build`] encounters conflicting tree paths.
///
/// This occurs when a dotted type path (e.g., `sw.os`) tries to create a
/// subtree at a segment that already holds a contract leaf.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathConflictError {
    /// The path segment that was already occupied by a leaf node.
    pub segment: String,
    /// The full dotted path that was being inserted.
    pub path: String,
}

impl fmt::Display for PathConflictError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "path conflict: intermediate segment '{}' in path '{}' is already a leaf node",
            self.segment, self.path
        )
    }
}

impl std::error::Error for PathConflictError {}

/// A strongly typed representation of the nested children tree.
///
/// The tree structure mirrors the JSON format used in contract serialization:
/// intermediate nodes map path segments to subtrees, while leaf nodes hold
/// one or more [`RawContract`] values.
///
/// The `Single` variant boxes its `RawContract` to keep the enum size small,
/// since `RawContract` is a large struct containing `HashMap`, `Vec`, and
/// `Option<ChildrenTree>` (recursive).
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

/// Extracts all contracts from a [`ChildrenTree`].
///
/// Recursively walks the tree and collects every [`RawContract`] found at leaf
/// positions into a flat vector.
pub fn get_all(tree: &ChildrenTree) -> Vec<RawContract> {
    let mut out = Vec::new();
    collect_all(tree, &mut out);
    out
}

/// Recursive helper that accumulates contracts into a single `Vec`, avoiding
/// intermediate allocations from `flat_map` + `collect` at each branch level.
fn collect_all(tree: &ChildrenTree, out: &mut Vec<RawContract>) {
    match tree {
        ChildrenTree::Branch(map) => {
            for child in map.values() {
                collect_all(child, out);
            }
        }
        ChildrenTree::Single(contract) => out.push(contract.as_ref().clone()),
        ChildrenTree::Multiple(contracts) => out.extend_from_slice(contracts),
    }
}

/// Trait exposing the children index data needed by [`build`].
///
/// This decouples the tree-building logic from the concrete `ChildrenIndex`
/// struct (defined in Phase 9), avoiding circular module dependencies.
pub trait ChildrenIndex {
    /// Iterates over the type strings of all child contracts.
    /// Each type string must appear at most once.
    fn child_types(&self) -> impl Iterator<Item = &str>;

    /// Returns an iterator over the unique contract hashes for the given type.
    ///
    /// The iterator must yield unique values and its `len()` must be O(1).
    /// Returns `None` if the type has no children.
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
/// * `source` - Any type implementing [`WithChildrenIndex`] (typically a
///   `ChildrenIndex`).
///
/// # Returns
///
/// A `ChildrenTree` (currently always a `Branch` variant) representing the
/// nested tree structure, or an error if the index data produces conflicting
/// tree paths.
///
/// # Errors
///
/// Returns an error if a dotted type path (e.g., `sw.os`) conflicts with an
/// already-stored leaf node at an intermediate segment.
pub fn build(source: &impl ChildrenIndex) -> Result<ChildrenTree, PathConflictError> {
    let mut tree = BTreeMap::new();

    for ty in source.child_types() {
        let Some(mut type_hashes) = source.type_hashes(ty) else {
            continue;
        };

        // Single child of this type: store directly at the type path.
        if type_hashes.len() == 1 {
            let hash = type_hashes.next().unwrap();
            if let Some(contract) = source.child_by_hash(hash) {
                set_path(
                    &mut tree,
                    ty,
                    ChildrenTree::Single(Box::new(contract.clone())),
                )?;
            }
            continue;
        }

        // Multiple children: nest by slug under the type path.
        for (slug, hashes) in source.type_slugs(ty) {
            let contracts: Vec<RawContract> = hashes
                .filter_map(|h| source.child_by_hash(h).cloned())
                .collect();

            if contracts.is_empty() {
                continue;
            }

            let node = if contracts.len() == 1 {
                ChildrenTree::Single(Box::new(contracts.into_iter().next().unwrap()))
            } else {
                ChildrenTree::Multiple(contracts)
            };

            let path = format!("{ty}.{slug}");
            set_path(&mut tree, &path, node)?;
        }
    }

    Ok(ChildrenTree::Branch(tree))
}

/// Sets a [`ChildrenTree`] node at a dotted path within a tree map, creating
/// intermediate [`ChildrenTree::Branch`] nodes as needed.
///
/// Mimics `lodash/set` behavior: `set_path(tree, "sw.os", node)` produces
/// `{ "sw" => Branch({ "os" => node }) }`.
///
/// # Errors
///
/// Returns an error if an intermediate path segment already exists as a leaf
/// node (either `Single` or `Multiple`), since a leaf cannot contain children.
fn set_path(
    tree: &mut BTreeMap<String, ChildrenTree>,
    path: &str,
    node: ChildrenTree,
) -> Result<(), PathConflictError> {
    let mut current = tree;
    let mut parts = path.split('.').peekable();

    while let Some(part) = parts.next() {
        if parts.peek().is_none() {
            current.insert(part.to_string(), node);
            return Ok(());
        }
        let entry = current
            .entry(part.to_string())
            .or_insert_with(|| ChildrenTree::Branch(BTreeMap::new()));
        match entry {
            ChildrenTree::Branch(map) => current = map,
            _ => {
                return Err(PathConflictError {
                    segment: part.to_string(),
                    path: path.to_string(),
                });
            }
        }
    }

    Ok(())
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
        assert!(get_all(&tree).is_empty());
    }

    // -----------------------------------------------------------------------
    // get_all tests
    // -----------------------------------------------------------------------

    #[test]
    fn get_all_empty_tree() {
        let tree = ChildrenTree::Branch(BTreeMap::new());
        assert!(get_all(&tree).is_empty());
    }

    #[test]
    fn get_all_root_is_single() {
        let c = raw_contract("sw.os", "debian", Some("wheezy"));
        let tree = ChildrenTree::Single(Box::new(c.clone()));
        let result = get_all(&tree);
        assert_eq!(result, vec![c]);
    }

    #[test]
    fn get_all_root_is_multiple() {
        let c1 = raw_contract("sw.os", "debian", Some("wheezy"));
        let c2 = raw_contract("sw.os", "debian", Some("jessie"));
        let tree = ChildrenTree::Multiple(vec![c1.clone(), c2.clone()]);
        let result = get_all(&tree);
        assert_eq!(result, vec![c1, c2]);
    }

    #[test]
    fn get_all_single_contract() {
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

        let result = get_all(&tree);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].body.slug.as_ref().unwrap().as_str(), "debian");
    }

    #[test]
    fn get_all_multiple_types() {
        let tree: ChildrenTree = serde_json::from_value(json!({
            "sw": {
                "os": { "type": "sw.os", "slug": "debian", "version": "wheezy" },
                "blob": { "type": "sw.blob", "slug": "nodejs", "version": "4.8.0" }
            }
        }))
        .unwrap();

        let result = get_all(&tree);
        assert_eq!(result.len(), 2);
        let slugs: HashSet<&str> = result
            .iter()
            .map(|c| c.body.slug.as_ref().unwrap().as_str())
            .collect();
        assert!(slugs.contains("debian"));
        assert!(slugs.contains("nodejs"));
    }

    #[test]
    fn get_all_nested_by_slug() {
        let tree: ChildrenTree = serde_json::from_value(json!({
            "sw": {
                "os": {
                    "debian": { "type": "sw.os", "slug": "debian", "version": "wheezy" },
                    "fedora": { "type": "sw.os", "slug": "fedora", "version": "25" }
                }
            }
        }))
        .unwrap();

        let result = get_all(&tree);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn get_all_array_of_same_slug() {
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

        let result = get_all(&tree);
        assert_eq!(result.len(), 2);
        let versions: HashSet<String> = result
            .iter()
            .map(|c| c.body.version.as_ref().unwrap().to_string())
            .collect();
        assert!(versions.contains("wheezy"));
        assert!(versions.contains("jessie"));
    }

    // -----------------------------------------------------------------------
    // build tests (ported from TS tests/children-tree/build.spec.ts)
    // -----------------------------------------------------------------------

    #[test]
    fn build_empty_index() {
        let index = empty_index();
        let result = build(&index).unwrap();
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
        let result = build(&index).unwrap();
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

        let result = build(&index).unwrap();
        let extracted = get_all(&result);
        assert_eq!(extracted.len(), 1);
        assert_eq!(extracted[0], c1);

        // Verify full tree shape: sw -> os -> Single(contract)
        let json = serde_json::to_value(&result).unwrap();
        assert_eq!(json.pointer("/sw/os/slug").unwrap(), "debian");
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

        let result = build(&index).unwrap();

        // Both types share the "sw" prefix, so they should be siblings.
        let json = serde_json::to_value(&result).unwrap();
        assert_eq!(json.pointer("/sw/os/slug").unwrap(), "debian");
        assert_eq!(json.pointer("/sw/blob/slug").unwrap(), "nodejs");

        let extracted = get_all(&result);
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
        let result = build(&index).unwrap();
        assert!(get_all(&result).is_empty());
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
        let result = build(&index).unwrap();
        assert!(get_all(&result).is_empty());
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
        let result = build(&index).unwrap();
        assert!(get_all(&result).is_empty());
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

        let result = build(&index).unwrap();
        let extracted = get_all(&result);
        assert_eq!(extracted.len(), 2);
        assert!(extracted.contains(&c1));
        assert!(extracted.contains(&c2));

        // Verify tree shape: sw -> os -> {debian, fedora}
        let json = serde_json::to_value(&result).unwrap();
        assert!(json.pointer("/sw/os/debian").is_some());
        assert!(json.pointer("/sw/os/fedora").is_some());
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

        let result = build(&index).unwrap();

        // Verify the debian node is Multiple
        let json = serde_json::to_value(&result).unwrap();
        let arr = json.pointer("/sw/os/debian").expect("should have debian");
        assert!(arr.is_array());
        assert_eq!(arr.as_array().unwrap().len(), 2);

        let extracted = get_all(&result);
        assert_eq!(extracted.len(), 2);
        assert!(extracted.contains(&c1));
        assert!(extracted.contains(&c2));
    }

    #[test]
    fn build_variants_same_slug_and_version() {
        let c1: RawContract = serde_json::from_value(json!({
            "type": "sw.os",
            "slug": "Debian Wheezy",
            "version": "wheezy",
            "requires": [{ "type": "arch.sw", "slug": "amd64" }]
        }))
        .unwrap();
        let c2: RawContract = serde_json::from_value(json!({
            "type": "sw.os",
            "slug": "Debian Wheezy",
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

        let result = build(&index).unwrap();
        let json = serde_json::to_value(&result).unwrap();
        let arr = json.pointer("/sw/os/debian").expect("should have debian");
        assert!(arr.is_array());
        assert_eq!(arr.as_array().unwrap().len(), 2);
    }

    // -----------------------------------------------------------------------
    // set_path tests
    // -----------------------------------------------------------------------

    #[test]
    fn set_path_single_segment() {
        let mut tree = BTreeMap::new();
        let contract = raw_contract("test", "foo", None);
        set_path(
            &mut tree,
            "foo",
            ChildrenTree::Single(Box::new(contract.clone())),
        )
        .unwrap();
        assert_eq!(tree.len(), 1);
        assert_eq!(tree["foo"], ChildrenTree::Single(Box::new(contract)));
    }

    #[test]
    fn set_path_nested() {
        let mut tree = BTreeMap::new();
        let contract = raw_contract("sw.os", "debian", None);
        set_path(
            &mut tree,
            "sw.os",
            ChildrenTree::Single(Box::new(contract.clone())),
        )
        .unwrap();
        match &tree["sw"] {
            ChildrenTree::Branch(inner) => {
                assert_eq!(inner["os"], ChildrenTree::Single(Box::new(contract)));
            }
            _ => panic!("expected Branch"),
        }
    }

    #[test]
    fn set_path_preserves_siblings() {
        let mut tree = BTreeMap::new();
        let c1 = raw_contract("sw.os", "debian", None);
        let c2 = raw_contract("sw.blob", "nodejs", None);
        set_path(
            &mut tree,
            "sw.os",
            ChildrenTree::Single(Box::new(c1.clone())),
        )
        .unwrap();
        set_path(
            &mut tree,
            "sw.blob",
            ChildrenTree::Single(Box::new(c2.clone())),
        )
        .unwrap();
        match &tree["sw"] {
            ChildrenTree::Branch(inner) => {
                assert_eq!(inner.len(), 2);
                assert!(inner.contains_key("os"));
                assert!(inner.contains_key("blob"));
            }
            _ => panic!("expected Branch"),
        }
    }

    #[test]
    fn set_path_three_segments() {
        let mut tree = BTreeMap::new();
        let contract = raw_contract("hw.device.type", "rpi", None);
        set_path(
            &mut tree,
            "hw.device.type",
            ChildrenTree::Single(Box::new(contract.clone())),
        )
        .unwrap();
        // Should create hw -> device -> type -> contract
        match &tree["hw"] {
            ChildrenTree::Branch(hw) => match &hw["device"] {
                ChildrenTree::Branch(device) => {
                    assert_eq!(device["type"], ChildrenTree::Single(Box::new(contract)));
                }
                _ => panic!("expected Branch at device"),
            },
            _ => panic!("expected Branch at hw"),
        }
    }

    #[test]
    fn set_path_conflict_on_multiple_leaf() {
        let mut tree = BTreeMap::new();
        let c1 = raw_contract("sw.os", "debian", Some("wheezy"));
        let c2 = raw_contract("sw.os", "debian", Some("jessie"));
        // Set "sw" as a Multiple leaf.
        set_path(
            &mut tree,
            "sw",
            ChildrenTree::Multiple(vec![c1.clone(), c2.clone()]),
        )
        .unwrap();
        // Trying to nest under "sw" should fail.
        let err = set_path(
            &mut tree,
            "sw.os",
            ChildrenTree::Single(Box::new(raw_contract("test", "x", None))),
        )
        .unwrap_err();
        assert_eq!(err.segment, "sw");
        assert_eq!(err.path, "sw.os");
    }

    #[test]
    fn path_conflict_error_display() {
        let err = PathConflictError {
            segment: "sw".to_string(),
            path: "sw.os".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("sw"));
        assert!(msg.contains("sw.os"));
        assert!(msg.contains("leaf node"));
    }

    #[test]
    fn set_path_conflict_returns_error() {
        let mut tree = BTreeMap::new();
        let c1 = raw_contract("test", "foo", None);
        let c2 = raw_contract("test.nested", "bar", None);
        // Set "sw" as a leaf.
        set_path(&mut tree, "sw", ChildrenTree::Single(Box::new(c1.clone()))).unwrap();
        // Trying to set "sw.os" should fail because "sw" is already a leaf.
        let err = set_path(
            &mut tree,
            "sw.os",
            ChildrenTree::Single(Box::new(c2.clone())),
        )
        .unwrap_err();
        assert_eq!(err.segment, "sw");
        assert_eq!(err.path, "sw.os");
    }

    // -----------------------------------------------------------------------
    // Round-trip: build → get_all
    // -----------------------------------------------------------------------

    #[test]
    fn round_trip_build_then_get_all() {
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

        let tree = build(&index).unwrap();
        let extracted = get_all(&tree);
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

        let tree = build(&index).unwrap();
        let extracted = get_all(&tree);
        assert_eq!(extracted.len(), 2);
        assert!(extracted.contains(&c1));
        assert!(extracted.contains(&c2));
    }
}
