//! Cache for search results keyed by matcher hash and type, with per-type
//! invalidation.
//!
//! Used by [`ChildrenIndex`](crate::children::ChildrenIndex) to cache
//! `find_children()` results. When children of a given type are added or
//! removed, only that type's cache entries are invalidated rather than
//! clearing the entire cache.

use std::collections::HashMap;

/// A contract matcher that can be used as a cache key.
///
/// In the TypeScript implementation, matchers are `Contract` instances with
/// `type: "meta.matcher"`. The cache path is derived from
/// `[matcher.raw.data.type, matcher.metadata.hash]`. This trait captures
/// those two properties so that `MatcherCache` can accept any matcher
/// directly.
pub trait Matcher {
    /// The contract type this matcher targets (e.g. `"sw.os"`).
    fn kind(&self) -> &str;

    /// The deterministic hash of this matcher.
    fn hash(&self) -> &str;
}

/// A two-level cache mapping `(type, matcher_hash) → value`.
///
/// The first level groups entries by contract type, enabling efficient
/// per-type invalidation via [`remove`](Self::remove). The second level
/// maps individual matcher hashes to their cached values.
///
/// # Type parameter
///
/// `V` is the cached value type. In practice this is typically a set of
/// contract hashes returned by a search operation.
#[derive(Debug, Clone)]
pub struct MatcherCache<V> {
    /// `type → (matcher_hash → value)`
    data: HashMap<String, HashMap<String, V>>,
}

impl<V> MatcherCache<V> {
    /// Creates an empty `MatcherCache`.
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    /// Stores a value in the cache keyed by the given matcher.
    ///
    /// The cache key is derived from the matcher's [`kind`](Matcher::kind)
    /// and [`hash`](Matcher::hash). If an entry with the same key already
    /// exists, it is overwritten.
    pub fn insert(&mut self, matcher: &impl Matcher, value: V) {
        self.data
            .entry(matcher.kind().to_string())
            .or_default()
            .insert(matcher.hash().to_string(), value);
    }

    /// Retrieves a cached value for the given matcher.
    ///
    /// Returns `None` if no entry exists for the matcher's cache key.
    pub fn get(&self, matcher: &impl Matcher) -> Option<&V> {
        self.data
            .get(matcher.kind())
            .and_then(|by_hash| by_hash.get(matcher.hash()))
    }

    /// Returns an iterator over the types that have at least one cache entry.
    pub fn types(&self) -> impl Iterator<Item = &str> {
        self.data.keys().map(String::as_str)
    }

    /// Removes all cache entries for the given type.
    ///
    /// If the type does not exist in the cache, this is a no-op.
    pub fn remove(&mut self, matcher_kind: &str) {
        self.data.remove(matcher_kind);
    }

    /// Merges another cache into this one, consuming it.
    ///
    /// For each type in `other`:
    /// - If this cache already has entries for that type, the type is **reset**
    ///   (all entries removed) in this cache. The entries from `other` are NOT
    ///   copied — the overlap signals stale data.
    /// - If this cache does not have the type, the entries are moved from
    ///   `other` into this cache.
    pub fn merge(&mut self, other: MatcherCache<V>) {
        for (kind, entries) in other.data {
            match self.data.entry(kind) {
                std::collections::hash_map::Entry::Occupied(e) => {
                    e.remove();
                }
                std::collections::hash_map::Entry::Vacant(e) => {
                    e.insert(entries);
                }
            }
        }
    }
}

impl<V> Default for MatcherCache<V> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::hash::hash_object;
    use serde_json::json;

    /// A lightweight matcher stand-in for tests. Mirrors the two fields
    /// that `Contract.createMatcher({type, slug})` would produce:
    /// `data.type` (the target type) and `metadata.hash` (the matcher hash).
    struct TestMatcher {
        target_type: String,
        hash: String,
    }

    impl TestMatcher {
        /// Creates a test matcher for the given type and slug.
        ///
        /// The hash is computed the same way a real matcher contract would
        /// be hashed — from the full `{type: "meta.matcher", data: {type, slug}}`
        /// JSON object.
        fn new(contract_type: &str, slug: &str) -> Self {
            Self {
                target_type: contract_type.to_string(),
                hash: hash_object(&json!({
                    "type": "meta.matcher",
                    "data": {
                        "type": contract_type,
                        "slug": slug,
                    }
                })),
            }
        }
    }

    impl Matcher for TestMatcher {
        fn kind(&self) -> &str {
            &self.target_type
        }

        fn hash(&self) -> &str {
            &self.hash
        }
    }

    // ── Constructor ──────────────────────────────────────────────────

    #[test]
    fn constructor_creates_empty_cache() {
        let cache = MatcherCache::<bool>::new();
        assert_eq!(cache.data.len(), 0);
    }

    // ── insert ───────────────────────────────────────────────────────

    #[test]
    fn insert_one_value() {
        let mut cache = MatcherCache::new();
        let m = TestMatcher::new("sw.os", "debian");

        cache.insert(&m, true);

        assert_eq!(cache.data.len(), 1);
        assert!(cache.data.contains_key("sw.os"));
        assert!(cache.data["sw.os"][&m.hash]);
    }

    #[test]
    fn insert_two_values_same_type() {
        let mut cache = MatcherCache::new();
        let m1 = TestMatcher::new("sw.os", "debian");
        let m2 = TestMatcher::new("sw.os", "fedora");

        cache.insert(&m1, true);
        cache.insert(&m2, false);

        assert_eq!(cache.data.len(), 1);
        assert_eq!(cache.data["sw.os"].len(), 2);
        assert!(cache.data["sw.os"][&m1.hash]);
        assert!(!cache.data["sw.os"][&m2.hash]);
    }

    #[test]
    fn insert_two_values_different_types() {
        let mut cache = MatcherCache::new();
        let m1 = TestMatcher::new("sw.os", "debian");
        let m2 = TestMatcher::new("sw.stack", "nodejs");

        cache.insert(&m1, true);
        cache.insert(&m2, false);

        assert_eq!(cache.data.len(), 2);
        assert!(cache.data["sw.os"][&m1.hash]);
        assert!(!cache.data["sw.stack"][&m2.hash]);
    }

    #[test]
    fn insert_overwrites_existing_value() {
        let mut cache = MatcherCache::new();
        let m = TestMatcher::new("sw.os", "debian");

        cache.insert(&m, "first");
        cache.insert(&m, "second");

        assert_eq!(cache.get(&m), Some(&"second"));
        assert_eq!(cache.data["sw.os"].len(), 1);
    }

    // ── get ──────────────────────────────────────────────────────────

    #[test]
    fn get_returns_cached_value() {
        let mut cache = MatcherCache::new();
        let m = TestMatcher::new("sw.os", "debian");

        cache.insert(&m, "debian result");

        assert_eq!(cache.get(&m), Some(&"debian result"));
    }

    #[test]
    fn get_returns_none_for_uncached_matcher() {
        let mut cache = MatcherCache::new();
        let m1 = TestMatcher::new("sw.os", "debian");
        let m2 = TestMatcher::new("sw.os", "fedora");

        cache.insert(&m1, "debian result");

        assert_eq!(cache.get(&m2), None);
    }

    #[test]
    fn get_returns_none_for_empty_cache() {
        let cache = MatcherCache::<String>::new();
        let m = TestMatcher::new("sw.os", "debian");

        assert_eq!(cache.get(&m), None);
    }

    #[test]
    fn get_returns_none_after_remove() {
        let mut cache = MatcherCache::new();
        let m = TestMatcher::new("sw.os", "debian");

        cache.insert(&m, true);
        cache.remove("sw.os");

        assert_eq!(cache.get(&m), None);
    }

    // ── types ────────────────────────────────────────────────────────

    #[test]
    fn types_empty_cache() {
        let cache = MatcherCache::<bool>::new();
        assert_eq!(cache.types().count(), 0);
    }

    #[test]
    fn types_single_type() {
        let mut cache = MatcherCache::new();
        let m = TestMatcher::new("sw.os", "debian");
        cache.insert(&m, true);

        let types: Vec<&str> = cache.types().collect();
        assert_eq!(types, ["sw.os"]);
    }

    #[test]
    fn types_no_duplicates() {
        let mut cache = MatcherCache::new();
        let m1 = TestMatcher::new("sw.os", "debian");
        let m2 = TestMatcher::new("sw.os", "fedora");

        cache.insert(&m1, true);
        cache.insert(&m2, false);

        let types: Vec<&str> = cache.types().collect();
        assert_eq!(types, ["sw.os"]);
    }

    #[test]
    fn types_multiple_types() {
        let mut cache = MatcherCache::new();
        let m1 = TestMatcher::new("sw.os", "debian");
        let m2 = TestMatcher::new("sw.stack", "nodejs");

        cache.insert(&m1, true);
        cache.insert(&m2, false);

        let mut types: Vec<&str> = cache.types().collect();
        types.sort();
        assert_eq!(types, ["sw.os", "sw.stack"]);
    }

    #[test]
    fn types_excludes_removed_types() {
        let mut cache = MatcherCache::new();
        let m1 = TestMatcher::new("sw.os", "debian");
        let m2 = TestMatcher::new("sw.stack", "nodejs");

        cache.insert(&m1, true);
        cache.insert(&m2, false);

        cache.remove("sw.stack");

        let types: Vec<&str> = cache.types().collect();
        assert_eq!(types, ["sw.os"]);
    }

    // ── remove ───────────────────────────────────────────────────────

    #[test]
    fn remove_all_entries_of_type() {
        let mut cache = MatcherCache::new();
        let m1 = TestMatcher::new("sw.os", "debian");
        let m2 = TestMatcher::new("sw.os", "fedora");
        let m3 = TestMatcher::new("sw.stack", "nodejs");

        cache.insert(&m1, true);
        cache.insert(&m2, false);
        cache.insert(&m3, true);

        cache.remove("sw.os");

        assert!(!cache.data.contains_key("sw.os"));
        assert!(cache.data.contains_key("sw.stack"));
        assert!(cache.data["sw.stack"][&m3.hash]);
    }

    #[test]
    fn remove_noop_for_nonexistent_type() {
        let mut cache = MatcherCache::new();
        let m1 = TestMatcher::new("sw.os", "debian");
        let m2 = TestMatcher::new("sw.os", "fedora");
        let m3 = TestMatcher::new("sw.stack", "nodejs");

        cache.insert(&m1, true);
        cache.insert(&m2, false);
        cache.insert(&m3, true);

        cache.remove("foobar");

        assert_eq!(cache.data.len(), 2);
        assert_eq!(cache.data["sw.os"].len(), 2);
        assert_eq!(cache.data["sw.stack"].len(), 1);
    }

    // ── merge ────────────────────────────────────────────────────────

    #[test]
    fn merge_two_empty_caches() {
        let mut cache1 = MatcherCache::<bool>::new();
        let cache2 = MatcherCache::<bool>::new();

        cache1.merge(cache2);

        assert!(cache1.data.is_empty());
    }

    #[test]
    fn merge_nonempty_into_empty_one_type() {
        let mut cache1 = MatcherCache::new();
        let mut cache2 = MatcherCache::new();

        let m1 = TestMatcher::new("sw.os", "debian");
        let m2 = TestMatcher::new("sw.os", "fedora");

        cache2.insert(&m1, true);
        cache2.insert(&m2, true);

        cache1.merge(cache2);

        assert_eq!(cache1.data.len(), 1);
        assert_eq!(cache1.data["sw.os"].len(), 2);
        assert!(cache1.data["sw.os"][&m1.hash]);
        assert!(cache1.data["sw.os"][&m2.hash]);
    }

    #[test]
    fn merge_nonempty_into_empty_two_types() {
        let mut cache1 = MatcherCache::new();
        let mut cache2 = MatcherCache::new();

        let m1 = TestMatcher::new("sw.os", "debian");
        let m2 = TestMatcher::new("sw.blob", "nodejs");

        cache2.insert(&m1, true);
        cache2.insert(&m2, true);

        cache1.merge(cache2);

        assert_eq!(cache1.data.len(), 2);
        assert!(cache1.data["sw.os"][&m1.hash]);
        assert!(cache1.data["sw.blob"][&m2.hash]);
    }

    #[test]
    fn merge_disjoint_types() {
        let mut cache1 = MatcherCache::new();
        let mut cache2 = MatcherCache::new();

        let m1 = TestMatcher::new("sw.os", "debian");
        let m2 = TestMatcher::new("sw.blob", "nodejs");

        cache1.insert(&m1, true);
        cache2.insert(&m2, true);

        cache1.merge(cache2);

        assert_eq!(cache1.data.len(), 2);
        assert!(cache1.data["sw.os"][&m1.hash]);
        assert!(cache1.data["sw.blob"][&m2.hash]);
    }

    #[test]
    fn merge_overlapping_types_resets_overlap() {
        let mut cache1 = MatcherCache::new();
        let mut cache2 = MatcherCache::new();

        let m1 = TestMatcher::new("sw.os", "debian");
        let m2 = TestMatcher::new("sw.blob", "nodejs");
        let m3 = TestMatcher::new("sw.os", "fedora");

        cache1.insert(&m1, true);
        cache2.insert(&m2, true);
        cache2.insert(&m3, true);

        cache1.merge(cache2);

        // sw.os was in both → reset in cache1, entries from cache2 NOT copied
        assert!(!cache1.data.contains_key("sw.os"));
        assert!(cache1.types().all(|t| t != "sw.os"));
        // sw.blob was only in cache2 → moved to cache1
        assert_eq!(cache1.data.len(), 1);
        assert!(cache1.data["sw.blob"][&m2.hash]);
    }
}
