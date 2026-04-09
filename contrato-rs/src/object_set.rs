//! Hash-based set of objects, used by the requirements index for storing
//! compiled matchers and by other internal subsystems that need deduplication.
//!
//! Each entry is keyed by either a caller-supplied key or by the object's own
//! [`Identifiable::id`] (typically a deterministic hash).

use std::collections::HashMap;

use serde_json::Value;

use crate::hash::hash_object;

/// Trait for types that can produce a deterministic string identity.
///
/// Used by [`ObjectSet`] to derive a default key when no explicit key is
/// provided to [`insert`](ObjectSet::insert).
pub trait Identifiable {
    /// Returns a deterministic string identifier for this object.
    fn id(&self) -> String;
}

impl Identifiable for Value {
    /// Computes identity via [`hash_object`](crate::hash::hash_object).
    fn id(&self) -> String {
        hash_object(self)
    }
}

/// A generic set of objects keyed by unique string IDs.
///
/// Objects are stored in a [`HashMap`] mapping IDs to values. When no explicit
/// key is provided, the ID is derived from the object's [`Identifiable`]
/// implementation. Duplicate IDs are silently ignored (first-write-wins).
///
/// **Note on `contains_value`**: this method derives the ID via
/// [`Identifiable::id`] and looks it up by that key. Objects inserted with a
/// custom key that differs from their `id()` will *not* be found by
/// `contains_value` — use [`contains_key`](Self::contains_key) instead.
#[derive(Debug, Clone)]
pub struct ObjectSet<T> {
    data: HashMap<String, T>,
}

impl<T: Identifiable> ObjectSet<T> {
    /// Creates an empty `ObjectSet`.
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    /// Inserts an object using its [`Identifiable::id`] as the key.
    ///
    /// If an entry with the same key already exists, the call is a no-op
    /// (the existing entry is kept).
    pub fn insert(&mut self, object: T) {
        let id = object.id();
        self.data.entry(id).or_insert(object);
    }

    /// Inserts an object using an explicit key.
    ///
    /// If an entry with the same key already exists, the call is a no-op
    /// (the existing entry is kept).
    pub fn insert_with_key(&mut self, key: impl Into<String>, object: T) {
        self.data.entry(key.into()).or_insert(object);
    }

    /// Returns the number of objects in the set.
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Returns `true` if the set is empty.
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Returns `true` if the set contains an object whose [`Identifiable::id`]
    /// matches that of `object`.
    ///
    /// Objects inserted with a custom key that differs from their `id()` will
    /// not be found by this method — use [`contains_key`](Self::contains_key)
    /// instead.
    pub fn contains_value(&self, object: &T) -> bool {
        self.data.contains_key(&object.id())
    }

    /// Returns `true` if the set contains an entry with the given key.
    pub fn contains_key(&self, key: &str) -> bool {
        self.data.contains_key(key)
    }

    /// Returns an iterator over all objects in the set.
    ///
    /// The iteration order is not guaranteed.
    pub fn values(&self) -> impl Iterator<Item = &T> {
        self.data.values()
    }

    /// Retains only the entries whose key also exists in `other`.
    ///
    /// This is an in-place intersection: entries in `self` whose key is absent
    /// from `other` are removed.
    ///
    /// Unlike the original TypeScript implementation, `len()` is accurate after
    /// intersection — the TS version had a bug where `size()` was not updated
    /// after removing entries.
    pub fn intersection<U: Identifiable>(&mut self, other: &ObjectSet<U>) {
        self.data.retain(|key, _| other.contains_key(key));
    }
}

impl<T: Identifiable> Default for ObjectSet<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Identifiable> FromIterator<T> for ObjectSet<T> {
    /// Creates an `ObjectSet` from an iterator.
    ///
    /// Each object is inserted using its [`Identifiable::id`] as the key.
    /// Duplicates are ignored (first-write-wins).
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut set = Self::new();
        for obj in iter {
            set.insert(obj);
        }
        set
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // ── Constructor ──────────────────────────────────────────────────

    #[test]
    fn constructor_creates_empty_set() {
        let set = ObjectSet::<Value>::new();
        assert_eq!(set.values().count(), 0);
    }

    #[test]
    fn constructor_creates_set_with_objects() {
        let set = ObjectSet::from_iter(vec![json!({"foo": 1}), json!({"foo": 2})]);
        assert_eq!(set.values().count(), 2);
        assert!(set.contains_value(&json!({"foo": 1})));
        assert!(set.contains_value(&json!({"foo": 2})));
    }

    #[test]
    fn constructor_ignores_duplicates() {
        let set = ObjectSet::from_iter(vec![json!({"foo": 1}), json!({"foo": 1})]);
        assert_eq!(set.values().count(), 1);
    }

    #[test]
    fn constructor_with_custom_keys() {
        let mut set = ObjectSet::new();
        set.insert_with_key("one", json!({"foo": 1}));
        set.insert_with_key("two", json!({"foo": 2}));

        assert_eq!(set.values().count(), 2);
        assert!(set.contains_key("one"));
        assert!(set.contains_key("two"));
    }

    // ── Insert ───────────────────────────────────────────────────────

    #[test]
    fn insert_to_empty_set() {
        let mut set = ObjectSet::new();
        set.insert(json!({"foo": "bar"}));

        let all: Vec<_> = set.values().collect();
        assert_eq!(all.len(), 1);
        assert_eq!(*all[0], json!({"foo": "bar"}));
    }

    #[test]
    fn insert_to_non_empty_set() {
        let mut set = ObjectSet::from_iter(vec![json!({"foo": 1})]);
        set.insert(json!({"foo": 2}));

        assert_eq!(set.values().count(), 2);
        assert!(set.contains_value(&json!({"foo": 1})));
        assert!(set.contains_value(&json!({"foo": 2})));
    }

    #[test]
    fn insert_duplicate_is_noop() {
        let mut set = ObjectSet::from_iter(vec![json!({"foo": 1})]);
        set.insert(json!({"foo": 1}));

        assert_eq!(set.values().count(), 1);
    }

    #[test]
    fn insert_with_key_custom() {
        let mut set = ObjectSet::new();
        set.insert_with_key("foo", json!({"foo": 1}));

        assert!(set.contains_key("foo"));
        assert_eq!(set.len(), 1);
    }

    // ── len ──────────────────────────────────────────────────────────

    #[test]
    fn len_empty_set() {
        let set = ObjectSet::<Value>::new();
        assert_eq!(set.len(), 0);
    }

    #[test]
    fn len_one_object() {
        let set = ObjectSet::from_iter(vec![json!({"foo": 1})]);
        assert_eq!(set.len(), 1);
    }

    #[test]
    fn len_two_objects() {
        let set = ObjectSet::from_iter(vec![json!({"foo": 1}), json!({"foo": 2})]);
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn len_ignores_duplicates() {
        let set = ObjectSet::from_iter(vec![json!({"foo": 1}), json!({"foo": 1})]);
        assert_eq!(set.len(), 1);
    }

    #[test]
    fn len_updates_on_insert() {
        let mut set = ObjectSet::new();
        set.insert(json!({"foo": 1}));
        assert_eq!(set.len(), 1);

        set.insert(json!({"foo": 2}));
        assert_eq!(set.len(), 2);
    }

    // ── contains_value ───────────────────────────────────────────────

    #[test]
    fn contains_value_true() {
        let set = ObjectSet::from_iter(vec![json!({"foo": "bar"})]);
        assert!(set.contains_value(&json!({"foo": "bar"})));
    }

    #[test]
    fn contains_value_false() {
        let set = ObjectSet::from_iter(vec![json!({"foo": "bar"})]);
        assert!(!set.contains_value(&json!({"foo": "baz"})));
    }

    // ── contains_key ─────────────────────────────────────────────────

    #[test]
    fn contains_key_true() {
        let mut set = ObjectSet::new();
        set.insert_with_key("myid", json!({"foo": 1}));
        assert!(set.contains_key("myid"));
    }

    #[test]
    fn contains_key_false() {
        let set = ObjectSet::<Value>::new();
        assert!(!set.contains_key("nonexistent"));
    }

    // ── values ───────────────────────────────────────────────────────

    #[test]
    fn values_empty() {
        let set = ObjectSet::<Value>::new();
        assert_eq!(set.values().count(), 0);
    }

    #[test]
    fn values_returns_all() {
        let set = ObjectSet::from_iter(vec![json!({"foo": 1}), json!({"foo": 2})]);
        assert_eq!(set.values().count(), 2);
    }

    // ── Intersection ─────────────────────────────────────────────────

    #[test]
    fn intersection_of_two_sets() {
        let mut set1 = ObjectSet::new();
        set1.insert(json!({"foo": "bar"}));
        set1.insert(json!({"bar": "baz"}));

        let mut set2 = ObjectSet::new();
        set2.insert(json!({"bar": "baz"}));
        set2.insert(json!({"qux": "foo"}));

        set1.intersection(&set2);

        let all: Vec<_> = set1.values().collect();
        assert_eq!(all.len(), 1);
        assert_eq!(*all[0], json!({"bar": "baz"}));
    }

    #[test]
    fn intersection_of_disjoint_sets() {
        let mut set1 = ObjectSet::new();
        set1.insert(json!({"foo": "bar"}));
        set1.insert(json!({"bar": "baz"}));

        let mut set2 = ObjectSet::new();
        set2.insert(json!({"qux": "foo"}));

        set1.intersection(&set2);
        assert_eq!(set1.values().count(), 0);
    }

    #[test]
    fn intersection_left_empty() {
        let mut set1 = ObjectSet::<Value>::new();
        let mut set2 = ObjectSet::new();
        set2.insert(json!({"qux": "foo"}));

        set1.intersection(&set2);
        assert_eq!(set1.values().count(), 0);
    }

    #[test]
    fn intersection_right_empty() {
        let mut set1 = ObjectSet::new();
        set1.insert(json!({"qux": "foo"}));

        let set2 = ObjectSet::<Value>::new();

        set1.intersection(&set2);
        assert_eq!(set1.values().count(), 0);
    }

    #[test]
    fn intersection_with_custom_keys() {
        let mut set1 = ObjectSet::new();
        set1.insert_with_key("foobar", json!({"foo": "bar"}));
        set1.insert_with_key("barbaz", json!({"bar": "baz"}));

        let mut set2 = ObjectSet::new();
        set2.insert_with_key("barbaz", json!({"bar": "baz"}));
        set2.insert_with_key("quxfoo", json!({"qux": "foo"}));

        set1.intersection(&set2);

        let all: Vec<_> = set1.values().collect();
        assert_eq!(all.len(), 1);
        assert_eq!(*all[0], json!({"bar": "baz"}));
    }

    #[test]
    fn intersection_trusts_keys_over_values() {
        let mut set1 = ObjectSet::new();
        set1.insert_with_key("foobar", json!({"foo": "bar"}));
        set1.insert_with_key("barbaz", json!({"bar": "baz"}));

        let mut set2 = ObjectSet::new();
        // Same object value as set1's "barbaz", but different key
        set2.insert_with_key("barbaz2", json!({"bar": "baz"}));
        set2.insert_with_key("quxfoo", json!({"qux": "foo"}));

        set1.intersection(&set2);
        assert_eq!(set1.values().count(), 0);
    }

    #[test]
    fn intersection_updates_len() {
        let mut set1 = ObjectSet::new();
        set1.insert(json!({"foo": "bar"}));
        set1.insert(json!({"bar": "baz"}));
        assert_eq!(set1.len(), 2);

        let mut set2 = ObjectSet::new();
        set2.insert(json!({"bar": "baz"}));

        set1.intersection(&set2);
        assert_eq!(set1.len(), 1);
    }

    // ── Identifiable ─────────────────────────────────────────────────

    /// A custom type implementing Identifiable, to verify the generic works
    /// beyond `Value`.
    #[derive(Debug, Clone)]
    struct Tagged {
        tag: String,
        label: String,
    }

    impl Identifiable for Tagged {
        fn id(&self) -> String {
            self.tag.clone()
        }
    }

    #[test]
    fn generic_with_custom_identifiable() {
        let mut set = ObjectSet::new();
        set.insert(Tagged {
            tag: "a".into(),
            label: "first".into(),
        });
        set.insert(Tagged {
            tag: "b".into(),
            label: "second".into(),
        });
        // Duplicate identity — should be ignored
        set.insert(Tagged {
            tag: "a".into(),
            label: "duplicate".into(),
        });

        assert_eq!(set.len(), 2);
        assert!(set.contains_key("a"));
        assert!(set.contains_key("b"));

        let first = set
            .values()
            .find(|t| t.tag == "a")
            .expect("should find 'a'");
        assert_eq!(first.label, "first");
    }

    #[test]
    fn generic_contains_value() {
        let mut set = ObjectSet::new();
        set.insert(Tagged {
            tag: "x".into(),
            label: "hello".into(),
        });

        assert!(set.contains_value(&Tagged {
            tag: "x".into(),
            label: "different label, same identity".into(),
        }));
        assert!(!set.contains_value(&Tagged {
            tag: "y".into(),
            label: "hello".into(),
        }));
    }
}
