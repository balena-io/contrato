//! The [`Universe`] newtype: a top-level [`Contract`] of type
//! `meta.universe` that aggregates other contracts as children.
//!
//! `Universe` is a thin wrapper whose sole purpose is to construct a
//! contract with the well-known `meta.universe` type. All contract
//! operations — adding children, searching, requirement checks — work
//! transparently via [`Deref`] / [`DerefMut`] to the inner [`Contract`].

use std::ops::{Deref, DerefMut};

use crate::contract::Contract;
use crate::types::{ContractType, RawContract, UNIVERSE};

/// A universe: a [`Contract`] with type `meta.universe` used as the
/// root container for a collection of contracts.
#[derive(Debug, Clone)]
pub struct Universe(Contract);

impl Universe {
    /// Creates an empty universe.
    pub fn new() -> Self {
        Self(Contract::new(RawContract {
            kind: ContractType::new(UNIVERSE),
            ..RawContract::default()
        }))
    }

    /// Consumes the universe and returns the inner [`Contract`].
    pub fn into_inner(self) -> Contract {
        self.0
    }
}

impl Default for Universe {
    fn default() -> Self {
        Self::new()
    }
}

impl Deref for Universe {
    type Target = Contract;

    fn deref(&self) -> &Contract {
        &self.0
    }
}

impl DerefMut for Universe {
    fn deref_mut(&mut self) -> &mut Contract {
        &mut self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    use crate::types::{ContractMatcher, ContractType, Slug};

    fn child(kind: &str, slug: &str) -> Contract {
        serde_json::from_value(json!({
            "type": kind,
            "slug": slug,
            "name": slug,
        }))
        .unwrap()
    }

    #[test]
    fn new_universe_has_universe_type() {
        let u = Universe::new();
        assert_eq!(u.get_type(), UNIVERSE);
    }

    #[test]
    fn default_is_empty_universe() {
        let u = Universe::default();
        assert_eq!(u.get_type(), UNIVERSE);
        assert!(u.get_children().is_empty());
    }

    #[test]
    fn add_child_through_deref_mut() {
        let mut u = Universe::new();
        u.add_child(child("sw.os", "debian"));

        let children = u.get_children();
        assert_eq!(children.len(), 1);
        assert_eq!(children[0].get_slug(), Some("debian"));
    }

    #[test]
    fn add_children_through_deref_mut() {
        let mut u = Universe::new();
        u.add_children(vec![
            child("sw.os", "debian"),
            child("sw.os", "fedora"),
            child("hw.device-type", "raspberry-pi"),
        ]);

        assert_eq!(u.get_children().len(), 3);
        assert_eq!(u.get_children_by_type("sw.os").len(), 2);
        assert_eq!(u.get_children_by_type("hw.device-type").len(), 1);
    }

    #[test]
    fn find_children_through_deref_mut() {
        let mut u = Universe::new();
        u.add_children(vec![
            child("sw.os", "debian"),
            child("sw.os", "fedora"),
        ]);

        let matcher = ContractMatcher::new(
            ContractType::new("sw.os"),
            Some(Slug::new("debian")),
            None,
            None,
        );
        let found = u.find_children(&matcher);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].get_slug(), Some("debian"));
    }

    #[test]
    fn remove_child_through_deref_mut() {
        let mut u = Universe::new();
        let c = child("sw.os", "debian");
        u.add_child(c.clone());
        assert_eq!(u.get_children().len(), 1);

        u.remove_child(&c);
        assert!(u.get_children().is_empty());
    }

    #[test]
    fn hash_is_stable_for_empty_universe() {
        let a = Universe::new();
        let b = Universe::new();
        assert_eq!(a.hash(), b.hash());
    }

    #[test]
    fn into_inner_preserves_contract() {
        let mut u = Universe::new();
        u.add_child(child("sw.os", "debian"));
        let contract = u.into_inner();
        assert_eq!(contract.get_type(), UNIVERSE);
        assert_eq!(contract.get_children().len(), 1);
    }
}
