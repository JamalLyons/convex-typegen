#![warn(missing_docs)]
//! End-to-end type safety for Convex in Rust.
//!
//! `convex-typegen-runtime` is the typed client layer on top of the official Convex SDK.
//! Generated function references, argument structs, table models, and `Id<T>` / document types
//! come from `convex-typegen` at build time, so mismatched tables, fields, arguments, and return
//! types are compile errors—not runtime surprises after deploy.
//!
//! Use this crate as a normal dependency and the matching generator release as a
//! build dependency. The runtime supports Rust 1.85.
//!
//! Follow the [first project](https://ctg.jamallyons.com/getting-started.html)
//! for setup, or see [typed calls and subscriptions](https://ctg.jamallyons.com/typed-client.html).
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

pub use client::{CallError, InvalidApplicationData, TransportError, TypedClient};
/// The Convex SDK used by this runtime.
pub use convex;
/// Derive the Convex wire representation for Rust models.
/// See [handwritten models](https://ctg.jamallyons.com/docs/macros.html#derive-a-convex-model).
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
/// See [including bindings](https://ctg.jamallyons.com/generation.html#include-bindings-from-cargo-output).
#[macro_export]
macro_rules! include {
    () => {
        $crate::include!("convex_api.rs");
    };
    ($file:literal $(,)?) => {
        ::core::include!(::core::concat!(::core::env!("OUT_DIR"), "/", $file));
    };
}
