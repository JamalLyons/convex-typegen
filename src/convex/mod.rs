//! Oxc → ESTree JSON → ad-hoc JSON traversal → Rust text.
//!
//! This module is intentionally private: the stable surface is [`crate::generate`], [`crate::config`],
//! and [`crate::prelude`]. What lives here:
//!
//! - **Lexer** ([`lexer`]) — parse TS to a [`serde_json::Value`] ESTree tree.
//! - **Parser** ([`parser`]) — interpret Convex `defineSchema` / `defineTable` / `v.*` and exported
//!   `query`/`mutation`/`action` shapes into [`types`].
//! - **Codegen** ([`codegen`]) — stringify that model as `src/convex_types.rs`-style output.
//!
//! ## JSON at the boundaries
//!
//! Column and argument types stay as [`serde_json::Value`] in [`types`] instead of a dedicated Rust
//! ADT: the parser’s output mirrors the nested `v.union`, `v.optional`, etc. tree closely, which keeps
//! codegen’s pattern matching aligned with how Convex encodes validators in TS.
//!
//! ## Client helpers
//!
//! [`IntoConvexValue`] and [`ConvexValueExt`] bridge `serde_json` and the official Convex client
//! crate’s `Value` type for callers who build args from JSON or inspect query results.
//! [`ConvexClientExt::prepare_args`] turns a generated args struct into the `BTreeMap<String, Value>`
//! the Convex client expects, using `TryFrom` so non-JSON-safe values (e.g. non-finite floats)
//! fail at prepare time.

use std::collections::BTreeMap;
use std::path::PathBuf;

use convex::Value as ConvexValue;
use serde_json::Value as JsonValue;

use crate::convex::lexer::generate_javascript_ast;
use crate::error::ConvexTypeGeneratorError;

pub(crate) mod codegen;
pub(crate) mod lexer;
pub(crate) mod parser;
pub(crate) mod types;
pub(crate) mod utils;

/// Infallible conversion from JSON-shaped args into [`ConvexValue`] (typically after serde / `TryFrom`).
pub trait IntoConvexValue
{
    /// Convert the type into a Convex Value
    fn into_convex_value(self) -> ConvexValue;
}

impl IntoConvexValue for JsonValue
{
    fn into_convex_value(self) -> ConvexValue
    {
        match self {
            JsonValue::Null => ConvexValue::Null,
            JsonValue::Bool(b) => ConvexValue::Boolean(b),
            JsonValue::Number(n) => {
                if let Some(i) = n.as_i64() {
                    ConvexValue::Int64(i)
                } else if let Some(f) = n.as_f64() {
                    ConvexValue::Float64(f)
                } else {
                    ConvexValue::Null
                }
            }
            JsonValue::String(s) => ConvexValue::String(s),
            JsonValue::Array(arr) => ConvexValue::Array(arr.into_iter().map(|v| v.into_convex_value()).collect()),
            JsonValue::Object(map) => {
                let converted: BTreeMap<String, ConvexValue> =
                    map.into_iter().map(|(k, v)| (k, v.into_convex_value())).collect();
                ConvexValue::Object(converted)
            }
        }
    }
}

/// Lossy-ish mapping from Convex wire values back to JSON ([`ConvexValue::Bytes`] becomes a JSON array of numbers).
pub trait ConvexValueExt
{
    /// Map a Convex runtime value into [`serde_json::Value`].
    fn into_serde_value(self) -> JsonValue;
}

impl ConvexValueExt for ConvexValue
{
    fn into_serde_value(self) -> JsonValue
    {
        match self {
            ConvexValue::Null => JsonValue::Null,
            ConvexValue::Boolean(b) => JsonValue::Bool(b),
            ConvexValue::Int64(i) => JsonValue::Number(i.into()),
            ConvexValue::Float64(f) => {
                if let Some(n) = serde_json::Number::from_f64(f) {
                    JsonValue::Number(n)
                } else {
                    JsonValue::Null
                }
            }
            ConvexValue::String(s) => JsonValue::String(s),
            ConvexValue::Array(arr) => JsonValue::Array(arr.into_iter().map(|v| v.into_serde_value()).collect()),
            ConvexValue::Object(map) => JsonValue::Object(map.into_iter().map(|(k, v)| (k, v.into_serde_value())).collect()),
            ConvexValue::Bytes(b) => JsonValue::Array(b.into_iter().map(|byte| JsonValue::Number(byte.into())).collect()),
        }
    }
}

/// Blanket helpers on [`convex::ConvexClient`] for generated argument structs.
pub trait ConvexClientExt
{
    /// Convert function arguments into Convex-compatible format.
    ///
    /// Uses [`TryFrom`] on the generated args type; serialization errors
    /// (for example non-finite floats) are returned as [`serde_json::Error`].
    fn prepare_args<T>(args: T) -> Result<BTreeMap<String, ConvexValue>, serde_json::Error>
    where
        BTreeMap<String, JsonValue>: TryFrom<T, Error = serde_json::Error>,
    {
        let map = BTreeMap::try_from(args)?;
        Ok(map.into_iter().map(|(k, v)| (k, v.into_convex_value())).collect())
    }
}

impl ConvexClientExt for convex::ConvexClient {}

/// Parse `schema.ts` through Oxc; returns the ESTree program as JSON ([`lexer::generate_javascript_ast`]).
pub(crate) fn create_schema_ast(path: PathBuf) -> Result<JsonValue, ConvexTypeGeneratorError>
{
    // Validate path exists before processing
    if !path.exists() {
        return Err(ConvexTypeGeneratorError::MissingSchemaFile);
    }

    generate_javascript_ast(&path)
}

/// Parse every function source path; keys are **canonical** path strings (UTF-8 lossy).
///
/// [`BTreeMap`] keeps iteration order deterministic for stable codegen. Basenames alone would collide
/// for `convex/a/foo.ts` vs `convex/b/foo.ts`; full canonical keys keep ASTs separate while
/// [`parser::parse_function_ast`] still derives the Convex module segment from the file name.
pub(crate) fn create_function_asts(paths: Vec<PathBuf>) -> Result<BTreeMap<String, JsonValue>, ConvexTypeGeneratorError>
{
    let mut function_asts = BTreeMap::new();

    for path in paths {
        let path_str = path.to_string_lossy().to_string();
        let canonical = path.canonicalize().map_err(|error| ConvexTypeGeneratorError::IOError {
            file: path_str.clone(),
            error,
        })?;

        let key = canonical.to_string_lossy().to_string();
        canonical
            .file_name()
            .ok_or_else(|| ConvexTypeGeneratorError::InvalidPath(key.clone()))?
            .to_str()
            .ok_or_else(|| ConvexTypeGeneratorError::InvalidUnicode(key.clone()))?;

        let function_ast = generate_javascript_ast(&canonical)?;
        function_asts.insert(key, function_ast);
    }

    Ok(function_asts)
}
