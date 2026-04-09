//! Contrato: a contract system for describing composable, versioned things
//! and their relationships.
//!
//! This crate provides the core data structures and logic for the contrato
//! contract system. Contracts represent versioned "things" (devices, OSes,
//! stacks, etc.) with typed relationships, requirements, and capabilities.

pub mod hash;
pub mod template;
pub mod types;
pub mod variants;

pub use hash::hash_object;
pub use template::compile_contract;
pub use types::{
    Asset, ContractCapability, ContractMatcher, ContractRequirement, ContractType, PartialContract,
    RawContract, Slug, UNIVERSE, Version, VersionReq,
};
pub use variants::build as build_variants;
