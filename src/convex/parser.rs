//! Walk ESTree JSON from [`crate::convex::lexer`] into [`crate::convex::types`].
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
//!
//! ## Cycles
//!
//! `v.object` nesting is tracked to catch self-referential object shapes; other wrappers (`array`,
//! `optional`, …) participate in recursion for validation but only `object` participates in the
//! stack used to detect cycles (see [`TypeContext`]).

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde_json::{json, Value as JsonValue};

use crate::convex::types::{ConvexColumn, ConvexFunction, ConvexFunctionParam, ConvexFunctions, ConvexSchema, ConvexTable};
use crate::convex::utils::validate_type_name;
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
pub(crate) fn parse_schema_ast(ast: JsonValue) -> Result<ConvexSchema, ConvexTypeGeneratorError>
{
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
            let mut context = TypeContext::new(context.to_string());
            let column_type = extract_column_type(column_prop, &mut context)?;

            columns.push(ConvexColumn {
                name: column_name.to_string(),
                data_type: column_type,
            });
        }

        tables.push(ConvexTable {
            name: table_name.to_string(),
            columns,
        });
    }

    Ok(ConvexSchema { tables })
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
                if let Some(callee) = declaration.get("callee") {
                    if callee["type"].as_str() == Some("Identifier") && callee["name"].as_str() == Some("defineSchema") {
                        return Some(declaration);
                    }
                }
            }
        }

        // Could also be a regular variable declaration or expression
        // that calls defineSchema
        if node["type"].as_str() == Some("CallExpression") {
            if let Some(callee) = node.get("callee") {
                if callee["type"].as_str() == Some("Identifier") && callee["name"].as_str() == Some("defineSchema") {
                    return Some(node);
                }
            }
        }
    }
    None
}

/// Helper function to extract the column type from a column property
fn extract_column_type(column_prop: &JsonValue, context: &mut TypeContext) -> Result<JsonValue, ConvexTypeGeneratorError>
{
    let value = &column_prop["value"];
    let callee = &value["callee"];

    let type_name = callee["property"]["name"]
        .as_str()
        .ok_or_else(|| ConvexTypeGeneratorError::InvalidSchema {
            context: context.get_error_context(),
            details: "Invalid column type".to_string(),
        })?;

    // Validate the type name
    validate_type_name(type_name)?;

    let binding = Vec::new();
    let args = value["arguments"].as_array().unwrap_or(&binding);

    let mut type_obj = serde_json::Map::new();
    type_obj.insert("type".to_string(), JsonValue::String(type_name.to_string()));

    // Handle nested types
    match type_name {
        "optional" => {
            // For optional types, recursively parse the inner type
            if let Some(inner_type) = args.first() {
                let inner_type_prop = json!({
                    "key": { "name": "inner" },
                    "value": inner_type
                });
                context.type_path.push("inner".to_string());
                let parsed_inner_type = extract_column_type(&inner_type_prop, context)?;
                context.type_path.pop();
                type_obj.insert("inner".to_string(), parsed_inner_type);
            } else {
                return Err(ConvexTypeGeneratorError::InvalidSchema {
                    context: context.type_path.join("."),
                    details: "Optional type must have an inner type".to_string(),
                });
            }
        }
        "array" => {
            // For arrays, recursively parse the element type
            if let Some(element_type) = args.first() {
                let element_type_prop = json!({
                    "key": { "name": "element" },
                    "value": element_type
                });
                context.type_path.push("elements".to_string());
                let parsed_element_type = extract_column_type(&element_type_prop, context)?;
                context.type_path.pop();
                type_obj.insert("elements".to_string(), parsed_element_type);
            }
        }
        "object" => {
            // For objects, parse each property type
            if let Some(obj_def) = args.first() {
                if let Some(properties) = obj_def["properties"].as_array() {
                    let mut prop_types = serde_json::Map::new();

                    for prop in properties {
                        let prop_name =
                            prop["key"]["name"]
                                .as_str()
                                .ok_or_else(|| ConvexTypeGeneratorError::InvalidSchema {
                                    context: context.type_path.join("."),
                                    details: "Invalid object property name".to_string(),
                                })?;

                        let prop_type = extract_column_type(prop, context)?;
                        prop_types.insert(prop_name.to_string(), prop_type);
                    }

                    type_obj.insert("properties".to_string(), JsonValue::Object(prop_types));
                }
            }
        }
        "record" => {
            // For records, parse both key and value types
            if args.len() >= 2 {
                // First argument is the key type
                let key_type_prop = json!({
                    "key": { "name": "key" },
                    "value": args[0]
                });
                let key_type = extract_column_type(&key_type_prop, context)?;
                type_obj.insert("keyType".to_string(), key_type);

                // Second argument is the value type
                let value_type_prop = json!({
                    "key": { "name": "value" },
                    "value": args[1]
                });
                let value_type = extract_column_type(&value_type_prop, context)?;
                type_obj.insert("valueType".to_string(), value_type);
            }
        }
        "union" => {
            // For unions, parse all variant types
            let mut variants = Vec::new();
            for variant in args {
                let variant_prop = json!({
                    "key": { "name": "variant" },
                    "value": variant
                });
                let variant_type = extract_column_type(&variant_prop, context)?;
                variants.push(variant_type);
            }
            type_obj.insert("variants".to_string(), JsonValue::Array(variants));
        }
        "literal" => {
            // For literals, store the literal value
            if let Some(literal_value) = args.first() {
                type_obj.insert("value".to_string(), literal_value.clone());
            }
        }
        // For other types, just include their arguments if any
        _ => {
            if !args.is_empty() {
                type_obj.insert("arguments".to_string(), JsonValue::Array(args.to_vec()));
            }
        }
    }

    // Build the type object as before...
    let type_value = JsonValue::Object(type_obj);

    // Check for circular references
    check_circular_references(&type_value, context)?;

    Ok(type_value)
}

/// Parser-local state for nested `v.*` extraction and cycle detection.
///
/// `type_stack` tracks `object` nesting paths; other composite kinds recurse but only `object`
/// participates in the “seen this path before?” cycle check.
#[derive(Debug, Default)]
struct TypeContext
{
    /// `(discriminator, full_path)` for nested `v.object` walks.
    type_stack: Vec<(String, String)>,
    /// Prefix for [`crate::error::ConvexTypeGeneratorError::InvalidSchema`] messages.
    file_name: String,
    /// Dot-separated path inside the validator tree (`inner`, `elements`, field names, …).
    type_path: Vec<String>,
}

impl TypeContext
{
    fn new(file_name: String) -> Self
    {
        Self {
            file_name,
            type_stack: Vec::new(),
            type_path: Vec::new(),
        }
    }

    fn push_type(&mut self, type_name: &str) -> Result<(), ConvexTypeGeneratorError>
    {
        let current_path = self.type_path.join(".");

        // Only check for circular references in object types
        // Arrays and other container types can be nested
        if type_name == "object" {
            let full_path = if current_path.is_empty() {
                type_name.to_string()
            } else {
                format!("{}.{}", current_path, type_name)
            };

            // Check if this exact path has been seen before
            if self.type_stack.iter().any(|(_, path)| path == &full_path) {
                return Err(ConvexTypeGeneratorError::CircularReference {
                    path: self.type_stack.iter().map(|(_, path)| path.clone()).collect(),
                });
            }
            self.type_stack.push((type_name.to_string(), full_path));
        }
        Ok(())
    }

    /// Get the current context for error messages
    fn get_error_context(&self) -> String
    {
        format!("{}:{}", self.file_name, self.type_path.join("."))
    }

    /// Removes the most recently pushed type from the stack
    fn pop_type(&mut self)
    {
        // Only pop if the last type was an object (matches push_type behavior)
        if let Some((type_name, _)) = self.type_stack.last() {
            if type_name == "object" {
                self.type_stack.pop();
            }
        }
    }
}

fn check_circular_references(type_obj: &JsonValue, context: &mut TypeContext) -> Result<(), ConvexTypeGeneratorError>
{
    let type_name = type_obj["type"]
        .as_str()
        .ok_or_else(|| ConvexTypeGeneratorError::InvalidSchema {
            context: context.type_path.join("."),
            details: "Missing type name".to_string(),
        })?;

    context.push_type(type_name)?;

    match type_name {
        "optional" => {
            if let Some(inner) = type_obj.get("inner") {
                context.type_path.push("inner".to_string());
                check_circular_references(inner, context)?;
                context.type_path.pop();
            }
        }
        "array" => {
            if let Some(elements) = type_obj.get("elements") {
                context.type_path.push("elements".to_string());
                check_circular_references(elements, context)?;
                context.type_path.pop();
            }
        }
        "object" => {
            if let Some(properties) = type_obj.get("properties") {
                if let Some(props) = properties.as_object() {
                    for (prop_name, prop_type) in props {
                        context.type_path.push(prop_name.to_string());
                        check_circular_references(prop_type, context)?;
                        context.type_path.pop();
                    }
                }
            }
        }
        "record" => {
            // Check key type
            if let Some(key_type) = type_obj.get("keyType") {
                context.type_path.push("keyType".to_string());
                check_circular_references(key_type, context)?;
                context.type_path.pop();
            }
            // Check value type
            if let Some(value_type) = type_obj.get("valueType") {
                context.type_path.push("valueType".to_string());
                check_circular_references(value_type, context)?;
                context.type_path.pop();
            }
        }
        "union" | "intersection" => {
            if let Some(variants) = type_obj["variants"].as_array() {
                for (i, variant) in variants.iter().enumerate() {
                    context.type_path.push(format!("variant_{}", i));
                    check_circular_references(variant, context)?;
                    context.type_path.pop();
                }
            }
        }
        _ => {} // Other types don't have nested types
    }

    context.pop_type();
    Ok(())
}

/// Walk every module’s ESTree JSON and collect exported Convex endpoints.
pub(crate) fn parse_function_ast(ast_map: BTreeMap<String, JsonValue>) -> Result<ConvexFunctions, ConvexTypeGeneratorError>
{
    let mut convex_functions = Vec::new();

    for (source_path, ast) in ast_map {
        let path_buf = PathBuf::from(&source_path);
        let file_name = path_buf
            .file_name()
            .ok_or_else(|| ConvexTypeGeneratorError::InvalidPath(source_path.clone()))?
            .to_str()
            .ok_or_else(|| ConvexTypeGeneratorError::InvalidUnicode(source_path.clone()))?;
        // Strip the .ts extension from the file name (Convex module path segment).
        let file_name = file_name.strip_suffix(".ts").unwrap_or(file_name).to_string();

        // Get the body array
        let body = ast["body"]
            .as_array()
            .ok_or_else(|| ConvexTypeGeneratorError::InvalidSchema {
                context: format!("file_{}", file_name),
                details: "Missing body array".to_string(),
            })?;

        for node in body {
            // Look for export declarations
            if node["type"].as_str() == Some("ExportNamedDeclaration") {
                if let Some(declaration) = node.get("declaration") {
                    // Handle variable declarations (const testQuery = query({...}))
                    if declaration["type"].as_str() == Some("VariableDeclaration") {
                        if let Some(declarators) = declaration["declarations"].as_array() {
                            for declarator in declarators {
                                // Get function name
                                let name = declarator["id"]["name"].as_str().ok_or_else(|| {
                                    ConvexTypeGeneratorError::InvalidSchema {
                                        context: format!("file_{}", file_name),
                                        details: "Missing function name".to_string(),
                                    }
                                })?;

                                // Get the function call (query/mutation/action)
                                let init = &declarator["init"];
                                if init["type"].as_str() == Some("CallExpression") {
                                    // Get the callee to determine function type
                                    let fn_type = init["callee"]["name"].as_str().ok_or_else(|| {
                                        ConvexTypeGeneratorError::InvalidSchema {
                                            context: format!("function_{}", name),
                                            details: "Missing function type".to_string(),
                                        }
                                    })?;

                                    // Get the first argument which contains the function config
                                    if let Some(args) = init["arguments"].as_array() {
                                        if let Some(config) = args.first() {
                                            // Extract function parameters from the args property
                                            let params = extract_function_params(config, &file_name)?;

                                            convex_functions.push(ConvexFunction {
                                                name: name.to_string(),
                                                params,
                                                type_: fn_type.to_string(),
                                                file_name: file_name.to_string(),
                                            });
                                        }
                                    }
                                }
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
fn extract_function_params(config: &JsonValue, file_name: &str)
    -> Result<Vec<ConvexFunctionParam>, ConvexTypeGeneratorError>
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
                        let mut context = TypeContext::new(format!("function_{}", param_name));
                        let param_type = extract_column_type(arg_prop, &mut context)?;

                        params.push(ConvexFunctionParam {
                            name: param_name.to_string(),
                            data_type: param_type,
                        });
                    }
                }
                break; // Found args object, no need to continue
            }
        }
    }

    Ok(params)
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
    use serde_json::json;

    use super::parse_schema_ast;
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
        let err = parse_schema_ast(json!({})).unwrap_err();
        assert!(
            matches!(err, ConvexTypeGeneratorError::InvalidSchema { ref details, .. } if details == "Missing body array")
        );
    }

    #[test]
    fn missing_define_schema_errors()
    {
        let ast = json!({ "body": [{ "type": "EmptyStatement" }] });
        let err = parse_schema_ast(ast).unwrap_err();
        assert!(matches!(err, ConvexTypeGeneratorError::InvalidSchema { .. }));
    }

    #[test]
    fn parses_single_string_column()
    {
        let ast = export_default_define_schema(vec![table("users", vec![col("email", v_member_call("string", vec![]))])]);
        let schema = parse_schema_ast(ast).unwrap();
        assert_eq!(schema.tables.len(), 1);
        assert_eq!(schema.tables[0].name, "users");
        assert_eq!(schema.tables[0].columns.len(), 1);
        assert_eq!(schema.tables[0].columns[0].name, "email");
        assert_eq!(schema.tables[0].columns[0].data_type["type"], "string");
    }

    #[test]
    fn parses_single_string_column_through_chained_index()
    {
        let ast = export_default_define_schema(vec![table_with_index_chain(
            "users",
            vec![col("email", v_member_call("string", vec![]))],
        )]);
        let schema = parse_schema_ast(ast).unwrap();
        assert_eq!(schema.tables[0].columns[0].name, "email");
        assert_eq!(schema.tables[0].columns[0].data_type["type"], "string");
    }

    #[test]
    fn optional_wraps_inner()
    {
        let ast = export_default_define_schema(vec![table(
            "t",
            vec![col("x", v_member_call("optional", vec![v_member_call("boolean", vec![])]))],
        )]);
        let schema = parse_schema_ast(ast).unwrap();
        let dt = &schema.tables[0].columns[0].data_type;
        assert_eq!(dt["type"], "optional");
        assert_eq!(dt["inner"]["type"], "boolean");
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
        let fns = parse_function_ast(m).unwrap();
        assert_eq!(fns.len(), 1);
        assert_eq!(fns[0].name, "list");
        assert_eq!(fns[0].params.len(), 1);
        assert_eq!(fns[0].params[0].name, "limit");
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
        let fns = parse_function_ast(m).unwrap();
        assert_eq!(fns[0].params.len(), 1);
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
        let err = parse_function_ast(m).unwrap_err();
        assert!(matches!(err, ConvexTypeGeneratorError::InvalidSchema { .. }));
    }

    #[test]
    fn missing_body_errors()
    {
        let mut m = BTreeMap::new();
        m.insert("/p.ts".into(), json!({}));
        let err = parse_function_ast(m).unwrap_err();
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

    use super::extract_function_params;
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
        assert!(extract_function_params(&config, "f").unwrap().is_empty());
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
        let err = extract_function_params(&config, "f").unwrap_err();
        assert!(
            matches!(err, ConvexTypeGeneratorError::InvalidSchema { ref details, .. } if details == "Function args must be an object")
        );
    }
}
