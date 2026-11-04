//! Runtime support for Rust bindings emitted by `convex-typegen`.
//!
//! Use this crate as a normal dependency and the matching generator release as a
//! build dependency. The runtime supports Rust 1.85.

mod client;
pub mod model;
pub mod value_serde;

pub use client::ConvexClientExt;
/// Serde derives in generated Rust resolve through this re-export.
pub use serde;
/// JSON utilities for applications working with JSON separately from the Convex codec.
pub use serde_json;

/// Imports for generated Rust and applications that call Convex.
pub mod prelude
{
    pub use serde::{Deserialize, Serialize};
    pub use serde_json::Value as ConvexJsonValue;

    pub use crate::ConvexClientExt;
    pub use crate::value_serde::{Bytes as ConvexBytes, Error as ConvexValueSerdeError, from_value, to_value};
}
