//! Source-neutral model shared by input adapters and code generation.

use crate::convex::validator::ConvexValidator;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SourceSpan
{
    pub start: u32,
    pub end: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Provenance
{
    pub source: String,
    pub span: Option<SourceSpan>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Declared<T>
{
    Declared(T),
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FieldPresence
{
    Required,
    Optional,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Field
{
    pub name: String,
    pub validator: ConvexValidator,
    pub provenance: Provenance,
}

impl Field
{
    pub fn presence(&self) -> FieldPresence
    {
        if matches!(self.validator, ConvexValidator::Optional(_)) {
            FieldPresence::Optional
        } else {
            FieldPresence::Required
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TableDocument
{
    pub name: String,
    pub shape: TableShape,
    pub provenance: Provenance,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum TableShape
{
    Object(Vec<Field>),
    Union(Vec<Self>),
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Schema
{
    pub tables: Vec<TableDocument>,
    pub provenance: Provenance,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FunctionIdentity
{
    /// Root-relative module path. Explicit sources outside the configured root have no known path.
    pub module: Option<Vec<String>>,
    pub export: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FunctionKind
{
    Query,
    Mutation,
    Action,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Visibility
{
    Public,
    Internal,
    Unknown,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ReturnValidator
{
    pub validator: ConvexValidator,
    pub provenance: Provenance,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Function
{
    pub identity: FunctionIdentity,
    /// Compatibility name for the current emitter, which uses the basename and strips only `.ts`.
    pub legacy_module_name: String,
    pub kind: FunctionKind,
    pub visibility: Visibility,
    /// Unknown differs from a declared empty argument object.
    pub args: Declared<Vec<Field>>,
    /// Unknown differs from a declared `v.any()`.
    pub returns: Declared<ReturnValidator>,
    pub provenance: Provenance,
}

impl Function
{
    pub fn params(&self) -> &[Field]
    {
        match &self.args {
            Declared::Declared(fields) => fields,
            Declared::Unknown => &[],
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct GenerationIr
{
    pub schema: Schema,
    pub functions: Vec<Function>,
}
