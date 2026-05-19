//! Convenient glob import for `build.rs` and for crates that call Convex with generated types.
//!
//! Serde derives in generated code resolve through [`crate::serde`] (see crate-level docs). The
//! [`ConvexJsonValue`] / [`ConvexJsonError`] aliases match the names referenced in emitted `TryFrom`
//! impls and type positions.

pub use serde::{Deserialize, Serialize};
pub use serde_json::{Error as ConvexJsonError, Value as ConvexJsonValue};

pub use crate::config::Configuration;
#[cfg(feature = "client")]
pub use crate::convex::{ConvexClientExt, ConvexValueExt, IntoConvexValue};
pub use crate::error::ConvexTypeGeneratorError;
pub use crate::fs::rcfp;
pub use crate::generate;
