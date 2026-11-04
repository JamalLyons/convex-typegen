//! Convenient glob import for `build.rs` and for crates that call Convex with generated types.
//!
//! Serde derives in generated code resolve through [`crate::serde`] (see crate-level docs).

pub use serde::{Deserialize, Serialize};
pub use serde_json::Value as ConvexJsonValue;

pub use crate::config::Configuration;
#[cfg(feature = "client")]
pub use crate::convex::ConvexClientExt;
#[cfg(feature = "client")]
pub use crate::convex::value_serde::{Bytes as ConvexBytes, Error as ConvexValueSerdeError, from_value, to_value};
pub use crate::error::ConvexTypeGeneratorError;
pub use crate::fs::rcfp;
pub use crate::generate;
