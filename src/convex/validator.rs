//! Validator trees shared by input adapters and code generation.

use std::collections::BTreeMap;

use crate::convex::ir::FunctionKind;

#[derive(Debug, Clone)]
pub(crate) enum Literal
{
    String(String),
    Boolean(bool),
    Float64(f64),
    Int64(i64),
}

impl PartialEq for Literal
{
    fn eq(&self, other: &Self) -> bool
    {
        match (self, other) {
            (Self::String(a), Self::String(b)) => a == b,
            (Self::Boolean(a), Self::Boolean(b)) => a == b,
            (Self::Float64(a), Self::Float64(b)) => a.to_bits() == b.to_bits(),
            (Self::Int64(a), Self::Int64(b)) => a == b,
            _ => false,
        }
    }
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
    FunctionHandle(Box<FunctionSignature>),
}

/// Type metadata only. The handle itself is an opaque Convex string.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct FunctionSignature
{
    pub kind: Option<FunctionKind>,
    pub args: Option<ConvexValidator>,
    pub returns: Option<ConvexValidator>,
}

impl ConvexValidator
{
    pub fn is_record_key(&self) -> bool
    {
        match self {
            Self::String | Self::Id(Some(_)) | Self::Literal(Some(Literal::String(_))) => true,
            Self::Union(branches) => branches.iter().all(Self::is_record_key),
            _ => false,
        }
    }

    #[cfg(test)]
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
            Self::FunctionHandle(_) => "functionHandle",
        }
    }
}
