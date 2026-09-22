#![doc = include_str!("../README.md")]

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
    Asset, InvalidIdentifier, InvalidSemver, Kind, Matcher, PartialContract, RawContract,
    Requirement, Slug, UNIVERSE, Version, VersionReq,
};
pub use universe::Universe;
