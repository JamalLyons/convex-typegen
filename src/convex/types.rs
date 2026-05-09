//! Intermediate representation between [`crate::convex::parser`] and [`crate::convex::codegen`].
//!
//! Table and function metadata are strongly typed; **column / parameter validators** stay as
//! [`serde_json::Value`] trees with a `"type"` string discriminator. That mirrors Convex’s nested
//! `v.*` calls without maintaining a parallel Rust enum that would need updating for every edge case.

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

/// Parsed `defineSchema` body: ordered list of tables.
///
/// A schema can contain many tables. https://docs.convex.dev/database/schemas
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct ConvexSchema
{
    pub tables: Vec<ConvexTable>,
}

/// A table in the convex schema.
///
/// A table can contain many columns.
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct ConvexTable
{
    /// The name of the table.
    pub name: String,
    /// The columns in the table.
    pub columns: Vec<ConvexColumn>,
}

/// A column in the convex schema.
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct ConvexColumn
{
    /// The name of the column.
    pub name: String,
    /// Normalized `v.*` subtree (see module docs).
    pub data_type: JsonValue,
}

/// A collection of all convex functions.
pub(crate) type ConvexFunctions = Vec<ConvexFunction>;

/// One exported `query`, `mutation`, or `action` binding.
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct ConvexFunction
{
    /// Rust-friendly export name (`export const foo = …`).
    pub name: String,
    pub params: Vec<ConvexFunctionParam>,
    /// Callee identifier from TS: `"query"`, `"mutation"`, or `"action"` (informational for now).
    pub type_: String,
    /// Convex module path segment: file stem without `.ts` (`api` for `convex/api.ts`).
    pub file_name: String,
}

/// A parameter in a convex function.
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct ConvexFunctionParam
{
    pub name: String,
    pub data_type: JsonValue,
}
