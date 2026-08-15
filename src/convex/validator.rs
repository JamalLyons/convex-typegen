//! Typed validator IR aligned with parser JSON and codegen.
//!
//! Parser output remains `serde_json::Value` for compatibility; this module provides
//! [`ConvexValidator`], structural hashing, and `structName` handling for named object codegen.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::error::ConvexTypeGeneratorError;

/// Maximum nesting depth for generated object structs; deeper shapes use `ConvexJsonValue`.
pub(crate) const MAX_OBJECT_NEST_DEPTH: usize = 8;

/// Normalized Convex `v.*` validator tree (mirrors parser JSON shape).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub(crate) enum ConvexValidator
{
    String,
    Number,
    Boolean,
    Null,
    Int64,
    Bytes,
    Any,
    Id,
    Array
    {
        #[serde(rename = "elements")]
        elements: Box<ConvexValidator>,
    },
    Object
    {
        #[serde(skip_serializing_if = "Option::is_none")]
        struct_name: Option<String>,
        properties: BTreeMap<String, ConvexValidator>,
    },
    Record
    {
        #[serde(rename = "keyType")]
        key_type: Box<ConvexValidator>,
        #[serde(rename = "valueType")]
        value_type: Box<ConvexValidator>,
    },
    Optional
    {
        inner: Box<ConvexValidator>,
    },
    Union
    {
        variants: Vec<ConvexValidator>,
    },
    Literal
    {
        value: JsonValue,
    },
    /// Fallback for unknown discriminators preserved from JSON.
    Unknown
    {
        raw: JsonValue,
    },
}

impl ConvexValidator
{
    /// Parse the parser's JSON validator into [`ConvexValidator`].
    pub(crate) fn from_json(value: &JsonValue) -> Result<Self, ConvexTypeGeneratorError>
    {
        let type_name = value["type"]
            .as_str()
            .ok_or_else(|| ConvexTypeGeneratorError::InvalidSchema {
                context: "validator".to_string(),
                details: "Missing type field".to_string(),
            })?;

        Ok(match type_name {
            "string" => Self::String,
            "number" => Self::Number,
            "boolean" => Self::Boolean,
            "null" => Self::Null,
            "int64" => Self::Int64,
            "bytes" => Self::Bytes,
            "any" => Self::Any,
            "id" => Self::Id,
            "array" => Self::Array {
                elements: Box::new(Self::from_json(&value["elements"])?),
            },
            "object" => {
                let struct_name = value.get("structName").and_then(|v| v.as_str()).map(str::to_string);
                let mut properties = BTreeMap::new();
                if let Some(props) = value["properties"].as_object() {
                    for (k, v) in props {
                        properties.insert(k.clone(), Self::from_json(v)?);
                    }
                }
                Self::Object { struct_name, properties }
            }
            "record" => Self::Record {
                key_type: Box::new(Self::from_json(&value["keyType"])?),
                value_type: Box::new(Self::from_json(&value["valueType"])?),
            },
            "optional" => Self::Optional {
                inner: Box::new(Self::from_json(&value["inner"])?),
            },
            "union" => {
                let variants = value["variants"]
                    .as_array()
                    .map(|arr| arr.iter().map(Self::from_json).collect::<Result<Vec<_>, _>>())
                    .transpose()?
                    .unwrap_or_default();
                Self::Union { variants }
            }
            "literal" => Self::Literal {
                value: value["value"].clone(),
            },
            other => Self::Unknown {
                raw: serde_json::json!({ "type": other }),
            },
        })
    }

    /// Structural fingerprint for deduplicating identical object shapes under one struct name.
    pub(crate) fn structural_key(&self) -> String
    {
        serde_json::to_string(self).unwrap_or_default()
    }

    /// All `struct_name` values on object nodes in this tree (depth-first).
    pub(crate) fn collect_struct_names(&self, out: &mut Vec<String>)
    {
        match self {
            Self::Object {
                struct_name: Some(name),
                properties,
                ..
            } => {
                out.push(name.clone());
                for v in properties.values() {
                    v.collect_struct_names(out);
                }
            }
            Self::Object { properties, .. } => {
                for v in properties.values() {
                    v.collect_struct_names(out);
                }
            }
            Self::Array { elements } => elements.collect_struct_names(out),
            Self::Optional { inner } => inner.collect_struct_names(out),
            Self::Record { key_type, value_type } => {
                key_type.collect_struct_names(out);
                value_type.collect_struct_names(out);
            }
            Self::Union { variants } => {
                for v in variants {
                    v.collect_struct_names(out);
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod validator_tests
{
    use serde_json::json;

    use super::ConvexValidator;

    #[test]
    fn object_struct_name_roundtrip()
    {
        let j = json!({
            "type": "object",
            "structName": "ProjectsSettings",
            "properties": {
                "theme": { "type": "string" },
                "notifyEmail": { "type": "boolean" }
            }
        });
        let v = ConvexValidator::from_json(&j).unwrap();
        assert!(matches!(
            v,
            ConvexValidator::Object {
                struct_name: Some(name),
                ..
            } if name == "ProjectsSettings"
        ));
    }

    #[test]
    fn structural_key_differs_for_different_shapes()
    {
        let a = ConvexValidator::from_json(&json!({
            "type": "object",
            "structName": "A",
            "properties": { "x": { "type": "string" } }
        }))
        .unwrap();
        let b = ConvexValidator::from_json(&json!({
            "type": "object",
            "structName": "B",
            "properties": { "x": { "type": "number" } }
        }))
        .unwrap();
        assert_ne!(a.structural_key(), b.structural_key());
    }
}
