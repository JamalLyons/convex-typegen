#![warn(missing_docs)]
//! Runtime support for Rust bindings emitted by `convex-typegen`.
//!
//! Use this crate as a normal dependency and the matching generator release as a
//! build dependency. The runtime supports Rust 1.85.
//!
//! Follow the [first project](https://jamallyons.github.io/convex-typegen/getting-started.html)
//! for setup, or see [typed calls and subscriptions](https://jamallyons.github.io/convex-typegen/typed-client.html).
//!
//! # Typed calls
//!
//! Generated references connect the official SDK to their generated argument and return types.
//! Keep the SDK client alive for the adapter and any subscription streams.
//!
//! ```no_run
//! use convex_typegen_runtime::TypedClient;
//!
//! fn adapter<'a>(client: &'a mut convex::ConvexClient) -> TypedClient<'a>
//! {
//!     TypedClient::new(client)
//! }
//! ```
//!
//! For direct value conversion, use [`value_serde::to_value`] and
//! [`value_serde::from_value`]. These operate on the official SDK's `convex::Value` without a
//! JSON round trip.

mod client;
pub mod function;
pub mod model;
pub mod value_serde;

pub use client::{CallError, InvalidApplicationData, TypedClient};
/// Derive the Convex wire representation for Rust models.
/// See [handwritten models](https://jamallyons.github.io/convex-typegen/docs/macros.html#derive-a-convex-model).
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
/// Defaults to `convex_api.rs`. Pass a string literal relative to `OUT_DIR`
/// when the build script uses a custom output filename.
/// See [including bindings](https://jamallyons.github.io/convex-typegen/generation.html#include-bindings-from-cargo-output).
#[macro_export]
macro_rules! include {
    () => {
        $crate::include!("convex_api.rs");
    };
    ($file:literal $(,)?) => {
        ::core::include!(::core::concat!(::core::env!("OUT_DIR"), "/", $file));
    };
}
