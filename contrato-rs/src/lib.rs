//! Contrato: a contract system for describing composable, versioned things
//! and their relationships.
//!
//! This crate provides the core data structures and logic for the contrato
//! contract system. Contracts represent versioned "things" (devices, OSes,
//! stacks, etc.) with typed relationships, requirements, and capabilities.

mod children_tree;
mod contract;
mod error;
mod hash;
mod index;
mod matcher;
mod path;
mod template;
mod types;
mod universe;
mod variants;

pub use children_tree::ChildrenTree;
pub use contract::Contract;
pub use error::Error;
pub use types::{
    Asset, ContractMatcher, ContractRequirement, ContractType, InvalidIdentifier, PartialContract,
    RawContract, Slug, UNIVERSE, Version, VersionReq,
};
pub use universe::Universe;
