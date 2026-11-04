//! Walk ESTree JSON from [`crate::convex::lexer`] into [`crate::convex::ir`].
//!
//! ## Schema
//!
//! Expects `export default defineSchema({ ... })` (or an equivalent top-level `defineSchema` call).
//! Table bodies are `defineTable({ ... })` optionally followed by Convex builder chains such as
//! `.index(...)`, `.searchIndex(...)`, or `.vectorIndex(...)`. We peel those calls to reach the
//! inner `defineTable` and read column validators from its first argument.
//!
//! ## Functions
//!
//! Looks for `export const name = query({ args: { ... } })` (also `mutation` / `action`). Argument
//! shapes reuse the same recursive extractor as schema columns so validator composition stays
//! consistent.
use std::collections::BTreeMap;
use std::path::Path;

use serde_json::Value as JsonValue;

use crate::convex::ir::{
    Declared, Field, Function, FunctionIdentity, FunctionKind, Provenance, ReturnValidator, Schema, SourceSpan,
    TableDocument, Visibility,
};
use crate::convex::utils::{
    function_args_struct_stem, nested_object_struct_name, object_depth_allows_struct, schema_column_object_struct_name,
};
use crate::convex::validator::{ConvexValidator as Validator, Literal};
use crate::error::ConvexTypeGeneratorError;

/// Convex `v.*` validator names we know how to map (must stay in sync with schema reference docs).
pub(crate) const VALID_CONVEX_TYPES: &[&str] = &[
    "id", "null", "int64", "number", "boolean", "string", "bytes", "array", "object", "record", "union", "literal",
    "optional", "any",
];

/// `MemberExpression.property` may be an `Identifier` (`obj.index`) or a `Literal` (`obj["index"]`).
fn member_property_name(property: &JsonValue) -> Option<&str>
{
    match property.get("type").and_then(|t| t.as_str()) {
        Some("Identifier") => property.get("name").and_then(|n| n.as_str()),
        Some("Literal") => property.get("value").and_then(|v| v.as_str()),
        _ => None,
    }
}

/// Unwraps `defineTable({ ... }).index(...).searchIndex(...)` (etc.) to the inner `defineTable` call.
fn peel_to_define_table_call<'a>(expr: &'a JsonValue, table_name: &str) -> Result<&'a JsonValue, ConvexTypeGeneratorError>
{
    let ctx = format!("table_{table_name}");
    let mut cur = expr;

    loop {
        if cur.get("type").and_then(|t| t.as_str()) != Some("CallExpression") {
            return Err(ConvexTypeGeneratorError::InvalidSchema {
                context: ctx,
                details: "Table value must be a CallExpression (defineTable or chained .index / …)".to_string(),
            });
        }

        let callee = &cur["callee"];

        if callee.get("type").and_then(|t| t.as_str()) == Some("Identifier")
            && callee.get("name").and_then(|n| n.as_str()) == Some("defineTable")
        {
            return Ok(cur);
        }

        if callee.get("type").and_then(|t| t.as_str()) == Some("MemberExpression") {
            let prop = member_property_name(&callee["property"]);
            let known = matches!(prop, Some("index") | Some("searchIndex") | Some("vectorIndex"));
            if known {
                cur = &callee["object"];
                continue;
            }
        }

        return Err(ConvexTypeGeneratorError::InvalidSchema {
            context: ctx,
            details: "Unsupported table expression: expected defineTable(...) or a chain of .index / .searchIndex / \
                      .vectorIndex after defineTable"
                .to_string(),
        });
    }
}

/// Extract tables and columns from a schema program JSON value.
pub(crate) fn parse_schema_ast(ast: JsonValue, source: &Path) -> Result<Schema, ConvexTypeGeneratorError>
{
    let source = source.to_string_lossy();
    let context = "root";
    // Get the body array
    let body = ast["body"]
        .as_array()
        .ok_or_else(|| ConvexTypeGeneratorError::InvalidSchema {
            context: context.to_string(),
            details: "Missing body array".to_string(),
        })?;

    // Find the defineSchema call
    let define_schema = find_define_schema(body).ok_or_else(|| ConvexTypeGeneratorError::InvalidSchema {
        context: context.to_string(),
        details: "Could not find defineSchema call".to_string(),
    })?;

    // Get the arguments array of defineSchema
    let schema_args = define_schema["arguments"]
        .as_array()
        .ok_or_else(|| ConvexTypeGeneratorError::InvalidSchema {
            context: context.to_string(),
            details: "Missing schema arguments".to_string(),
        })?;

    // Get the first argument which is an object containing table definitions
    let tables_obj = schema_args
        .first()
        .and_then(|arg| arg["properties"].as_array())
        .ok_or_else(|| ConvexTypeGeneratorError::InvalidSchema {
            context: context.to_string(),
            details: "Missing table definitions".to_string(),
        })?;

    let mut tables = Vec::new();

    // Iterate through each table definition
    for table_prop in tables_obj {
        // Get the table name
        let table_name = table_prop["key"]["name"]
            .as_str()
            .ok_or_else(|| ConvexTypeGeneratorError::InvalidSchema {
                context: context.to_string(),
                details: "Invalid table name".to_string(),
            })?;

        let table_expr = peel_to_define_table_call(&table_prop["value"], table_name)?;

        // Get the defineTable call arguments
        let define_table_args =
            table_expr["arguments"]
                .as_array()
                .ok_or_else(|| ConvexTypeGeneratorError::InvalidSchema {
                    context: format!("table_{table_name}"),
                    details: "Invalid table definition (defineTable has no arguments array)".to_string(),
                })?;

        // Get the first argument which contains column definitions
        let columns_obj = define_table_args
            .first()
            .and_then(|arg| arg["properties"].as_array())
            .ok_or_else(|| ConvexTypeGeneratorError::InvalidSchema {
                context: format!("table_{table_name}"),
                details: "Missing column definitions".to_string(),
            })?;

        let mut columns = Vec::new();

        // Iterate through each column definition
        for column_prop in columns_obj {
            // Get column name
            let column_name =
                column_prop["key"]["name"]
                    .as_str()
                    .ok_or_else(|| ConvexTypeGeneratorError::InvalidSchema {
                        context: context.to_string(),
                        details: "Invalid column name".to_string(),
                    })?;

            // Get column type by looking at the property chain
            let mut context = TypeContext::for_schema_column(table_name, column_name);
            let validator = parse_validator(&column_prop["value"], &mut context)?;

            columns.push(Field {
                name: column_name.to_string(),
                validator,
                provenance: provenance(&source, column_prop),
            });
        }

        tables.push(TableDocument {
            name: table_name.to_string(),
            fields: columns,
            provenance: provenance(&source, table_prop),
        });
    }

    Ok(Schema {
        tables,
        provenance: provenance(&source, define_schema),
    })
}

/// Helper function to find the defineSchema call in the AST
fn find_define_schema(body: &[JsonValue]) -> Option<&JsonValue>
{
    for node in body {
        // Check if this is an export default declaration
        if let Some(declaration) = node.get("declaration") {
            // Check if this is a call expression
            if declaration["type"].as_str() == Some("CallExpression") {
                // Check if the callee is defineSchema
                if let Some(callee) = declaration.get("callee")
                    && callee["type"].as_str() == Some("Identifier")
                    && callee["name"].as_str() == Some("defineSchema")
                {
                    return Some(declaration);
                }
            }
        }

        // Could also be a regular variable declaration or expression
        // that calls defineSchema
        if node["type"].as_str() == Some("CallExpression")
            && let Some(callee) = node.get("callee")
            && callee["type"].as_str() == Some("Identifier")
            && callee["name"].as_str() == Some("defineSchema")
        {
            return Some(node);
        }
    }
    None
}

fn provenance(source: &str, node: &JsonValue) -> Provenance
{
    let offset = |key: &str| node[key].as_u64().and_then(|offset| u32::try_from(offset).ok());
    let span = offset("start")
        .zip(offset("end"))
        .map(|(start, end)| SourceSpan { start, end });
    Provenance {
        source: source.to_string(),
        span,
    }
}

/// Interpret an ESTree validator expression directly as a typed validator.
fn parse_validator(value: &JsonValue, context: &mut TypeContext) -> Result<Validator, ConvexTypeGeneratorError>
{
    let type_name = value["callee"]["property"]["name"]
        .as_str()
        .ok_or_else(|| ConvexTypeGeneratorError::InvalidSchema {
            context: context.get_error_context(),
            details: "Invalid column type".to_string(),
        })?;
    let args = value["arguments"].as_array().map_or(&[][..], Vec::as_slice);

    Ok(match type_name {
        "string" => Validator::String,
        "number" => Validator::Number,
        "boolean" => Validator::Boolean,
        "null" => Validator::Null,
        "int64" => Validator::Int64,
        "bytes" => Validator::Bytes,
        "any" => Validator::Any,
        "id" => Validator::Id(args.first().and_then(|node| node["value"].as_str()).map(str::to_string)),
        "literal" => Validator::Literal(args.first().and_then(parse_literal)),
        "optional" => {
            let inner = args.first().ok_or_else(|| ConvexTypeGeneratorError::InvalidSchema {
                context: context.type_path.join("."),
                details: "Optional type must have an inner type".to_string(),
            })?;
            context.type_path.push("inner".to_string());
            let inner = parse_validator(inner, context)?;
            context.type_path.pop();
            Validator::Optional(Box::new(inner))
        }
        "array" => {
            context.type_path.push("elements".to_string());
            let elements = args.first().map(|arg| parse_validator(arg, context)).transpose()?;
            context.type_path.pop();
            Validator::Array(Box::new(elements.unwrap_or(Validator::Unknown)))
        }
        "object" => {
            context.object_depth += 1;
            let struct_name = context.current_object_struct_name();
            if let Some(name) = &struct_name {
                context.struct_name_stack.push(name.clone());
            }

            let properties = if let Some(props) = args.first().and_then(|arg| arg["properties"].as_array()) {
                let mut properties = BTreeMap::new();
                for prop in props {
                    let name = prop["key"]["name"]
                        .as_str()
                        .ok_or_else(|| ConvexTypeGeneratorError::InvalidSchema {
                            context: context.type_path.join("."),
                            details: "Invalid object property name".to_string(),
                        })?;
                    context.type_path.push(name.to_string());
                    let validator = parse_validator(&prop["value"], context)?;
                    context.type_path.pop();
                    properties.insert(name.to_string(), validator);
                }
                Some(properties)
            } else {
                None
            };

            if struct_name.is_some() {
                context.struct_name_stack.pop();
            }
            context.object_depth -= 1;
            Validator::Object {
                struct_name: struct_name.filter(|_| properties.as_ref().is_some_and(|props| !props.is_empty())),
                properties,
            }
        }
        "record" => {
            let (key_type, value_type) = if let [key, value, ..] = args {
                (parse_validator(key, context)?, parse_validator(value, context)?)
            } else {
                (Validator::Unknown, Validator::Unknown)
            };
            Validator::Record {
                key_type: Box::new(key_type),
                value_type: Box::new(value_type),
            }
        }
        "union" => Validator::Union(
            args.iter()
                .map(|arg| parse_validator(arg, context))
                .collect::<Result<_, _>>()?,
        ),
        _ => {
            return Err(ConvexTypeGeneratorError::InvalidType {
                found: type_name.to_string(),
                valid_types: VALID_CONVEX_TYPES.iter().map(|&name| name.to_string()).collect(),
            });
        }
    })
}

fn parse_literal(node: &JsonValue) -> Option<Literal>
{
    let negative = node["type"] == "UnaryExpression" && node["operator"] == "-";
    let node = if negative { &node["argument"] } else { node };
    if node["type"] != "Literal" || node.get("regex").is_some() {
        return None;
    }
    if let Some(bigint) = node["bigint"].as_str() {
        let bigint = if negative { format!("-{bigint}") } else { bigint.to_string() };
        return bigint.parse().ok().map(Literal::Int64);
    }
    match node.get("value")? {
        JsonValue::Number(value) => value
            .as_f64()
            .map(|value| Literal::Float64(if negative { -value } else { value })),
        JsonValue::String(value) if !negative => Some(Literal::String(value.clone())),
        JsonValue::Bool(value) if !negative => Some(Literal::Boolean(*value)),
        JsonValue::Null if !negative => Some(Literal::Null),
        _ => None,
    }
}

/// How to name generated structs for `v.object` at the root of a walk.
#[derive(Debug, Clone)]
enum ObjectNaming
{
    SchemaColumn
    {
        table: String, column: String
    },
    FunctionParam
    {
        module: String, export: String, param: String
    },
}

/// Parser-local state for validator context and generated object names.
#[derive(Debug)]
struct TypeContext
{
    /// Prefix for [`crate::error::ConvexTypeGeneratorError::InvalidSchema`] messages.
    file_name: String,
    /// Dot-separated path inside the validator tree (`inner`, `elements`, field names, …).
    type_path: Vec<String>,
    naming: Option<ObjectNaming>,
    object_depth: usize,
    /// Struct names of enclosing `v.object` nodes (for nested object naming).
    struct_name_stack: Vec<String>,
}

impl TypeContext
{
    fn for_schema_column(table: &str, column: &str) -> Self
    {
        Self {
            file_name: format!("table_{table}.{column}"),
            type_path: Vec::new(),
            naming: Some(ObjectNaming::SchemaColumn {
                table: table.to_string(),
                column: column.to_string(),
            }),
            object_depth: 0,
            struct_name_stack: Vec::new(),
        }
    }

    fn for_function_param(module: &str, export: &str, param: &str) -> Self
    {
        Self {
            file_name: format!("function_{module}.{export}.{param}"),
            type_path: Vec::new(),
            naming: Some(ObjectNaming::FunctionParam {
                module: module.to_string(),
                export: export.to_string(),
                param: param.to_string(),
            }),
            object_depth: 0,
            struct_name_stack: Vec::new(),
        }
    }

    fn current_object_struct_name(&self) -> Option<String>
    {
        if !object_depth_allows_struct(self.object_depth) {
            return None;
        }
        if self.type_path.last().is_some_and(|s| s == "elements") && self.struct_name_stack.is_empty() {
            if let Some(ObjectNaming::SchemaColumn { table, column }) = &self.naming {
                return Some(format!("{}Element", schema_column_object_struct_name(table, column)));
            }
            if let Some(ObjectNaming::FunctionParam { module, export, param }) = &self.naming {
                let stem = function_args_struct_stem(module, export);
                return Some(format!(
                    "{stem}{}Element",
                    crate::convex::utils::capitalize_first_letter(param)
                ));
            }
        }
        if let Some(parent) = self.struct_name_stack.last() {
            let field = self.type_path.last()?;
            return Some(nested_object_struct_name(parent, field));
        }
        match &self.naming {
            Some(ObjectNaming::SchemaColumn { table, column }) => Some(schema_column_object_struct_name(table, column)),
            Some(ObjectNaming::FunctionParam { module, export, param }) => {
                let stem = function_args_struct_stem(module, export);
                Some(format!("{stem}{}", crate::convex::utils::capitalize_first_letter(param)))
            }
            None => None,
        }
    }

    /// Get the current context for error messages.
    fn get_error_context(&self) -> String
    {
        format!("{}:{}", self.file_name, self.type_path.join("."))
    }
}

/// Walk every module’s ESTree JSON and collect exported Convex endpoints.
pub(crate) fn parse_function_ast(
    ast_map: BTreeMap<String, JsonValue>,
    convex_root: Option<&Path>,
) -> Result<Vec<Function>, ConvexTypeGeneratorError>
{
    let mut convex_functions = Vec::new();

    for (source_path, ast) in ast_map {
        let path = Path::new(&source_path);
        let file_name = path
            .file_name()
            .ok_or_else(|| ConvexTypeGeneratorError::InvalidPath(source_path.clone()))?
            .to_str()
            .ok_or_else(|| ConvexTypeGeneratorError::InvalidUnicode(source_path.clone()))?;
        // Strip the .ts extension from the file name (Convex module path segment).
        let file_name = file_name.strip_suffix(".ts").unwrap_or(file_name).to_string();

        let module = convex_root.and_then(|root| path.strip_prefix(root).ok()).map(|relative| {
            relative
                .with_extension("")
                .iter()
                .map(|segment| segment.to_string_lossy().into_owned())
                .collect::<Vec<_>>()
        });

        // Get the body array
        let body = ast["body"]
            .as_array()
            .ok_or_else(|| ConvexTypeGeneratorError::InvalidSchema {
                context: format!("file_{}", file_name),
                details: "Missing body array".to_string(),
            })?;

        for node in body {
            // Look for export declarations
            if node["type"].as_str() == Some("ExportNamedDeclaration")
                && let Some(declaration) = node.get("declaration")
            {
                // Handle variable declarations (const testQuery = query({...}))
                if declaration["type"].as_str() == Some("VariableDeclaration")
                    && let Some(declarators) = declaration["declarations"].as_array()
                {
                    for declarator in declarators {
                        // Get function name
                        let name =
                            declarator["id"]["name"]
                                .as_str()
                                .ok_or_else(|| ConvexTypeGeneratorError::InvalidSchema {
                                    context: format!("file_{}", file_name),
                                    details: "Missing function name".to_string(),
                                })?;

                        // Get the function call (query/mutation/action)
                        let init = &declarator["init"];
                        if init["type"].as_str() == Some("CallExpression") {
                            // Get the callee to determine function type
                            let fn_type =
                                init["callee"]["name"]
                                    .as_str()
                                    .ok_or_else(|| ConvexTypeGeneratorError::InvalidSchema {
                                        context: format!("function_{}", name),
                                        details: "Missing function type".to_string(),
                                    })?;

                            // Get the first argument which contains the function config
                            if let Some(args) = init["arguments"].as_array()
                                && let Some(config) = args.first()
                            {
                                let (kind, visibility) = match fn_type {
                                    "query" => (FunctionKind::Query, Visibility::Public),
                                    "mutation" => (FunctionKind::Mutation, Visibility::Public),
                                    "action" => (FunctionKind::Action, Visibility::Public),
                                    "internalQuery" => (FunctionKind::Query, Visibility::Internal),
                                    "internalMutation" => (FunctionKind::Mutation, Visibility::Internal),
                                    "internalAction" => (FunctionKind::Action, Visibility::Internal),
                                    _ => (FunctionKind::Unknown, Visibility::Unknown),
                                };

                                convex_functions.push(Function {
                                    identity: FunctionIdentity {
                                        module: module.clone(),
                                        export: name.to_string(),
                                    },
                                    legacy_module_name: file_name.clone(),
                                    kind,
                                    visibility,
                                    args: extract_function_params(config, &file_name, name, &source_path)?,
                                    returns: extract_return_validator(config, &source_path),
                                    provenance: provenance(&source_path, init),
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(convex_functions)
}

/// ESTree / Oxc object literal member: either Babel-style `ObjectProperty` or spec `Property`.
fn is_estree_object_property_like(node: &JsonValue) -> bool
{
    matches!(node["type"].as_str(), Some("ObjectProperty") | Some("Property"))
}

/// Helper function to extract function parameters from the function configuration
fn extract_function_params(
    config: &JsonValue,
    file_name: &str,
    export_name: &str,
    source: &str,
) -> Result<Declared<Vec<Field>>, ConvexTypeGeneratorError>
{
    let mut params = Vec::new();

    // Get the args object from the function config
    if let Some(properties) = config["properties"].as_array() {
        for prop in properties {
            if prop["key"]["name"].as_str() == Some("args") {
                // Ensure args is an object
                if prop["value"]["type"].as_str() != Some("ObjectExpression") {
                    return Err(ConvexTypeGeneratorError::InvalidSchema {
                        context: format!("file_{}", file_name),
                        details: "Function args must be an object".to_string(),
                    });
                }

                // Get the args object value
                if let Some(args_props) = prop["value"]["properties"].as_array() {
                    for arg_prop in args_props {
                        // Oxc ESTree JSON uses `ObjectProperty` (Babel-style) or standard `Property` for
                        // `{ key: value }` entries; both carry `key` + `value` like our schema walk expects.
                        if !is_estree_object_property_like(arg_prop) {
                            let got = arg_prop["type"].as_str().unwrap_or("<missing type>");
                            return Err(ConvexTypeGeneratorError::InvalidSchema {
                                context: format!("file_{}", file_name),
                                details: format!(
                                    "Invalid argument property structure (expected ObjectProperty or Property, got {got})"
                                ),
                            });
                        }

                        // Get parameter name (identifier keys only for generated Rust field names)
                        let param_name =
                            arg_prop["key"]["name"]
                                .as_str()
                                .ok_or_else(|| ConvexTypeGeneratorError::InvalidSchema {
                                    context: format!("file_{}", file_name),
                                    details: "Invalid parameter name (expected a simple identifier key)".to_string(),
                                })?;

                        // Get parameter type using the same extraction logic as schema
                        let mut context = TypeContext::for_function_param(file_name, export_name, param_name);
                        let validator = parse_validator(&arg_prop["value"], &mut context)?;

                        params.push(Field {
                            name: param_name.to_string(),
                            validator,
                            provenance: provenance(source, arg_prop),
                        });
                    }
                }
                return Ok(Declared::Declared(params));
            }
        }
    }

    Ok(Declared::Unknown)
}

/// Returns are optional metadata for the current emitter. Unresolved expressions are unknown.
fn extract_return_validator(config: &JsonValue, source: &str) -> Declared<ReturnValidator>
{
    let Some(property) = config["properties"].as_array().and_then(|props| {
        props
            .iter()
            .find(|prop| prop["computed"] != true && member_property_name(&prop["key"]) == Some("returns"))
    }) else {
        return Declared::Unknown;
    };
    let mut context = TypeContext {
        file_name: source.to_string(),
        type_path: Vec::new(),
        naming: None,
        object_depth: 0,
        struct_name_stack: Vec::new(),
    };
    let value = &property["value"];
    match parse_validator(value, &mut context) {
        Ok(validator) => Declared::Declared(ReturnValidator {
            validator,
            provenance: provenance(source, value),
        }),
        Err(_) => Declared::Unknown,
    }
}

#[cfg(test)]
mod is_estree_object_property_like_tests
{
    use serde_json::json;

    use super::is_estree_object_property_like;

    #[test]
    fn accepts_object_property_and_property_nodes()
    {
        assert!(is_estree_object_property_like(&json!({ "type": "ObjectProperty" })));
        assert!(is_estree_object_property_like(&json!({ "type": "Property" })));
        assert!(!is_estree_object_property_like(&json!({ "type": "SpreadElement" })));
    }
}

#[cfg(test)]
mod parse_schema_ast_tests
{
    use std::path::Path;

    use serde_json::json;

    use super::parse_schema_ast;
    use crate::convex::validator::ConvexValidator as Validator;
    use crate::error::ConvexTypeGeneratorError;

    fn v_member_call(type_name: &str, arguments: Vec<serde_json::Value>) -> serde_json::Value
    {
        json!({
            "type": "CallExpression",
            "callee": {
                "type": "StaticMemberExpression",
                "object": { "type": "Identifier", "name": "v" },
                "property": { "type": "Identifier", "name": type_name }
            },
            "arguments": arguments
        })
    }

    fn col(name: &str, value: serde_json::Value) -> serde_json::Value
    {
        json!({
            "type": "ObjectProperty",
            "key": { "type": "Identifier", "name": name },
            "value": value
        })
    }

    fn export_default_define_schema(tables_props: Vec<serde_json::Value>) -> serde_json::Value
    {
        json!({
            "body": [{
                "type": "ExportDefaultDeclaration",
                "declaration": {
                    "type": "CallExpression",
                    "callee": { "type": "Identifier", "name": "defineSchema" },
                    "arguments": [{
                        "type": "ObjectExpression",
                        "properties": tables_props
                    }]
                }
            }]
        })
    }

    fn table(name: &str, columns: Vec<serde_json::Value>) -> serde_json::Value
    {
        json!({
            "type": "ObjectProperty",
            "key": { "type": "Identifier", "name": name },
            "value": {
                "type": "CallExpression",
                "callee": { "type": "Identifier", "name": "defineTable" },
                "arguments": [{
                    "type": "ObjectExpression",
                    "properties": columns
                }]
            }
        })
    }

    /// `defineTable({ ... }).index("name", ["field"])` as emitted by Oxc for real Convex schemas.
    fn table_with_index_chain(name: &str, columns: Vec<serde_json::Value>) -> serde_json::Value
    {
        json!({
            "type": "ObjectProperty",
            "key": { "type": "Identifier", "name": name },
            "value": {
                "type": "CallExpression",
                "callee": {
                    "type": "MemberExpression",
                    "object": {
                        "type": "CallExpression",
                        "callee": { "type": "Identifier", "name": "defineTable" },
                        "arguments": [{
                            "type": "ObjectExpression",
                            "properties": columns
                        }]
                    },
                    "property": { "type": "Identifier", "name": "index" }
                },
                "arguments": [
                    { "type": "Literal", "value": "by_email" },
                    { "type": "ArrayExpression", "elements": [] }
                ]
            }
        })
    }

    #[test]
    fn missing_body_errors()
    {
        let err = parse_schema_ast(json!({}), Path::new("schema.ts")).unwrap_err();
        assert!(
            matches!(err, ConvexTypeGeneratorError::InvalidSchema { ref details, .. } if details == "Missing body array")
        );
    }

    #[test]
    fn missing_define_schema_errors()
    {
        let ast = json!({ "body": [{ "type": "EmptyStatement" }] });
        let err = parse_schema_ast(ast, Path::new("schema.ts")).unwrap_err();
        assert!(matches!(err, ConvexTypeGeneratorError::InvalidSchema { .. }));
    }

    #[test]
    fn parses_single_string_column()
    {
        let ast = export_default_define_schema(vec![table("users", vec![col("email", v_member_call("string", vec![]))])]);
        let schema = parse_schema_ast(ast, Path::new("schema.ts")).unwrap();
        assert_eq!(schema.tables.len(), 1);
        assert_eq!(schema.tables[0].name, "users");
        assert_eq!(schema.tables[0].fields.len(), 1);
        assert_eq!(schema.tables[0].fields[0].name, "email");
        assert_eq!(schema.tables[0].fields[0].validator, Validator::String);
    }

    #[test]
    fn parses_single_string_column_through_chained_index()
    {
        let ast = export_default_define_schema(vec![table_with_index_chain(
            "users",
            vec![col("email", v_member_call("string", vec![]))],
        )]);
        let schema = parse_schema_ast(ast, Path::new("schema.ts")).unwrap();
        assert_eq!(schema.tables[0].fields[0].name, "email");
        assert_eq!(schema.tables[0].fields[0].validator, Validator::String);
    }

    #[test]
    fn optional_wraps_inner()
    {
        let ast = export_default_define_schema(vec![table(
            "t",
            vec![col("x", v_member_call("optional", vec![v_member_call("boolean", vec![])]))],
        )]);
        let schema = parse_schema_ast(ast, Path::new("schema.ts")).unwrap();
        assert_eq!(
            schema.tables[0].fields[0].validator,
            Validator::Optional(Box::new(Validator::Boolean))
        );
    }
}

#[cfg(test)]
mod parse_function_ast_tests
{
    use std::collections::BTreeMap;

    use serde_json::json;

    use super::parse_function_ast;
    use crate::error::ConvexTypeGeneratorError;

    fn export_query_fn(name: &str, args_props: Vec<serde_json::Value>) -> serde_json::Value
    {
        json!({
            "body": [{
                "type": "ExportNamedDeclaration",
                "declaration": {
                    "type": "VariableDeclaration",
                    "declarations": [{
                        "id": { "type": "Identifier", "name": name },
                        "init": {
                            "type": "CallExpression",
                            "callee": { "type": "Identifier", "name": "query" },
                            "arguments": [{
                                "type": "ObjectExpression",
                                "properties": [{
                                    "type": "ObjectProperty",
                                    "key": { "name": "args" },
                                    "value": {
                                        "type": "ObjectExpression",
                                        "properties": args_props
                                    }
                                }]
                            }]
                        }
                    }]
                }
            }]
        })
    }

    fn arg_property(kind: &str, name: &str, value: serde_json::Value) -> serde_json::Value
    {
        json!({
            "type": kind,
            "key": { "type": "Identifier", "name": name },
            "value": value
        })
    }

    fn v_member_call(type_name: &str, arguments: Vec<serde_json::Value>) -> serde_json::Value
    {
        json!({
            "type": "CallExpression",
            "callee": {
                "type": "StaticMemberExpression",
                "object": { "type": "Identifier", "name": "v" },
                "property": { "type": "Identifier", "name": type_name }
            },
            "arguments": arguments
        })
    }

    #[test]
    fn parses_query_with_object_property_args()
    {
        let ast = export_query_fn(
            "list",
            vec![arg_property("ObjectProperty", "limit", v_member_call("number", vec![]))],
        );
        let mut m = BTreeMap::new();
        m.insert("/x/api.ts".into(), ast);
        let fns = parse_function_ast(m, None).unwrap();
        assert_eq!(fns.len(), 1);
        assert_eq!(fns[0].identity.export, "list");
        assert_eq!(fns[0].params().len(), 1);
        assert_eq!(fns[0].params()[0].name, "limit");
    }

    #[test]
    fn parses_query_with_property_args()
    {
        let ast = export_query_fn(
            "list2",
            vec![arg_property("Property", "limit", v_member_call("number", vec![]))],
        );
        let mut m = BTreeMap::new();
        m.insert("/y/z.ts".into(), ast);
        let fns = parse_function_ast(m, None).unwrap();
        assert_eq!(fns[0].params().len(), 1);
    }

    #[test]
    fn rejects_spread_in_args_object()
    {
        let ast = export_query_fn(
            "bad",
            vec![json!({
                "type": "SpreadElement",
                "argument": { "type": "Identifier", "name": "x" }
            })],
        );
        let mut m = BTreeMap::new();
        m.insert("/a/b.ts".into(), ast);
        let err = parse_function_ast(m, None).unwrap_err();
        assert!(matches!(err, ConvexTypeGeneratorError::InvalidSchema { .. }));
    }

    #[test]
    fn missing_body_errors()
    {
        let mut m = BTreeMap::new();
        m.insert("/p.ts".into(), json!({}));
        let err = parse_function_ast(m, None).unwrap_err();
        assert!(matches!(err, ConvexTypeGeneratorError::InvalidSchema { .. }));
    }
}

#[cfg(test)]
mod find_define_schema_tests
{
    use serde_json::json;

    use super::find_define_schema;

    #[test]
    fn none_on_empty_body()
    {
        assert!(find_define_schema(&[]).is_none());
    }

    #[test]
    fn finds_export_default_define_schema()
    {
        let body = vec![json!({
            "type": "ExportDefaultDeclaration",
            "declaration": {
                "type": "CallExpression",
                "callee": { "type": "Identifier", "name": "defineSchema" },
                "arguments": []
            }
        })];
        let got = find_define_schema(&body).unwrap();
        assert_eq!(got["callee"]["name"], "defineSchema");
    }

    #[test]
    fn finds_top_level_define_schema_call()
    {
        let body = vec![json!({
            "type": "CallExpression",
            "callee": { "type": "Identifier", "name": "defineSchema" },
            "arguments": []
        })];
        assert!(find_define_schema(&body).is_some());
    }
}

#[cfg(test)]
mod extract_function_params_tests
{
    use serde_json::json;

    use super::{Declared, extract_function_params};
    use crate::error::ConvexTypeGeneratorError;

    #[test]
    fn empty_when_no_args_property()
    {
        let config = json!({
            "type": "ObjectExpression",
            "properties": [{
                "type": "ObjectProperty",
                "key": { "name": "handler" },
                "value": { "type": "ArrowFunctionExpression" }
            }]
        });
        assert_eq!(
            extract_function_params(&config, "f", "fn", "f.ts").unwrap(),
            Declared::Unknown
        );
    }

    #[test]
    fn errors_when_args_is_not_object_expression()
    {
        let config = json!({
            "type": "ObjectExpression",
            "properties": [{
                "type": "ObjectProperty",
                "key": { "name": "args" },
                "value": { "type": "Identifier", "name": "sharedArgs" }
            }]
        });
        let err = extract_function_params(&config, "f", "fn", "f.ts").unwrap_err();
        assert!(
            matches!(err, ConvexTypeGeneratorError::InvalidSchema { ref details, .. } if details == "Function args must be an object")
        );
    }
}

#[cfg(test)]
mod oxc_ir_tests
{
    use std::fs;

    use tempfile::tempdir;

    use super::{parse_function_ast, parse_schema_ast};
    use crate::convex::ir::{Declared, FieldPresence, FunctionKind, Provenance, Visibility};
    use crate::convex::validator::{ConvexValidator as Validator, Literal};
    use crate::convex::{create_function_asts, create_schema_ast};

    fn source_excerpt<'a>(text: &'a str, provenance: &Provenance) -> &'a str
    {
        let span = provenance.span.expect("Oxc provides a byte span");
        &text[span.start as usize..span.end as usize]
    }

    #[test]
    fn oxc_preserves_identity_validators_and_provenance()
    {
        let dir = tempdir().unwrap();
        let root = dir.path().canonicalize().unwrap();
        let schema_path = root.join("schema.ts");
        let module_path = root.join("admin/users.ts");
        fs::create_dir(root.join("admin")).unwrap();
        let schema_source = "export default defineSchema({ users: defineTable({ team: v.optional(v.id(\"teams\")) }) });";
        let function_source = r#"
            export const list = internalQuery({
                args: { limit: v.optional(v.number()) },
                returns: v.union(v.any(), v.literal(-9223372036854775808n), v.literal("none")),
            });
            export const empty = query({ args: {}, returns: v.any() });
            export const absent = query({ handler: () => null });
            export const unresolved = action({ returns: sharedValidator });
        "#;
        fs::write(&schema_path, schema_source).unwrap();
        fs::write(&module_path, function_source).unwrap();

        let schema = parse_schema_ast(create_schema_ast(&schema_path).unwrap(), &schema_path).unwrap();
        let table = &schema.tables[0];
        let field = &table.fields[0];
        assert_eq!(field.presence(), FieldPresence::Optional);
        assert_eq!(
            field.validator,
            Validator::Optional(Box::new(Validator::Id(Some("teams".into()))))
        );
        assert_eq!(schema.provenance.source, schema_path.to_str().unwrap());
        assert!(source_excerpt(schema_source, &schema.provenance).starts_with("defineSchema("));
        assert!(source_excerpt(schema_source, &table.provenance).starts_with("users: defineTable("));
        assert_eq!(
            source_excerpt(schema_source, &field.provenance),
            "team: v.optional(v.id(\"teams\"))"
        );

        let functions = parse_function_ast(create_function_asts(vec![module_path.clone()]).unwrap(), Some(&root)).unwrap();
        let [list, empty, absent, unresolved] = functions.as_slice() else {
            panic!("expected four functions, got {functions:?}");
        };
        assert_eq!(list.identity.module, Some(vec!["admin".into(), "users".into()]));
        assert_eq!(list.identity.export, "list");
        assert_eq!(list.kind, FunctionKind::Query);
        assert_eq!(list.visibility, Visibility::Internal);
        assert_eq!(list.provenance.source, module_path.to_str().unwrap());
        assert!(source_excerpt(function_source, &list.provenance).starts_with("internalQuery("));
        assert_eq!(
            source_excerpt(function_source, &list.params()[0].provenance),
            "limit: v.optional(v.number())"
        );
        let Declared::Declared(returns) = &list.returns else {
            panic!("expected a declared return validator");
        };
        assert_eq!(
            returns.validator,
            Validator::Union(vec![
                Validator::Any,
                Validator::Literal(Some(Literal::Int64(i64::MIN))),
                Validator::Literal(Some(Literal::String("none".into()))),
            ])
        );
        assert!(source_excerpt(function_source, &returns.provenance).starts_with("v.union("));
        assert_eq!(empty.args, Declared::Declared(vec![]));
        assert!(matches!(empty.returns, Declared::Declared(ref returns) if returns.validator == Validator::Any));
        assert_eq!(absent.args, Declared::Unknown);
        assert_eq!(absent.returns, Declared::Unknown);
        assert_eq!(unresolved.returns, Declared::Unknown);
    }
}
