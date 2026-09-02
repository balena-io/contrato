//! Contract index: fast lookup of child contracts by hash, type, and
//! type+slug.
//!
//! The index assumes its children are fully hashed before insertion
//! (construction via [`Contract::new`](crate::Contract) always hashes).
//! Each secondary index — `by_type`, `by_type_slug`, and `types` — is
//! kept in sync by [`insert_all`](ContractIndex::insert_all) and
//! [`remove_by_hash`](ContractIndex::remove_by_hash).
//!
//! Insertion also rejects children that cannot be serialized into a tree: contracts with
//! overlapping types and slugless contracts

use std::collections::BTreeSet;

use indexmap::{IndexMap, IndexSet};

use crate::contract::Contract;
use crate::error::Error;
use crate::path::DottedPath;

/// Checks that a child can be keyed in the children tree by its type
/// and slug.
///
/// # Errors
///
/// Returns [`Error::InvalidChildType`] when the type cannot be split
/// into tree segments, and [`Error::MissingChildSlug`] when the child
/// as no slug to be nested under.
fn validate_child(kind: &str, slug: Option<&str>) -> Result<(), Error> {
    if !DottedPath::is_valid(kind) {
        return Err(Error::InvalidChildType {
            kind: kind.to_string(),
        });
    }

    if slug.is_none() {
        return Err(Error::MissingChildSlug {
            kind: kind.to_string(),
        });
    }

    Ok(())
}

/// Checks that no child type is a proper prefix of another, e.g. `sw.os`
/// and `sw.os.kernel`.
///
/// Such a pair cannot be keyed in the same tree: the shorter type needs a
/// leaf exactly where the longer one needs a subtree.
fn validate_types(types: &BTreeSet<&str>) -> Result<(), Error> {
    for kind in types.iter().copied() {
        // Every proper prefix of a dotted type is a slice of it up to a dot.
        for (dot, _) in kind.match_indices('.') {
            if types.contains(&kind[..dot]) {
                return Err(Error::OverlappingChildTypes {
                    outer: kind[..dot].to_string(),
                    inner: kind.to_string(),
                });
            }
        }
    }

    Ok(())
}

/// Index of contracts for fast lookup by hash, type, and type+slug.
///
/// Secondary indexes (`by_type`, `by_type_slug`, `types`) are
/// maintained in step with `map` by [`insert_all`](Self::insert_all) and
/// [`remove_by_hash`](Self::remove_by_hash), both of which report
/// whether the caller should rebuild derived state.
#[derive(Debug, Clone, Default)]
pub(crate) struct ContractIndex {
    /// Maps contract hashes to owned child contracts, in insertion order.
    map: IndexMap<String, Contract>,

    /// Maps a contract type to the set of child hashes having that type,
    /// preserving insertion order.
    by_type: IndexMap<String, IndexSet<String>>,

    /// Maps type → slug (including aliases) → set of child hashes, all
    /// preserving insertion order.
    by_type_slug: IndexMap<String, IndexMap<String, IndexSet<String>>>,

    /// Set of contract types currently known to this index, in insertion
    /// order.
    types: IndexSet<String>,
}

impl ContractIndex {
    /// Inserts a batch of child contracts, skipping duplicate hashes.
    ///
    /// Returns `true` when the index changed, `false` when every contract
    /// was a duplicate or the batch was empty.
    ///
    /// The batch is validated before the first insertion, so the call is
    /// all-or-nothing.
    ///
    /// # Errors
    ///
    /// Returns the first [`Error`] raised by [`validate_child`] for an
    /// incoming contract, or by [`validate_types`] for the types the index
    /// would end up holding.
    pub(crate) fn insert_all(&mut self, mut incoming: Vec<Contract>) -> Result<bool, Error> {
        incoming.retain(|contract| !self.map.contains_key(contract.hash()));

        if incoming.is_empty() {
            return Ok(false);
        }

        for contract in &incoming {
            validate_child(contract.get_type(), contract.get_slug())?;
        }

        // Overlaps are a property of the whole set, so the stored types
        // are checked alongside the incoming ones.
        let mut types: BTreeSet<&str> = self.types().collect();
        types.extend(incoming.iter().map(Contract::get_type));
        validate_types(&types)?;

        for contract in incoming {
            self.insert(contract);
        }

        Ok(true)
    }

    /// Appends an admitted child, keeping every secondary index in step.
    ///
    /// The child's hash is requested via [`Contract::hash`], which is
    /// computed lazily on first access — so insertion is the point at
    /// which a previously-unhashed contract becomes hashed.
    fn insert(&mut self, contract: Contract) {
        let child_hash = contract.hash().to_string();
        let ty = contract.get_type().to_string();

        // Keep `types` in sync with `by_type` without a redundant
        // membership probe on every index.
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
    }

    /// Removes a child contract by hash, reporting whether the index
    /// changed. A missing hash is a full no-op.
    ///
    /// When the last child of a given type is removed, the
    /// corresponding entries in `by_type`, `by_type_slug`, and `types`
    /// are cleaned up so the index does not retain empty shells. The
    /// slug cleanup walks the slugs of the *stored* contract, so it
    /// removes exactly the keys that were registered on insertion.
    #[must_use]
    pub(crate) fn remove_by_hash(&mut self, hash: &str) -> bool {
        // `shift_remove` preserves the insertion order of the remaining
        // entries (unlike `swap_remove`), so the surviving children keep
        // the order in which they were added.
        let Some(contract) = self.map.shift_remove(hash) else {
            return false;
        };
        let ty = contract.get_type().to_string();

        if let Some(hashes) = self.by_type.get_mut(&ty) {
            hashes.shift_remove(hash);
            if hashes.is_empty() {
                self.by_type.shift_remove(&ty);
                self.types.shift_remove(&ty);
            }
        }

        if let Some(slug_map) = self.by_type_slug.get_mut(&ty) {
            for slug in contract.get_all_slugs() {
                if let Some(hashes) = slug_map.get_mut(slug) {
                    hashes.shift_remove(hash);
                    if hashes.is_empty() {
                        slug_map.shift_remove(slug);
                    }
                }
            }
            if slug_map.is_empty() {
                self.by_type_slug.shift_remove(&ty);
            }
        }

        true
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

    /// Returns an iterator over the slugs (including aliases) registered
    /// for `ty`.
    ///
    /// Yields nothing if no children of that type are registered.
    pub(crate) fn slugs_by_type<'a>(&'a self, ty: &str) -> impl Iterator<Item = &'a str> {
        self.by_type_slug
            .get(ty)
            .into_iter()
            .flat_map(|m| m.keys().map(String::as_str))
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
    /// The iteration order follows the underlying [`IndexMap`] and is
    /// therefore the order in which the children were inserted.
    pub(crate) fn values(&self) -> impl Iterator<Item = &Contract> {
        self.map.values()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_types_rejects_a_type_that_extends_another() {
        for (outer, inner) in [("sw", "sw.os"), ("sw.os", "sw.os.kernel")] {
            let types = BTreeSet::from([outer, inner]);
            assert!(matches!(
                validate_types(&types).unwrap_err(),
                Error::OverlappingChildTypes { outer: o, inner: i } if o == outer && i == inner
            ));
        }
    }

    /// Sharing a prefix is not overlapping: neither type is a prefix of
    /// the other.
    #[test]
    fn validate_types_accepts_types_that_only_share_a_prefix() {
        let types = BTreeSet::from(["sw.os", "sw.blob", "hw.device-type"]);
        assert!(validate_types(&types).is_ok());
    }

    #[test]
    fn validate_child_requires_a_slug() {
        assert!(matches!(
            validate_child("sw.os", None).unwrap_err(),
            Error::MissingChildSlug { kind } if kind == "sw.os"
        ));
        assert!(validate_child("sw.os", Some("debian")).is_ok());
    }

    #[test]
    fn validate_child_rejects_a_type_that_is_not_a_tree_path() {
        assert!(matches!(
            validate_child("sw..os", Some("debian")).unwrap_err(),
            Error::InvalidChildType { kind } if kind == "sw..os"
        ));
    }
}
