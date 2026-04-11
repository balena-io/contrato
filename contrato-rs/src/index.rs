//! Contract index: fast lookup of child contracts by hash, type, and
//! type+slug.
//!
//! Holds all of a contract's direct children in a [`HashMap`] keyed by
//! hash, together with secondary indexes that support the public
//! `get_children_by_type`, `get_child_by_hash`, and matcher-based search
//! APIs on [`Contract`](crate::Contract).
//!
//! The index assumes its children are fully hashed before insertion
//! (construction via [`Contract::new`](crate::Contract) always hashes).
//! Each secondary index — `by_type`, `by_type_slug`, and `types` — is
//! kept in sync by [`insert`](ChildrenIndex::insert) and
//! [`remove`](ChildrenIndex::remove); the contract layer is responsible
//! for calling [`rebuild`](crate::Contract) after any mutation so that
//! the serialized tree in `raw.body.children` and the requirements index
//! stay consistent with the in-memory state.
//!
//! Mutations return a [`Mutation`] value that reports the set of
//! search-result cache types the caller must invalidate. This keeps
//! the index free of any references to [`Contract`]'s own cache
//! state, so the caller can split-borrow the index and the cache as
//! disjoint fields of [`Contract`].

use std::collections::{HashMap, HashSet};

use crate::children_tree::ChildrenIndex;
use crate::contract::Contract;
use crate::types::RawContract;

/// Index of contracts for fast lookup by hash, type, and type+slug.
///
/// Secondary indexes (`by_type`, `by_type_slug`, `types`) are
/// maintained in step with `map` by [`insert`](Self::insert) and
/// [`remove`](Self::remove), both of which return a [`Mutation`]
/// describing which cache types the caller should invalidate.
#[derive(Debug, Clone, Default)]
pub(crate) struct ContractIndex {
    /// Maps contract hashes to owned child contracts.
    map: HashMap<String, Contract>,

    /// Maps a contract type to the set of child hashes having that type.
    by_type: HashMap<String, HashSet<String>>,

    /// Maps type → slug (including aliases) → set of child hashes.
    by_type_slug: HashMap<String, HashMap<String, HashSet<String>>>,

    /// Set of contract types currently known to this index.
    types: HashSet<String>,
}

/// Outcome of a mutation on [`ChildrenIndex`] — tells the caller
/// whether derived state needs rebuilding and, when it does, which
/// cache types must be invalidated.
///
/// Returned by [`ChildrenIndex::insert`] and [`ChildrenIndex::remove`]
/// so the caller (typically [`Contract`]) can invalidate
/// [`Contract::search_cache`](crate::Contract) via split-borrow
/// without the index itself holding a reference to the cache.
///
/// Marked `#[must_use]` because discarding a `Mutation` silently
/// skips both the rebuild of derived state and the search-cache
/// invalidation — a latent bug that leaves stale cache entries
/// pointing at removed children. Every caller must explicitly
/// inspect `changed` and handle `invalidated_types`.
#[must_use]
pub(crate) struct Mutation {
    /// Whether the index was actually changed.
    ///
    /// `false` when the mutation was a no-op — a duplicate insert
    /// or a remove of a non-existent hash. Callers skip the
    /// `rebuild` + hash-invalidation work in that case.
    pub changed: bool,

    /// Types whose search-cache entries went stale.
    ///
    /// Empty when `changed` is `false`. Otherwise contains the
    /// mutated contract's own type plus every type reachable
    /// inside its subtree — every cached search result keyed on
    /// any of these types must be dropped because the walk that
    /// built it may now be inaccurate. Clearing only the own type
    /// would leave stale results when a child carrying nested
    /// contracts of a previously-cached type is inserted or
    /// removed.
    pub invalidated_types: Vec<String>,
}

impl Mutation {
    /// Builds a `Mutation` that signals no change — used for the
    /// duplicate-insert and missing-remove cases. Avoids allocating
    /// the invalidation vector on the no-op path.
    fn unchanged() -> Self {
        Self {
            changed: false,
            invalidated_types: Vec::new(),
        }
    }
}

impl ContractIndex {
    /// Inserts a child contract into the index.
    ///
    /// Returns a [`Mutation`] describing what the caller needs to do
    /// next: `changed` is `false` for a duplicate hash (full no-op,
    /// caller skips rebuild), otherwise `true` and `invalidated_types`
    /// lists every type whose cached search results may now be stale.
    ///
    /// The child's hash is requested via [`Contract::hash`], which is
    /// computed lazily on first access — so insertion is the point at
    /// which a previously-unhashed contract becomes hashed.
    pub(crate) fn insert(&mut self, contract: Contract) -> Mutation {
        let child_hash = contract.hash().to_string();
        if self.map.contains_key(&child_hash) {
            return Mutation::unchanged();
        }
        let ty = contract.get_type().to_string();
        // Collected before `self.map.insert` moves `contract`, so
        // the search for invalidated types runs against the fully
        // populated inbound subtree.
        let mut invalidated_types = contract.get_children_types();
        invalidated_types.push(ty.clone());

        // Keep `types` in sync with `by_type` without a redundant
        // membership probe on every index. One `contains_key` check
        // decides whether `types` needs an update, then the `entry`
        // API handles the inner container creation for `by_type` and
        // `by_type_slug` without any `.expect(...)` on a just-inserted
        // key.
        if !self.by_type.contains_key(&ty) {
            self.types.insert(ty.clone());
        }
        self.by_type
            .entry(ty.clone())
            .or_default()
            .insert(child_hash.clone());

        let slug_map = self.by_type_slug.entry(ty).or_default();
        for slug in contract.get_all_slugs() {
            slug_map
                .entry(slug.to_string())
                .or_default()
                .insert(child_hash.clone());
        }

        self.map.insert(child_hash, contract);
        Mutation {
            changed: true,
            invalidated_types,
        }
    }

    /// Removes a child contract from the index.
    ///
    /// Returns a [`Mutation`] with the same semantics as
    /// [`Self::insert`]'s return: `changed` is `false` when the hash
    /// is not present (full no-op), otherwise `true` and
    /// `invalidated_types` lists the types whose cached searches
    /// need to be dropped.
    ///
    /// When the last child of a given type is removed, the
    /// corresponding entries in `by_type`, `by_type_slug`, and
    /// `types` are cleaned up so the index does not retain empty
    /// shells. A child is identified by its hash, so the caller may
    /// pass any [`Contract`] that hashes to the same value as the
    /// stored child. Because [`Contract::hash`] is a deterministic
    /// SHA-256 of the serialized raw data, two contracts with equal
    /// hashes have identical raw data — in particular, identical
    /// slug and alias sets — so the slug-cleanup loop below removes
    /// the same keys that the stored child registered on insertion.
    pub(crate) fn remove(&mut self, contract: &Contract) -> Mutation {
        let child_hash = contract.hash();
        if !self.map.contains_key(child_hash) {
            return Mutation::unchanged();
        }
        let ty = contract.get_type().to_string();
        // Same equality-of-hash argument as the slug cleanup below:
        // the stored child and the caller-supplied `contract` carry
        // identical raw data, so `get_children_types` is stable
        // across the two references.
        let mut invalidated_types = contract.get_children_types();
        invalidated_types.push(ty.clone());
        // Rebind to owned so that later `&mut self` operations don't
        // collide with the borrow from `contract.hash()`.
        let child_hash = child_hash.to_string();
        self.map.remove(&child_hash);

        if let Some(hashes) = self.by_type.get_mut(&ty) {
            hashes.remove(&child_hash);
            if hashes.is_empty() {
                self.by_type.remove(&ty);
                self.types.remove(&ty);
            }
        }

        if let Some(slug_map) = self.by_type_slug.get_mut(&ty) {
            for slug in contract.get_all_slugs() {
                if let Some(hashes) = slug_map.get_mut(slug) {
                    hashes.remove(&child_hash);
                    if hashes.is_empty() {
                        slug_map.remove(slug);
                    }
                }
            }
            if slug_map.is_empty() {
                self.by_type_slug.remove(&ty);
            }
        }

        Mutation {
            changed: true,
            invalidated_types,
        }
    }

    /// Returns `true` if the index contains no children.
    pub(crate) fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// Returns `true` if the index knows at least one child of `ty`.
    ///
    /// Used as a fast-rejection check by matcher-based child search:
    /// when a candidate parent has no children of the target type,
    /// the walk over its secondary indexes is skipped entirely.
    pub(crate) fn has_type(&self, ty: &str) -> bool {
        self.types.contains(ty)
    }

    /// Returns an iterator over the child hashes indexed under
    /// `(ty, slug)`.
    ///
    /// Yields nothing if no children are registered for that pair.
    /// The iterator borrows from the index; callers that need to
    /// retain the hashes past further mutations must clone them.
    pub(crate) fn hashes_by_type_slug<'a>(
        &'a self,
        ty: &str,
        slug: &str,
    ) -> impl Iterator<Item = &'a str> + 'a {
        self.by_type_slug
            .get(ty)
            .and_then(|m| m.get(slug))
            .into_iter()
            .flat_map(|set| set.iter().map(String::as_str))
    }

    /// Returns an iterator over the child hashes indexed under `ty`.
    ///
    /// Yields nothing if no children of that type are registered.
    /// The iterator borrows from the index; callers that need to
    /// retain the hashes past further mutations must clone them.
    pub(crate) fn hashes_by_type<'a>(&'a self, ty: &str) -> impl Iterator<Item = &'a str> + 'a {
        self.by_type
            .get(ty)
            .into_iter()
            .flat_map(|set| set.iter().map(String::as_str))
    }

    /// Returns an iterator over the known child types.
    pub(crate) fn types(&self) -> impl Iterator<Item = &str> {
        self.types.iter().map(String::as_str)
    }

    /// Looks up a child contract by its hash.
    pub(crate) fn get(&self, hash: &str) -> Option<&Contract> {
        self.map.get(hash)
    }

    /// Returns an iterator over all direct children in the index.
    ///
    /// The iteration order follows the underlying [`HashMap`] and is
    /// therefore unspecified.
    pub(crate) fn values(&self) -> impl Iterator<Item = &Contract> {
        self.map.values()
    }
}

impl ChildrenIndex for ContractIndex {
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
        self.by_type_slug.get(ty).into_iter().flat_map(|m| {
            m.iter()
                .map(|(slug, hashes)| (slug.as_str(), hashes.iter().map(String::as_str)))
        })
    }

    fn child_by_hash(&self, hash: &str) -> Option<&RawContract> {
        self.map.get(hash).map(Contract::raw)
    }
}
