//! Runtime helpers for generated args and the official [`convex`] crate.

use std::collections::BTreeMap;

use convex::Value as ConvexValue;
use serde::Serialize;

use super::value_serde::{self, Error as ConvexValueSerdeError};

/// Blanket helpers on [`convex::ConvexClient`] for generated argument structs.
pub trait ConvexClientExt
{
    /// Serialize generated function arguments directly into Convex values.
    ///
    /// Generated optional fields are omitted according to their Serde field attributes. Values
    /// that cannot be represented by Convex, such as unsigned integers above `i64::MAX`, return a
    /// path-aware [`ConvexValueSerdeError`].
    fn prepare_args<T: Serialize>(args: T) -> Result<BTreeMap<String, ConvexValue>, ConvexValueSerdeError>
    {
        match value_serde::to_value(&args)? {
            ConvexValue::Object(map) => Ok(map),
            _ => Err(ConvexValueSerdeError::expected_object()),
        }
    }
}

impl ConvexClientExt for convex::ConvexClient {}

#[cfg(test)]
mod tests
{
    use serde::Serialize;

    use super::ConvexClientExt;
    use crate::value_serde::Bytes;

    #[derive(Serialize)]
    struct Args
    {
        integer: i64,
        float: f64,
        #[serde(skip_serializing_if = "Option::is_none")]
        omitted: Option<bool>,
        bytes: Bytes,
    }

    #[test]
    fn prepare_args_preserves_wire_kinds_and_optional_omission()
    {
        let args = Args {
            integer: 9,
            float: 2.0,
            omitted: None,
            bytes: Bytes(vec![1, 2, 3]),
        };
        let map = <convex::ConvexClient as ConvexClientExt>::prepare_args(args).unwrap();

        assert!(matches!(map.get("integer"), Some(convex::Value::Int64(9))));
        assert!(matches!(map.get("float"), Some(convex::Value::Float64(2.0))));
        assert!(matches!(map.get("bytes"), Some(convex::Value::Bytes(bytes)) if bytes == &vec![1, 2, 3]));
        assert!(!map.contains_key("omitted"));
    }

    #[test]
    fn prepare_args_rejects_non_object_root()
    {
        let error = <convex::ConvexClient as ConvexClientExt>::prepare_args("not args").unwrap_err();
        assert_eq!(error.path(), "$");
    }
}
