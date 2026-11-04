//! Walk ESTree JSON from [`crate::convex::lexer`] into [`crate::convex::ir`].
//!
//! ## Schema
//!
//! Expects `export default defineSchema({ ... })` (or an equivalent top-level `defineSchema` call).
//! Table bodies accept property maps, object validators, and unions of document validators.
//! A `defineTable` call may be followed by Convex builder chains such as
//! `.index(...)`, `.searchIndex(...)`, or `.vectorIndex(...)`. We peel those calls to reach the
//! inner `defineTable` and lower its first argument to a document shape.
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
    TableDocument, TableShape, Visibility,
};
use crate::convex::validator::{ConvexValidator as Validator, FunctionSignature, Literal};
use crate::error::ConvexTypeGeneratorError;

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
fn peel_to_define_table_call<'a>(
    expr: &'a JsonValue,
    source: &ValidatorSource<'_>,
) -> Result<&'a JsonValue, ConvexTypeGeneratorError>
{
    let mut cur = expr;
    while cur["type"] == "CallExpression" {
        let callee = &cur["callee"];
        if callee["type"] == "Identifier" && callee["name"] == "defineTable" {
            return Ok(cur);
        }
        if callee["type"] == "MemberExpression"
            && matches!(
                static_member_name(callee, "property"),
                Some("index" | "searchIndex" | "vectorIndex")
            )
        {
            cur = &callee["object"];
        } else {
            break;
        }
    }
    Err(source.error(
        cur,
        "defineTable",
        "expected defineTable(...) optionally followed by .index(), .searchIndex(), or .vectorIndex()",
    ))
}

/// Extract tables and columns from a schema program JSON value.
#[cfg(test)]
pub(crate) fn parse_schema_ast(ast: JsonValue, source: &Path) -> Result<Schema, ConvexTypeGeneratorError>
{
    parse_schema_ast_with_modules(ast, source, None)
}

pub(crate) fn parse_schema_ast_with_modules(
    ast: JsonValue,
    source: &Path,
    modules: Option<&BTreeMap<String, JsonValue>>,
) -> Result<Schema, ConvexTypeGeneratorError>
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

    let input = ValidatorSource::new(&source, body, modules);
    let mut tables = Vec::new();
    for table_prop in tables_obj {
        let table_name = static_property_key(table_prop)
            .ok_or_else(|| input.error(table_prop, "defineSchema", "expected a static table name"))?;
        let table_expr = peel_to_define_table_call(&table_prop["value"], &input)?;
        let args = input.call(table_expr, "defineTable", 1, 1)?;
        let shape = parse_table_shape(&args[0], &input, true)?;
        tables.push(TableDocument {
            name: table_name.to_string(),
            shape,
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

/// File-local, top-level const initializers. Only earlier bindings can be used by an initializer.
struct ValidatorSource<'a>
{
    file: &'a str,
    bindings: BTreeMap<&'a str, (&'a JsonValue, bool)>,
    node_files: BTreeMap<usize, &'a str>,
}

type StaticProperty<'a> = Result<(&'a str, &'a JsonValue), ConvexTypeGeneratorError>;

impl<'a> ValidatorSource<'a>
{
    fn new(file: &'a str, body: &'a [JsonValue], modules: Option<&'a BTreeMap<String, JsonValue>>) -> Self
    {
        let mut node_files = BTreeMap::new();
        for node in body {
            index_node_files(node, file, &mut node_files);
        }
        let mut bindings: BTreeMap<&'a str, (&'a JsonValue, bool)> = body
            .iter()
            .map(|node| {
                if node["type"] == "ExportNamedDeclaration" {
                    &node["declaration"]
                } else {
                    node
                }
            })
            .filter(|node| node["type"] == "VariableDeclaration" && node["kind"] == "const")
            .filter_map(|node| node["declarations"].as_array())
            .flatten()
            .filter_map(|decl| {
                Some((
                    decl["id"]["name"].as_str()?,
                    (decl.get("init").filter(|init| !init.is_null())?, false),
                ))
            })
            .collect();
        if let Some(modules) = modules {
            for (module_path, ast) in modules {
                index_node_files(ast, module_path, &mut node_files);
            }
            for node in body.iter().filter(|node| node["type"] == "ImportDeclaration") {
                let Some(specifier) = node["source"]["value"].as_str() else {
                    continue;
                };
                if !specifier.starts_with('.') {
                    continue;
                }
                let Some(imported_module) = resolve_import_path(file, specifier, modules) else {
                    continue;
                };
                let Some(module_body) = modules[imported_module]["body"].as_array() else {
                    continue;
                };
                // Imported validator constants may themselves refer to sibling constants in the
                // same module (for example `payload = v.object({ ...common })`).
                for declaration in module_body
                    .iter()
                    .filter_map(|node| {
                        if node["type"] == "ExportNamedDeclaration" {
                            node.get("declaration")
                        } else {
                            Some(node)
                        }
                    })
                    .filter(|node| node["type"] == "VariableDeclaration" && node["kind"] == "const")
                {
                    for decl in declaration["declarations"].as_array().into_iter().flatten() {
                        if let (Some(name), Some(init)) = (decl["id"]["name"].as_str(), decl.get("init")) {
                            bindings.entry(name).or_insert((init, true));
                        }
                    }
                }
                for binding in node["specifiers"].as_array().into_iter().flatten() {
                    if binding["type"] != "ImportSpecifier" {
                        continue;
                    }
                    let Some(imported) = binding["imported"]["name"]
                        .as_str()
                        .or_else(|| binding["imported"]["value"].as_str())
                    else {
                        continue;
                    };
                    let Some(local) = binding["local"]["name"].as_str() else {
                        continue;
                    };
                    if let Some(init) = exported_const(module_body, imported) {
                        bindings.insert(local, (init, true));
                    }
                }
            }
        }
        Self {
            file,
            bindings,
            node_files,
        }
    }

    fn error(&self, node: &JsonValue, validator: &str, details: impl std::fmt::Display) -> ConvexTypeGeneratorError
    {
        let source_file = self
            .node_files
            .get(&(node as *const JsonValue as usize))
            .copied()
            .unwrap_or(self.file);
        let location = provenance(source_file, node).span.map_or_else(
            || source_file.to_string(),
            |span| format!("{} bytes {}..{}", source_file, span.start, span.end),
        );
        ConvexTypeGeneratorError::InvalidSchema {
            context: location,
            details: format!("{validator}: {details}"),
        }
    }

    fn resolve<'node>(&'node self, mut node: &'node JsonValue) -> Result<&'node JsonValue, ConvexTypeGeneratorError>
    {
        let mut chain = Vec::new();
        loop {
            match node["type"].as_str() {
                Some("TSAsExpression" | "TSSatisfiesExpression" | "TSTypeAssertion" | "TSNonNullExpression") => {
                    node = &node["expression"];
                }
                Some("Identifier") if let Some(name) = node["name"].as_str() => {
                    if chain.len() >= 64 {
                        return Err(self.error(node, name, "validator reference depth exceeds the limit of 64"));
                    }
                    if chain.iter().any(|entry| entry == name) {
                        chain.push(name.to_string());
                        return Err(self.error(node, name, format!("cyclic validator reference: {}", chain.join(" -> "))));
                    }
                    let (init, imported) = self.bindings.get(name).ok_or_else(|| {
                        self.error(node, name, "expected an inline validator or a same-file const binding")
                    })?;
                    if !imported
                        && matches!(
                            (init["start"].as_u64(), init["end"].as_u64(), node["start"].as_u64()),
                            (Some(init_start), Some(init_end), Some(start)) if init_start <= start && start < init_end
                        )
                    {
                        return Err(self.error(node, name, format!("cyclic validator reference: {name} -> {name}")));
                    }
                    if !imported
                        && !matches!((init["end"].as_u64(), node["start"].as_u64()), (Some(end), Some(start)) if end <= start)
                    {
                        return Err(self.error(node, name, "validator const must be initialized before use"));
                    }
                    chain.push(name.to_string());
                    node = init;
                }
                _ => return Ok(node),
            }
        }
    }

    fn is_undefined(&self, node: &JsonValue) -> bool
    {
        node["type"] == "Identifier" && node["name"] == "undefined" && !self.bindings.contains_key("undefined")
    }

    fn properties(
        &'a self,
        node: &'a JsonValue,
        validator: &str,
    ) -> Result<Vec<StaticProperty<'a>>, ConvexTypeGeneratorError>
    {
        let props = node["properties"]
            .as_array()
            .filter(|_| node["type"] == "ObjectExpression")
            .ok_or_else(|| self.error(node, validator, "expected an object literal or a const bound to one"))?;
        let mut result = Vec::new();
        self.expand_properties(props, validator, &mut result, 0)?;
        Ok(result)
    }

    fn expand_properties(
        &'a self,
        props: &'a [JsonValue],
        validator: &str,
        result: &mut Vec<StaticProperty<'a>>,
        depth: usize,
    ) -> Result<(), ConvexTypeGeneratorError>
    {
        if depth >= 64 {
            return Err(self.error(&props[0], validator, "object spread depth exceeds the limit of 64"));
        }
        for prop in props {
            if prop["type"] == "SpreadElement" {
                let spread = self.resolve(&prop["argument"]).map_err(|_| {
                    self.error(
                        prop,
                        validator,
                        "unsupported dynamic object spread; use a const object literal or imported const",
                    )
                })?;
                let Some(spread_props) = spread["properties"]
                    .as_array()
                    .filter(|_| spread["type"] == "ObjectExpression")
                else {
                    result.push(Err(self.error(
                        prop,
                        validator,
                        "unsupported dynamic object spread; use a const object literal or imported const",
                    )));
                    continue;
                };
                self.expand_properties(spread_props, validator, result, depth + 1)?;
                continue;
            }
            let name = static_property_key(prop)
                .ok_or_else(|| self.error(prop, validator, "unsupported dynamic property; use a static key and value"))?;
            result.push(Ok((name, prop)));
        }
        Ok(())
    }

    fn call<'node>(
        &self,
        node: &'node JsonValue,
        name: &str,
        min: usize,
        max: usize,
    ) -> Result<&'node [JsonValue], ConvexTypeGeneratorError>
    {
        let args = node["arguments"]
            .as_array()
            .ok_or_else(|| self.error(node, name, "expected a call"))?;
        if args.len() < min || args.len() > max {
            let expected = if min == max {
                min.to_string()
            } else if max == usize::MAX {
                format!("at least {min}")
            } else {
                format!("{min} to {max}")
            };
            return Err(self.error(node, name, format!("expected {expected} argument(s), got {}", args.len())));
        }
        Ok(args)
    }
}

fn index_node_files<'a>(node: &'a JsonValue, file: &'a str, output: &mut BTreeMap<usize, &'a str>)
{
    output.insert(node as *const JsonValue as usize, file);
    match node {
        JsonValue::Array(values) => {
            for value in values {
                index_node_files(value, file, output);
            }
        }
        JsonValue::Object(values) => {
            for value in values.values() {
                index_node_files(value, file, output);
            }
        }
        _ => {}
    }
}

fn resolve_import_path<'a>(importer: &str, specifier: &str, modules: &'a BTreeMap<String, JsonValue>) -> Option<&'a String>
{
    let base = Path::new(importer).parent()?.join(specifier);
    let mut candidates = vec![base.clone()];
    if base.extension().is_none() {
        candidates.extend([base.with_extension("ts"), base.with_extension("tsx"), base.join("index.ts")]);
    }
    for candidate in candidates {
        let Ok(canonical) = candidate.canonicalize() else { continue };
        let key = canonical.to_string_lossy();
        if let Some((path, _)) = modules.get_key_value(key.as_ref()) {
            return Some(path);
        }
    }
    None
}

fn exported_const<'a>(body: &'a [JsonValue], name: &str) -> Option<&'a JsonValue>
{
    body.iter()
        .filter(|node| node["type"] == "ExportNamedDeclaration")
        .filter_map(|node| node["declaration"].get("declarations").and_then(JsonValue::as_array))
        .flatten()
        .find(|decl| decl["id"]["name"].as_str() == Some(name))
        .and_then(|decl| decl.get("init"))
}

fn static_property_key(node: &JsonValue) -> Option<&str>
{
    if !is_estree_object_property_like(node)
        || node["method"] == true
        || matches!(node["kind"].as_str(), Some("get" | "set"))
    {
        return None;
    }
    static_member_name(node, "key")
}

fn static_member_name<'a>(node: &'a JsonValue, key: &str) -> Option<&'a str>
{
    if node["computed"] == true && node[key]["type"] != "Literal" {
        return None;
    }
    member_property_name(&node[key])
}

fn validator_name(node: &JsonValue) -> Option<&str>
{
    let callee = &node["callee"];
    if node["type"] == "CallExpression" && callee["object"]["type"] == "Identifier" && callee["object"]["name"] == "v" {
        static_member_name(callee, "property")
    } else {
        None
    }
}

fn parse_table_shape(
    node: &JsonValue,
    source: &ValidatorSource<'_>,
    allow_raw_object: bool,
) -> Result<TableShape, ConvexTypeGeneratorError>
{
    let node = source.resolve(node)?;
    let (object, label) = match validator_name(node) {
        Some("union") => {
            let variants = source.call(node, "v.union", 1, usize::MAX)?;
            return Ok(TableShape::Union(
                variants
                    .iter()
                    .map(|variant| parse_table_shape(variant, source, false))
                    .collect::<Result<_, _>>()?,
            ));
        }
        Some("object") => (source.resolve(&source.call(node, "v.object", 1, 1)?[0])?, "v.object"),
        None if allow_raw_object && node["type"] == "ObjectExpression" => (node, "defineTable"),
        name => {
            return Err(source.error(
                node,
                name.map_or_else(|| "defineTable".into(), |name| format!("v.{name}")).as_str(),
                "expected a required v.object() or a v.union() of document objects",
            ));
        }
    };

    let mut fields = Vec::new();
    for property in source.properties(object, label)? {
        let (name, prop) = property?;
        fields.push(Field {
            name: name.to_string(),
            validator: parse_validator(&prop["value"], source)?,
            provenance: provenance(source.file, prop),
        });
    }
    Ok(TableShape::Object(fields))
}

/// Interpret an ESTree validator expression directly as a typed validator.
fn parse_validator(node: &JsonValue, source: &ValidatorSource<'_>) -> Result<Validator, ConvexTypeGeneratorError>
{
    let node = source.resolve(node)?;
    if node["type"] == "CallExpression"
        && static_member_name(&node["callee"], "property") == Some("optional")
        && node["callee"]["object"]["name"] != "v"
    {
        source.call(node, ".optional()", 0, 0)?;
        let inner = parse_validator(&node["callee"]["object"], source)?;
        return Ok(make_optional(inner));
    }

    let name = validator_name(node).ok_or_else(|| source.error(node, "validator", "expected a static v.* call"))?;
    let label = format!("v.{name}");
    let (min, max) = match name {
        "string" | "number" | "float64" | "boolean" | "null" | "int64" | "bigint" | "commitTs" | "bytes" | "any" => (0, 0),
        "id" | "literal" | "optional" | "nullable" | "array" | "object" => (1, 1),
        "record" => (2, 2),
        "union" => (1, usize::MAX),
        "function" => (0, 1),
        _ => return Err(source.error(node, &label, "unsupported validator; use a Convex 1.49.0 validator")),
    };
    let args = source.call(node, &label, min, max)?;
    let required = |arg: &JsonValue| {
        let validator = parse_validator(arg, source)?;
        if matches!(validator, Validator::Optional(_)) {
            return Err(source.error(
                arg,
                &label,
                "expected a required validator; put optionality on an object field",
            ));
        }
        Ok(validator)
    };

    Ok(match name {
        "string" => Validator::String,
        "number" | "float64" => Validator::Number,
        "boolean" => Validator::Boolean,
        "null" => Validator::Null,
        "int64" | "bigint" | "commitTs" => Validator::Int64,
        "bytes" => Validator::Bytes,
        "any" => Validator::Any,
        "id" => Validator::Id(Some(
            {
                let table = source.resolve(&args[0])?;
                table["value"].as_str().filter(|_| table["type"] == "Literal")
            }
            .ok_or_else(|| source.error(&args[0], &label, "expected a string literal table name"))?
            .to_string(),
        )),
        "literal" => Validator::Literal(Some(parse_literal(source.resolve(&args[0])?).ok_or_else(|| {
            source.error(
                &args[0],
                &label,
                "expected a string, number, signed 64-bit bigint, or boolean literal; use v.null() for null",
            )
        })?)),
        "optional" => {
            let inner = parse_validator(&args[0], source)?;
            make_optional(inner)
        }
        "nullable" => Validator::Union(vec![required(&args[0])?, Validator::Null]),
        "array" => {
            let elements = required(&args[0])?;
            Validator::Array(Box::new(elements))
        }
        "object" => parse_object_validator(source.resolve(&args[0])?, source, &label)?,
        "record" => {
            let key_type = required(&args[0])?;
            if !key_type.is_record_key() {
                return Err(source.error(
                    &args[0],
                    &label,
                    "keys must use strings, IDs, string literals, or unions of those validators",
                ));
            }
            Validator::Record {
                key_type: Box::new(key_type),
                value_type: Box::new(required(&args[1])?),
            }
        }
        "union" => Validator::Union(args.iter().map(required).collect::<Result<_, _>>()?),
        "function" => Validator::FunctionHandle(Box::new(parse_signature(args.first(), source)?)),
        _ => unreachable!("validator name was checked above"),
    })
}

fn make_optional(validator: Validator) -> Validator
{
    match validator {
        Validator::Optional(_) => validator,
        _ => Validator::Optional(Box::new(validator)),
    }
}

fn parse_object_validator(
    node: &JsonValue,
    source: &ValidatorSource<'_>,
    label: &str,
) -> Result<Validator, ConvexTypeGeneratorError>
{
    let mut properties = BTreeMap::new();
    for property in source.properties(node, label)? {
        let (name, prop) = property?;
        let validator = parse_validator(&prop["value"], source)?;
        properties.insert(name.to_string(), validator);
    }

    Ok(Validator::Object {
        properties: Some(properties),
    })
}

fn parse_signature(
    signature: Option<&JsonValue>,
    source: &ValidatorSource<'_>,
) -> Result<FunctionSignature, ConvexTypeGeneratorError>
{
    let mut result = FunctionSignature::default();
    let Some(signature) = signature.filter(|node| !source.is_undefined(node)) else {
        return Ok(result);
    };
    let label = "v.function";
    for property in source.properties(source.resolve(signature)?, label)? {
        let (name, prop) = property?;
        let value = &prop["value"];
        if source.is_undefined(value) && matches!(name, "fnType" | "args" | "returns") {
            continue;
        }
        match name {
            "fnType" => {
                let kind = source.resolve(value)?;
                result.kind = Some(match kind["value"].as_str().filter(|_| kind["type"] == "Literal") {
                    Some("query") => FunctionKind::Query,
                    Some("mutation") => FunctionKind::Mutation,
                    Some("action") => FunctionKind::Action,
                    _ => {
                        return Err(source.error(
                            value,
                            label,
                            "fnType must be a query, mutation, or action string literal",
                        ));
                    }
                })
            }
            "args" | "returns" => {
                let value = source.resolve(value)?;
                let validator = if value["type"] == "ObjectExpression" {
                    parse_object_validator(value, source, label)?
                } else {
                    parse_validator(value, source)?
                };
                if name == "args" {
                    result.args = Some(validator);
                } else {
                    result.returns = Some(validator);
                }
            }
            _ => return Err(source.error(prop, label, "expected fnType, args, or returns")),
        }
    }
    Ok(result)
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
        _ => None,
    }
}

/// Walk every module’s ESTree JSON and collect exported Convex endpoints.
pub(crate) fn parse_function_ast(
    ast_map: BTreeMap<String, JsonValue>,
    convex_root: Option<&Path>,
) -> Result<Vec<Function>, ConvexTypeGeneratorError>
{
    let mut convex_functions = Vec::new();

    for (source_path, ast) in &ast_map {
        let path = Path::new(&source_path);
        if matches!(
            path.file_name().and_then(|name| name.to_str()),
            Some("http.ts" | "crons.ts" | "convex.config.ts")
        ) || path
            .components()
            .any(|component| matches!(component.as_os_str().to_str(), Some("_generated" | "node_modules")))
            || path.to_string_lossy().ends_with(".d.ts")
        {
            continue;
        }
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
                context: source_path.clone(),
                details: "Missing body array".to_string(),
            })?;

        let input = ValidatorSource::new(source_path, body, Some(&ast_map));
        let builders = function_builder_bindings(body);
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
                                    context: source_path.clone(),
                                    details: "Missing function name".to_string(),
                                })?;

                        // Only imported Convex server builders define endpoints. Arbitrary calls
                        // and member calls are ordinary exported values.
                        let init = &declarator["init"];
                        if init["type"].as_str() == Some("CallExpression") {
                            let Some(fn_type) = init["callee"]["name"].as_str().and_then(|name| builders.get(name)) else {
                                continue;
                            };

                            // Get the first argument which contains the function config
                            if let Some(args) = init["arguments"].as_array()
                                && let Some(config) = args.first()
                            {
                                let (kind, visibility) = *fn_type;

                                convex_functions.push(Function {
                                    identity: FunctionIdentity {
                                        module: module.clone(),
                                        export: name.to_string(),
                                    },
                                    kind,
                                    visibility,
                                    args: extract_function_params(config, &input)?,
                                    returns: extract_return_validator(config, &input)?,
                                    provenance: provenance(source_path, init),
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

/// Local binding names for the recognized Convex function builders imported by this module.
fn function_builder_bindings(body: &[JsonValue]) -> BTreeMap<String, (FunctionKind, Visibility)>
{
    let mut bindings = BTreeMap::new();
    for node in body.iter().filter(|node| node["type"] == "ImportDeclaration") {
        let Some(module) = node["source"]["value"].as_str() else {
            continue;
        };
        if module != "convex/server" && !module.ends_with("/_generated/server") && module != "./_generated/server" {
            continue;
        }
        for specifier in node["specifiers"].as_array().into_iter().flatten() {
            if specifier["type"] != "ImportSpecifier" {
                continue;
            }
            let Some(imported) = specifier["imported"]["name"]
                .as_str()
                .or_else(|| specifier["imported"]["value"].as_str())
            else {
                continue;
            };
            let Some(local) = specifier["local"]["name"].as_str() else {
                continue;
            };
            let pair = match imported {
                "query" => (FunctionKind::Query, Visibility::Public),
                "mutation" => (FunctionKind::Mutation, Visibility::Public),
                "action" => (FunctionKind::Action, Visibility::Public),
                "internalQuery" => (FunctionKind::Query, Visibility::Internal),
                "internalMutation" => (FunctionKind::Mutation, Visibility::Internal),
                "internalAction" => (FunctionKind::Action, Visibility::Internal),
                _ => continue,
            };
            bindings.insert(local.to_string(), pair);
        }
    }
    bindings
}

/// ESTree / Oxc object literal member: either Babel-style `ObjectProperty` or spec `Property`.
fn is_estree_object_property_like(node: &JsonValue) -> bool
{
    matches!(node["type"].as_str(), Some("ObjectProperty") | Some("Property"))
}

/// Helper function to extract function parameters from the function configuration
fn extract_function_params(
    config: &JsonValue,
    source: &ValidatorSource<'_>,
) -> Result<Declared<Vec<Field>>, ConvexTypeGeneratorError>
{
    let Some(property) = config["properties"]
        .as_array()
        .and_then(|props| props.iter().find(|prop| static_property_key(prop) == Some("args")))
    else {
        return Ok(Declared::Unknown);
    };
    let value = source.resolve(&property["value"])?;
    let object = match validator_name(value) {
        Some("any") => {
            source.call(value, "v.any", 0, 0)?;
            return Ok(Declared::Unknown);
        }
        Some("object") => source.resolve(&source.call(value, "v.object", 1, 1)?[0])?,
        _ => value,
    };
    let mut params = Vec::new();
    for property in source.properties(object, "args")? {
        let (name, node) = property?;
        params.push(Field {
            name: name.to_string(),
            validator: parse_validator(&node["value"], source)?,
            provenance: provenance(source.file, node),
        });
    }
    Ok(Declared::Declared(params))
}

/// Unbound return validators remain unknown in the IR.
fn extract_return_validator(
    config: &JsonValue,
    source: &ValidatorSource<'_>,
) -> Result<Declared<ReturnValidator>, ConvexTypeGeneratorError>
{
    let Some(property) = config["properties"]
        .as_array()
        .and_then(|props| props.iter().find(|prop| static_property_key(prop) == Some("returns")))
    else {
        return Ok(Declared::Unknown);
    };
    let value = &property["value"];
    if value["type"] == "Identifier" && !source.bindings.contains_key(value["name"].as_str().unwrap_or_default()) {
        return Ok(Declared::Unknown);
    }
    let validator = parse_validator(value, source)?;
    Ok(Declared::Declared(ReturnValidator {
        validator,
        provenance: provenance(source.file, value),
    }))
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

    pub(super) fn object_fields(table: &super::TableDocument) -> &[super::Field]
    {
        let super::TableShape::Object(fields) = &table.shape else {
            panic!("expected object table");
        };
        fields
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
        assert_eq!(object_fields(&schema.tables[0]).len(), 1);
        assert_eq!(object_fields(&schema.tables[0])[0].name, "email");
        assert_eq!(object_fields(&schema.tables[0])[0].validator, Validator::String);
    }

    #[test]
    fn parses_single_string_column_through_chained_index()
    {
        let ast = export_default_define_schema(vec![table_with_index_chain(
            "users",
            vec![col("email", v_member_call("string", vec![]))],
        )]);
        let schema = parse_schema_ast(ast, Path::new("schema.ts")).unwrap();
        assert_eq!(object_fields(&schema.tables[0])[0].name, "email");
        assert_eq!(object_fields(&schema.tables[0])[0].validator, Validator::String);
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
            object_fields(&schema.tables[0])[0].validator,
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
                "type": "ImportDeclaration",
                "source": { "type": "Literal", "value": "convex/server" },
                "specifiers": [{
                    "type": "ImportSpecifier",
                    "imported": { "type": "Identifier", "name": "query" },
                    "local": { "type": "Identifier", "name": "query" }
                }]
            }, {
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
                                    "key": { "type": "Identifier", "name": "args" },
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
            extract_function_params(&config, &super::ValidatorSource::new("f.ts", &[], None)).unwrap(),
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
                "key": { "type": "Identifier", "name": "args" },
                "value": { "type": "Identifier", "name": "sharedArgs" }
            }]
        });
        let err = extract_function_params(&config, &super::ValidatorSource::new("f.ts", &[], None)).unwrap_err();
        assert!(matches!(err, ConvexTypeGeneratorError::InvalidSchema { .. }));
    }
}

#[cfg(test)]
mod oxc_ir_tests
{
    use std::fs;

    use tempfile::tempdir;

    use super::{parse_function_ast, parse_schema_ast};
    use crate::convex::ir::{Declared, FunctionKind, Provenance, Visibility};
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
            import { internalQuery, query, action } from "convex/server";
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
        let field = &super::parse_schema_ast_tests::object_fields(table)[0];
        assert!(matches!(field.validator, Validator::Optional(_)));
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

#[cfg(test)]
mod validator_tests;
