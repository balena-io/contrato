//! Two-level cache mapping `(target type, matcher hash) → value`, with
//! per-type invalidation.
//!
//! The children index uses this to memoize matcher-based child searches.
//! When a child of a given type is added or removed, only that type's
//! cache entries are dropped — other entries stay warm, so unrelated
//! searches still resolve in O(1).
//! without silencing other dead-code signals.

use std::collections::HashMap;

use crate::matcher::Matcher;

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
pub(crate) struct MatcherCache<V> {
    /// `type → (matcher_hash → value)`
    data: HashMap<String, HashMap<String, V>>,
}

impl<V> MatcherCache<V> {
    /// Creates an empty `MatcherCache`.
    pub(crate) fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    /// Stores a value in the cache keyed by the given matcher.
    ///
    /// The cache key is derived from the matcher's [`kind`](Matcher::kind)
    /// and [`hash`](Matcher::hash). If an entry with the same key already
    /// exists, it is overwritten.
    pub(crate) fn insert(&mut self, matcher: &impl Matcher, value: V) {
        self.data
            .entry(matcher.kind().to_string())
            .or_default()
            .insert(matcher.hash().to_string(), value);
    }

    /// Retrieves a cached value for the given matcher.
    ///
    /// Returns `None` if no entry exists for the matcher's cache key.
    pub(crate) fn get(&self, matcher: &impl Matcher) -> Option<&V> {
        self.data
            .get(matcher.kind())
            .and_then(|by_hash| by_hash.get(matcher.hash()))
    }

    /// Removes all cache entries for the given type.
    ///
    /// If the type does not exist in the cache, this is a no-op.
    pub(crate) fn remove(&mut self, matcher_kind: &str) {
        self.data.remove(matcher_kind);
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
    use crate::matcher::Matcher;
    use serde_json::json;

    fn types<V>(cache: &MatcherCache<V>) -> impl Iterator<Item = &str> {
        cache.data.keys().map(String::as_str)
    }

    /// Minimal stand-in for a real matcher contract. Carries the
    /// target contract type and a deterministic hash derived from the
    /// match criteria — exactly the two fields that back the cache
    /// key — without dragging in the full [`Contract`](crate::Contract)
    /// lifecycle.
    struct TestMatcher {
        target_type: String,
        hash: String,
    }

    impl TestMatcher {
        /// Creates a test matcher for the given type and slug.
        ///
        /// The hash is a deterministic digest over a canonical JSON
        /// shape carrying both keys, so two `TestMatcher`s with the
        /// same `(contract_type, slug)` produce the same cache key.
        fn new(contract_type: &str, slug: &str) -> Self {
            Self {
                target_type: contract_type.to_string(),
                hash: hash_object(&json!({
                    "type": contract_type,
                    "slug": slug,
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
        assert_eq!(types(&cache).count(), 0);
    }

    #[test]
    fn types_single_type() {
        let mut cache = MatcherCache::new();
        let m = TestMatcher::new("sw.os", "debian");
        cache.insert(&m, true);

        let types: Vec<&str> = types(&cache).collect();
        assert_eq!(types, ["sw.os"]);
    }

    #[test]
    fn types_no_duplicates() {
        let mut cache = MatcherCache::new();
        let m1 = TestMatcher::new("sw.os", "debian");
        let m2 = TestMatcher::new("sw.os", "fedora");

        cache.insert(&m1, true);
        cache.insert(&m2, false);

        let types: Vec<&str> = types(&cache).collect();
        assert_eq!(types, ["sw.os"]);
    }

    #[test]
    fn types_multiple_types() {
        let mut cache = MatcherCache::new();
        let m1 = TestMatcher::new("sw.os", "debian");
        let m2 = TestMatcher::new("sw.stack", "nodejs");

        cache.insert(&m1, true);
        cache.insert(&m2, false);

        let mut types: Vec<&str> = types(&cache).collect();
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

        let types: Vec<&str> = types(&cache).collect();
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
}
