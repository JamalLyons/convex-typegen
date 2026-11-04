//! Validator trees shared by input adapters and code generation.

use std::collections::BTreeMap;

/// Maximum nesting depth for generated object structs.
pub(crate) const MAX_OBJECT_NEST_DEPTH: usize = 8;

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Literal
{
    String(String),
    Boolean(bool),
    Float64(f64),
    Int64(i64),
    Null,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ConvexValidator
{
    String,
    Number,
    Boolean,
    Null,
    Int64,
    Bytes,
    Any,
    Id(Option<String>),
    Array(Box<Self>),
    Object
    {
        /// Name hint for the current Rust emitter.
        struct_name: Option<String>,
        properties: Option<BTreeMap<String, Self>>,
    },
    Record
    {
        key_type: Box<Self>,
        value_type: Box<Self>,
    },
    Optional(Box<Self>),
    Union(Vec<Self>),
    Literal(Option<Literal>),
    /// Missing child of an incomplete array or record validator.
    Unknown,
}

impl ConvexValidator
{
    pub fn type_name(&self) -> &'static str
    {
        match self {
            Self::String => "string",
            Self::Number => "number",
            Self::Boolean => "boolean",
            Self::Null => "null",
            Self::Int64 => "int64",
            Self::Bytes => "bytes",
            Self::Any => "any",
            Self::Id(_) => "id",
            Self::Array(_) => "array",
            Self::Object { .. } => "object",
            Self::Record { .. } => "record",
            Self::Optional(_) => "optional",
            Self::Union(_) => "union",
            Self::Literal(_) => "literal",
            Self::Unknown => "unknown",
        }
    }
}
