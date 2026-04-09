//! Contrato: a contract system for describing composable, versioned things
//! and their relationships.
//!
//! This crate provides the core data structures and logic for the contrato
//! contract system. Contracts represent versioned "things" (devices, OSes,
//! stacks, etc.) with typed relationships, requirements, and capabilities.

pub mod children_tree;
pub mod hash;
pub mod matcher_cache;
pub mod object_set;
pub mod template;
pub mod types;
pub mod variants;

pub use children_tree::{
    ChildrenIndex, ChildrenTree, PathConflictError, build as build_children_tree,
    get_all as get_all_children,
};
pub use hash::hash_object;
pub use matcher_cache::{Matcher, MatcherCache};
pub use object_set::{Identifiable, ObjectSet};
pub use template::compile_contract;
pub use types::{
    Asset, ContractCapability, ContractMatcher, ContractRequirement, ContractType, PartialContract,
    RawContract, Slug, UNIVERSE, Version, VersionReq,
};
pub use variants::build as build_variants;
