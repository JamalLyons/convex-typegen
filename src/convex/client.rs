//! Runtime helpers bridging generated args and the official [`convex`] crate.

use std::collections::BTreeMap;

use convex::Value as ConvexValue;
use serde_json::Value as JsonValue;

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
