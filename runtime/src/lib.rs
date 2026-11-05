//! Runtime support for Rust bindings emitted by `convex-typegen`.
//!
//! Use this crate as a normal dependency and the matching generator release as a
//! build dependency. The runtime supports Rust 1.85.

mod client;
pub mod function;
pub mod model;
pub mod value_serde;

pub use client::{CallError, InvalidApplicationData, TypedClient};
/// Derive the Convex wire representation for Rust models.
pub use convex_typegen_macros::ConvexValue;
/// Serde derives in generated Rust resolve through this re-export.
pub use serde;
/// JSON utilities for applications working with JSON separately from the Convex codec.
pub use serde_json;

/// Imports for generated Rust and applications that call Convex.
pub mod prelude
{
    pub use serde::{Deserialize, Serialize};
    pub use serde_json::Value as ConvexJsonValue;

    pub use crate::value_serde::{Bytes as ConvexBytes, Error as ConvexValueSerdeError, from_value, to_value};
    pub use crate::{CallError, TypedClient};
}

/// Include Rust bindings from this package's Cargo output directory.
///
/// Defaults to `convex_types.rs`. Pass a string literal relative to `OUT_DIR`
/// when the build script uses a custom output filename.
#[macro_export]
macro_rules! include {
    () => {
        $crate::include!("convex_types.rs");
    };
    ($file:literal $(,)?) => {
        ::core::include!(::core::concat!(::core::env!("OUT_DIR"), "/", $file));
    };
}
