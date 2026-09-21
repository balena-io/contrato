//! Contrato: a contract system for describing composable, versioned things
//! and their relationships.
//!
//! A [`Contract`] describes a *thing* — a device, an OS, a library, a
//! feature — with a type, a slug, a version, the capabilities it provides
//! (`children`) and the capabilities it needs (`requires`). Contracts are
//! normally read from JSON.
//!
//! ```rust
//! use contrato::{Contract, Matcher};
//!
//! let os: Contract = serde_json::from_value(serde_json::json!({
//!     "type": "sw.os",
//!     "slug": "balenaos",
//!     "version": "6.1.2",
//!     "children": [
//!         { "type": "sw.library", "slug": "glibc", "version": "2.31" },
//!         { "type": "sw.feature", "slug": "secureboot" }
//!     ]
//! }))?;
//!
//! let app: Contract = serde_json::from_value(serde_json::json!({
//!     "type": "sw.application",
//!     "slug": "myapp",
//!     "requires": [{ "type": "sw.library", "slug": "glibc", "version": ">=2.17" }]
//! }))?;
//!
//! assert!(os.satisfies_child_contract(&app, None));
//!
//! let matcher = Matcher::new("sw.library").with_version(">=2");
//! assert_eq!(os.find_children(&matcher).len(), 1);
//! # Ok::<(), serde_json::Error>(())
//! ```

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
