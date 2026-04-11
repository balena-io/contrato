//! Contrato: a contract system for describing composable, versioned things
//! and their relationships.
//!
//! This crate provides the core data structures and logic for the contrato
//! contract system. Contracts represent versioned "things" (devices, OSes,
//! stacks, etc.) with typed relationships, requirements, and capabilities.

mod hash;
mod index;
mod matcher;
mod matcher_cache;
mod object_set;
mod path;
mod template;
mod variants;

pub mod children_tree;
pub mod contract;
pub mod types;
pub mod universe;

pub use contract::Contract;
pub use types::{
    Asset, ContractCapability, ContractMatcher, ContractRequirement, ContractType, PartialContract,
    RawContract, Slug, UNIVERSE, Version, VersionReq,
};
pub use universe::Universe;
