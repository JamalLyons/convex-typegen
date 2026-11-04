//! Source-neutral model passed from input adapters to code generation.
//!
//! The Oxc adapter currently lowers ESTree into this model through `parser`; future adapters (for
//! example Convex function-spec) can produce the same nodes without changing the emitter.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::convex::types::{ConvexFunction, ConvexSchema};
use crate::convex::validator::ConvexValidator;
use crate::error::ConvexTypeGeneratorError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct SourceSpan
{
    pub start: u32,
    pub end: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Provenance
{
    pub source: String,
    /// ESTree/Oxc offsets are absent for adapters that do not expose source locations.
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
    pub presence: FieldPresence,
    pub provenance: Provenance,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TableDocument
{
    pub name: String,
    pub fields: Vec<Field>,
    pub provenance: Provenance,
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
    /// Canonical Convex module path, relative to the `convex/` source root.
    pub module: Vec<String>,
    pub export: String,
}

impl FunctionIdentity
{
    /// Keep legacy emitted names stable while the typed client naming change is handled separately.
    pub fn legacy_module_name(&self) -> &str
    {
        self.module.last().map(String::as_str).unwrap_or("")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FunctionKind
{
    Query,
    Mutation,
    Action,
    InternalQuery,
    InternalMutation,
    InternalAction,
    Unknown,
}

impl FunctionKind
{
    pub fn from_name(value: &str) -> Self
    {
        match value {
            "query" => Self::Query,
            "mutation" => Self::Mutation,
            "action" => Self::Action,
            "internalQuery" => Self::InternalQuery,
            "internalMutation" => Self::InternalMutation,
            "internalAction" => Self::InternalAction,
            _ => Self::Unknown,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Visibility
{
    Public,
    Internal,
    Unknown,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Parameter
{
    pub name: String,
    pub validator: ConvexValidator,
    pub presence: FieldPresence,
    pub provenance: Provenance,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Function
{
    pub identity: FunctionIdentity,
    pub kind: FunctionKind,
    pub visibility: Visibility,
    /// `Unknown` distinguishes absent/unresolved args metadata from a declared empty object.
    pub args: Declared<Vec<Parameter>>,
    /// `Unknown` distinguishes an undeclared return validator from a declared `v.any()`.
    pub returns: Declared<ConvexValidator>,
    pub provenance: Provenance,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct GenerationIr
{
    pub schema: Schema,
    pub functions: Vec<Function>,
}

impl GenerationIr
{
    /// Transitional adapter preserving the current emitter's exact naming and output behavior.
    /// Validator parsing is centralized here; the IR itself contains no ESTree or JSON nodes.
    pub fn from_legacy(
        schema: ConvexSchema,
        functions: Vec<ConvexFunction>,
        convex_root: &Path,
        schema_source: &Path,
    ) -> Result<Self, ConvexTypeGeneratorError>
    {
        let schema = Schema {
            tables: schema
                .tables
                .into_iter()
                .map(|table| {
                    let provenance = Provenance {
                        source: schema_source.to_string_lossy().into_owned(),
                        span: None,
                    };
                    let fields = table
                        .columns
                        .into_iter()
                        .map(|column| {
                            let validator = ConvexValidator::from_json(&column.data_type)?;
                            let presence = if matches!(validator, ConvexValidator::Optional { .. }) {
                                FieldPresence::Optional
                            } else {
                                FieldPresence::Required
                            };
                            Ok(Field {
                                name: column.name,
                                validator,
                                presence,
                                provenance: provenance.clone(),
                            })
                        })
                        .collect::<Result<Vec<_>, ConvexTypeGeneratorError>>()?;
                    Ok(TableDocument {
                        name: table.name,
                        fields,
                        provenance,
                    })
                })
                .collect::<Result<Vec<_>, ConvexTypeGeneratorError>>()?,
            provenance: Provenance {
                source: schema_source.to_string_lossy().into_owned(),
                span: None,
            },
        };

        let functions = functions
            .into_iter()
            .map(|function| {
                let path = Path::new(&function.source_path);
                let relative = path.strip_prefix(convex_root).unwrap_or(path);
                let mut module: Vec<String> = relative
                    .with_extension("")
                    .components()
                    .map(|part| part.as_os_str().to_string_lossy().into_owned())
                    .collect();
                if module.is_empty() {
                    module.push(function.file_name.clone());
                }
                let provenance = Provenance {
                    source: function.source_path.clone(),
                    span: function.span,
                };
                let params = function
                    .params
                    .into_iter()
                    .map(|param| {
                        let validator = ConvexValidator::from_json(&param.data_type)?;
                        let presence = if matches!(validator, ConvexValidator::Optional { .. }) {
                            FieldPresence::Optional
                        } else {
                            FieldPresence::Required
                        };
                        Ok(Parameter {
                            name: param.name,
                            validator,
                            presence,
                            provenance: provenance.clone(),
                        })
                    })
                    .collect::<Result<Vec<_>, ConvexTypeGeneratorError>>()?;
                let kind = FunctionKind::from_name(&function.type_);
                let visibility = match kind {
                    FunctionKind::InternalQuery | FunctionKind::InternalMutation | FunctionKind::InternalAction => {
                        Visibility::Internal
                    }
                    FunctionKind::Unknown => Visibility::Unknown,
                    _ => Visibility::Public,
                };
                Ok(Function {
                    identity: FunctionIdentity {
                        module,
                        export: function.name,
                    },
                    kind,
                    visibility,
                    args: if function.args_declared {
                        Declared::Declared(params)
                    } else {
                        Declared::Unknown
                    },
                    returns: Declared::Unknown,
                    provenance,
                })
            })
            .collect::<Result<Vec<_>, ConvexTypeGeneratorError>>()?;

        Ok(Self { schema, functions })
    }

    /// Flatten to the current emitter input. Kept in one place as the transition boundary.
    pub fn into_emitter_input(self) -> Result<(ConvexSchema, Vec<ConvexFunction>), ConvexTypeGeneratorError>
    {
        use crate::convex::types::{ConvexColumn, ConvexFunction, ConvexFunctionParam, ConvexSchema, ConvexTable};
        let tables = self
            .schema
            .tables
            .into_iter()
            .map(|table| {
                let columns = table
                    .fields
                    .into_iter()
                    .map(|field| {
                        Ok(ConvexColumn {
                            name: field.name,
                            data_type: serde_json::to_value(field.validator)?,
                        })
                    })
                    .collect::<Result<Vec<_>, serde_json::Error>>()?;
                Ok(ConvexTable {
                    name: table.name,
                    columns,
                })
            })
            .collect::<Result<Vec<_>, serde_json::Error>>()?;
        let functions = self
            .functions
            .into_iter()
            .map(|function| {
                let file_name = function.identity.legacy_module_name().to_string();
                let args_declared = matches!(function.args, Declared::Declared(_));
                let params = match function.args {
                    Declared::Declared(params) => params
                        .into_iter()
                        .map(|param| {
                            Ok(ConvexFunctionParam {
                                name: param.name,
                                data_type: serde_json::to_value(param.validator)?,
                            })
                        })
                        .collect::<Result<Vec<_>, serde_json::Error>>()?,
                    Declared::Unknown => Vec::new(),
                };
                Ok(ConvexFunction {
                    name: function.identity.export,
                    params,
                    args_declared,
                    type_: match function.kind {
                        FunctionKind::Query => "query",
                        FunctionKind::Mutation => "mutation",
                        FunctionKind::Action => "action",
                        FunctionKind::InternalQuery => "internalQuery",
                        FunctionKind::InternalMutation => "internalMutation",
                        FunctionKind::InternalAction => "internalAction",
                        FunctionKind::Unknown => "unknown",
                    }
                    .into(),
                    file_name,
                    source_path: function.provenance.source,
                    span: function.provenance.span,
                })
            })
            .collect::<Result<Vec<_>, serde_json::Error>>()?;
        Ok((ConvexSchema { tables }, functions))
    }
}

#[cfg(test)]
mod tests
{
    use std::path::Path;

    use serde_json::json;

    use super::{Declared, FunctionKind, GenerationIr, Visibility};
    use crate::convex::types::{ConvexFunction, ConvexFunctionParam, ConvexSchema};

    #[test]
    fn lowering_keeps_nested_identity_kind_provenance_and_unknown_returns()
    {
        let function = ConvexFunction {
            name: "list".into(),
            params: vec![ConvexFunctionParam {
                name: "limit".into(),
                data_type: json!({"type":"number"}),
            }],
            args_declared: true,
            type_: "internalQuery".into(),
            file_name: "users".into(),
            source_path: "/workspace/convex/admin/users.ts".into(),
            span: Some(super::SourceSpan { start: 90, end: 170 }),
        };
        let ir = GenerationIr::from_legacy(
            ConvexSchema { tables: Vec::new() },
            vec![function],
            Path::new("/workspace/convex"),
            Path::new("/workspace/convex/schema.ts"),
        )
        .unwrap();
        let function = &ir.functions[0];
        assert_eq!(function.identity.module, vec!["admin".to_string(), "users".to_string()]);
        assert_eq!(function.identity.export, "list");
        assert_eq!(function.kind, FunctionKind::InternalQuery);
        assert_eq!(function.visibility, Visibility::Internal);
        assert!(matches!(function.args, Declared::Declared(_)));
        assert!(matches!(function.returns, Declared::Unknown));
        assert_eq!(function.provenance.span, Some(super::SourceSpan { start: 90, end: 170 }));

        let no_args_metadata = ConvexFunction {
            name: "health".into(),
            params: Vec::new(),
            args_declared: false,
            type_: "query".into(),
            file_name: "health".into(),
            source_path: "/workspace/convex/health.ts".into(),
            span: None,
        };
        let ir = GenerationIr::from_legacy(
            ConvexSchema { tables: Vec::new() },
            vec![no_args_metadata],
            Path::new("/workspace/convex"),
            Path::new("/workspace/convex/schema.ts"),
        )
        .unwrap();
        assert!(matches!(ir.functions[0].args, Declared::Unknown));
    }
}
