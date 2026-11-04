use std::fs;

use serde_json::json;
use tempfile::tempdir;

use super::*;
use crate::convex::{create_function_asts, create_schema_ast};

fn schema_from_source(source: &str) -> Result<Schema, ConvexTypeGeneratorError>
{
    let dir = tempdir().unwrap();
    let path = dir.path().join("schema.ts");
    fs::write(&path, source).unwrap();
    parse_schema_ast(create_schema_ast(&path).unwrap(), &path)
}

fn validator_from_source(source: &str) -> Result<Validator, ConvexTypeGeneratorError>
{
    let schema = schema_from_source(&format!(
        "export default defineSchema({{ t: defineTable({{ field: {source} }}) }});"
    ))?;
    let TableShape::Object(fields) = &schema.tables[0].shape else {
        panic!("expected object table");
    };
    Ok(fields[0].validator.clone())
}

#[test]
fn schema_adapter_resolves_imported_validator_objects_and_spreads()
{
    let dir = tempdir().unwrap();
    let schema = dir.path().join("schema.ts");
    fs::write(
        dir.path().join("fields.ts"),
        "import { v } from 'convex/values'; export const fields = { name: v.string() };",
    )
    .unwrap();
    fs::write(
        &schema,
        "import { defineSchema, defineTable } from 'convex/server'; import { v } from 'convex/values'; import { fields } \
         from './fields'; export default defineSchema({ users: defineTable({ ...fields, active: v.boolean() }) });",
    )
    .unwrap();
    let modules = create_function_asts(vec![schema.clone()]).unwrap();
    let schema_key = schema.canonicalize().unwrap().to_string_lossy().into_owned();
    let parsed = parse_schema_ast_with_modules(modules[&schema_key].clone(), &schema, Some(&modules)).unwrap();
    let TableShape::Object(fields) = &parsed.tables[0].shape else {
        panic!("expected object table")
    };
    assert_eq!(
        fields.iter().map(|field| field.name.as_str()).collect::<Vec<_>>(),
        ["name", "active"]
    );

    fs::write(
        dir.path().join("fields.ts"),
        "import { v } from 'convex/values'; export const fields = { name: v.unsupported() };",
    )
    .unwrap();
    let modules = create_function_asts(vec![schema.clone()]).unwrap();
    let schema_key = schema.canonicalize().unwrap().to_string_lossy().into_owned();
    let error = parse_schema_ast_with_modules(modules[&schema_key].clone(), &schema, Some(&modules))
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("fields.ts bytes ") && error.contains("v.unsupported"),
        "{error}"
    );
}

#[test]
fn lowers_every_factory_in_convex_1_49()
{
    let cases = [
        ("v.string()", Validator::String),
        ("v.number()", Validator::Number),
        ("v.float64()", Validator::Number),
        ("v.int64()", Validator::Int64),
        ("v.bigint()", Validator::Int64),
        ("v.commitTs()", Validator::Int64),
        ("v.boolean()", Validator::Boolean),
        ("v.null()", Validator::Null),
        ("v.bytes()", Validator::Bytes),
        ("v.any()", Validator::Any),
        ("v.id('t')", Validator::Id(Some("t".into()))),
        ("v.array(v.string())", Validator::Array(Box::new(Validator::String))),
        (
            "v.object({})",
            Validator::Object {
                struct_name: None,
                properties: Some(BTreeMap::new()),
            },
        ),
        (
            "v.record(v.string(), v.int64())",
            Validator::Record {
                key_type: Box::new(Validator::String),
                value_type: Box::new(Validator::Int64),
            },
        ),
        (
            "v.nullable(v.float64())",
            Validator::Union(vec![Validator::Number, Validator::Null]),
        ),
        (
            "v.union(v.number(), v.null())",
            Validator::Union(vec![Validator::Number, Validator::Null]),
        ),
        ("v.optional(v.string())", Validator::Optional(Box::new(Validator::String))),
        ("v.string().optional()", Validator::Optional(Box::new(Validator::String))),
        (
            "v.optional(v.string().optional())",
            Validator::Optional(Box::new(Validator::String)),
        ),
        ("v.function()", Validator::FunctionHandle(Box::default())),
        ("v.function(undefined)", Validator::FunctionHandle(Box::default())),
        ("v.function({ args: undefined })", Validator::FunctionHandle(Box::default())),
    ];

    for (source, expected) in cases {
        assert_eq!(validator_from_source(source).unwrap(), expected, "{source}");
    }
}

#[test]
fn keeps_literal_wire_kinds_and_values()
{
    for (source, expected) in [
        ("'123'", Literal::String("123".into())),
        (r#""a\nb""#, Literal::String("a\nb".into())),
        ("123", Literal::Float64(123.0)),
        ("123n", Literal::Int64(123)),
        ("false", Literal::Boolean(false)),
        ("true", Literal::Boolean(true)),
        ("0x7bn", Literal::Int64(123)),
        ("0b111_1011n", Literal::Int64(123)),
        ("-0x8000000000000000n", Literal::Int64(i64::MIN)),
        ("9_223_372_036_854_775_807n", Literal::Int64(i64::MAX)),
        ("-1.25e-10", Literal::Float64(-1.25e-10)),
        ("1.0000000000000002", Literal::Float64(1.0000000000000002)),
    ] {
        assert_eq!(
            validator_from_source(&format!("v.literal({source})")).unwrap(),
            Validator::Literal(Some(expected)),
            "{source}"
        );
    }
    let Validator::Literal(Some(Literal::Float64(zero))) = validator_from_source("v.literal(-0)").unwrap() else {
        panic!("expected float literal");
    };
    assert_eq!(zero.to_bits(), (-0.0_f64).to_bits());
}

#[test]
fn parses_opaque_handles_with_partial_or_complete_signature_metadata()
{
    let Validator::FunctionHandle(signature) = validator_from_source(
        r#"v.function({ fnType: "mutation", args: { "$weird": v.int64() }, returns: v.nullable(v.string()) })"#,
    )
    .unwrap() else {
        panic!("expected a function handle");
    };
    assert_eq!(signature.kind, Some(FunctionKind::Mutation));
    assert_eq!(
        signature.args,
        Some(Validator::Object {
            struct_name: None,
            properties: Some(BTreeMap::from([("$weird".into(), Validator::Int64)])),
        })
    );
    assert_eq!(
        signature.returns,
        Some(Validator::Union(vec![Validator::String, Validator::Null]))
    );
    assert_eq!(
        validator_from_source(r#"v.function({ fnType: "query" })"#).unwrap(),
        Validator::FunctionHandle(Box::new(FunctionSignature {
            kind: Some(FunctionKind::Query),
            ..Default::default()
        })),
    );
}

#[test]
fn accepts_string_and_id_record_keys_including_unions()
{
    assert_eq!(
        validator_from_source("v.record(v.union(v.string(), v.id('t')), v.null())").unwrap(),
        Validator::Record {
            key_type: Box::new(Validator::Union(vec![Validator::String, Validator::Id(Some("t".into()))])),
            value_type: Box::new(Validator::Null),
        },
    );
}

#[test]
fn accepts_indexed_union_tables_and_reuses_local_shapes()
{
    let schema = schema_from_source(
        r#"
        const kind = v.literal("user");
        const columns = { "my-field": kind, "$weird": v.object({ "inner-key": v.boolean() }) };
        const user = v.object(columns);
        const other = v.object({ "my-field": v.number() });
        const either = v.union(user, other);
        export default defineSchema({
            "object-table": defineTable(user),
            raw: defineTable(columns),
            mixed: defineTable(either)
                .index("field", ["my-field"])
                .searchIndex("search", { searchField: "text" })
                .vectorIndex("vec", { vectorField: "embedding", dimensions: 2 }),
            inline: defineTable(v.union(user, other)),
        });
    "#,
    )
    .unwrap();
    assert_eq!(schema.tables[0].name, "object-table");
    let TableShape::Object(fields) = &schema.tables[0].shape else {
        panic!("expected object table");
    };
    assert_eq!(fields[0].name, "my-field");
    assert_eq!(fields[0].validator, Validator::Literal(Some(Literal::String("user".into()))));
    assert_eq!(fields[1].name, "$weird");
    assert!(
        matches!(&fields[1].validator, Validator::Object { properties: Some(props), .. }
        if props["inner-key"] == Validator::Boolean)
    );
    assert!(matches!(&schema.tables[1].shape, TableShape::Object(fields) if fields.len() == 2));
    let TableShape::Union(variants) = &schema.tables[2].shape else {
        panic!("expected union table");
    };
    let [TableShape::Object(user), TableShape::Object(other)] = variants.as_slice() else {
        panic!("expected two object variants");
    };
    assert_eq!(user[0].validator, fields[0].validator);
    assert_eq!(other[0].validator, Validator::Number);
    assert!(matches!(&schema.tables[3].shape, TableShape::Union(variants) if variants.len() == 2));
}

#[test]
fn rejects_malformed_shapes_with_source_span_and_validator_name()
{
    for (expression, validator, details) in [
        ("v.future()", "v.future", "unsupported validator"),
        ("v.array()", "v.array", "expected 1 argument"),
        ("v.id(123)", "v.id", "string literal table name"),
        ("v.object(false)", "v.object", "expected an object literal"),
        ("v.object({ ...other })", "v.object", "unsupported dynamic object spread"),
        ("v.literal(null)", "v.literal", "use v.null() for null"),
        ("v.literal(9223372036854775808n)", "v.literal", "signed 64-bit bigint"),
        ("v.union()", "v.union", "expected at least 1 argument"),
        ("v.nullable(v.optional(v.string()))", "v.nullable", "required validator"),
        ("v.record(v.number(), v.string())", "v.record", "keys must use"),
        (
            "v.record(v.union(v.id('t'), v.int64()), v.string())",
            "v.record",
            "keys must use",
        ),
        (
            "v.record(v.optional(v.string()), v.string())",
            "v.record",
            "required validator",
        ),
        (
            "v.record(v.string(), v.optional(v.int64()))",
            "v.record",
            "required validator",
        ),
        ("v.record(v.string())", "v.record", "expected 2 argument"),
        ("v.function({ fnType: 'invalid' })", "v.function", "fnType must be"),
    ] {
        let diagnostic = validator_from_source(expression).unwrap_err().to_string();
        for expected in ["schema.ts bytes ", validator, details] {
            assert!(diagnostic.contains(expected), "{expression}: {diagnostic}");
        }
    }
    for (declaration, expression, details) in [
        ("", "defineTable(v.union(v.object({}), v.string()))", "required v.object()"),
        ("", "defineTable(missing)", "same-file const binding"),
        (
            "const cycle = v.object({ x: cycle });",
            "defineTable(cycle)",
            "cyclic validator reference",
        ),
    ] {
        let diagnostic = schema_from_source(&format!("{declaration} export default defineSchema({{ t: {expression} }});"))
            .unwrap_err()
            .to_string();
        assert!(
            diagnostic.contains("schema.ts bytes ") && diagnostic.contains(details),
            "{diagnostic}"
        );
    }
}

#[test]
fn preserves_quoted_function_args_and_reports_unsupported_returns()
{
    let dir = tempdir().unwrap();
    let path = dir.path().join("functions.ts");
    fs::write(
        &path,
        r#"import { query } from "convex/server"; export const q = query({ "args": { "my-field": v.number(), "$weird": v.int64() } });"#,
    )
    .unwrap();
    let read = || parse_function_ast(create_function_asts(vec![path.clone()]).unwrap(), Some(dir.path()));
    let functions = read().unwrap();
    let fields = functions[0].params();
    assert_eq!(
        fields.iter().map(|f| f.name.as_str()).collect::<Vec<_>>(),
        ["my-field", "$weird"]
    );
    fs::write(
        &path,
        "import { query } from 'convex/server'; export const q = query({ returns: v.future() });",
    )
    .unwrap();
    let diagnostic = read().unwrap_err().to_string();
    assert!(
        diagnostic.contains("functions.ts bytes ") && diagnostic.contains("v.future"),
        "{diagnostic}"
    );
}

#[test]
fn recognizes_only_imported_convex_builders_and_preserves_root_relative_identity()
{
    let dir = tempdir().unwrap();
    let root = dir.path().join("convex");
    fs::create_dir_all(root.join("admin")).unwrap();
    let root = root.canonicalize().unwrap();
    let root_module = root.join("users.ts");
    let nested_module = root.join("admin/users.ts");
    let outside_module = dir.path().join("outside.ts");
    fs::write(
        &root_module,
        r#"import { query as q } from "convex/server";
export const fake = helper({ args: { ignored: v.string() } });
export const member = api.query({ args: { ignored: v.string() } });
export const list = q({ args: {} });"#,
    )
    .unwrap();
    fs::write(
        &nested_module,
        r#"import { query as q } from "./_generated/server";
export const list = q({ args: { name: v.string() } });"#,
    )
    .unwrap();
    fs::write(
        &outside_module,
        "import { action } from 'convex/server'; export const run = action({ args: {} });",
    )
    .unwrap();
    let functions = parse_function_ast(
        create_function_asts(vec![root_module, nested_module, outside_module]).unwrap(),
        Some(&root),
    )
    .unwrap();
    assert_eq!(functions.len(), 3);
    assert_eq!(functions[0].identity.module, Some(vec!["admin".into(), "users".into()]));
    assert_eq!(functions[1].identity.module, Some(vec!["users".into()]));
    assert_eq!(functions[0].identity.export, "list");
    assert_eq!(functions[1].args, Declared::Declared(vec![]));
    assert_eq!(functions[2].identity.module, None);
}

/// Project only the shared validator metadata used by the fixture comparison into SDK JSON.
fn spec_validator(validator: &Validator) -> JsonValue
{
    match validator {
        Validator::Id(Some(table)) => json!({ "type": "id", "tableName": table }),
        Validator::Array(element) => json!({ "type": "array", "value": spec_validator(element) }),
        Validator::Object {
            properties: Some(fields),
            ..
        } => spec_object(fields.iter().map(|(key, value)| (key.as_str(), value))),
        Validator::Literal(Some(Literal::String(value))) => json!({ "type": "literal", "value": value }),
        Validator::Boolean | Validator::String | Validator::Null | Validator::Number => {
            json!({ "type": validator.type_name() })
        }
        _ => panic!("unexpected fixture validator {validator:?}"),
    }
}

fn spec_object<'a>(fields: impl Iterator<Item = (&'a str, &'a Validator)>) -> JsonValue
{
    let fields: serde_json::Map<_, _> = fields
        .map(|(name, validator)| {
            let (inner, optional) = match validator {
                Validator::Optional(inner) => (inner.as_ref(), true),
                _ => (validator, false),
            };
            (
                name.to_string(),
                json!({ "fieldType": spec_validator(inner), "optional": optional }),
            )
        })
        .collect();
    json!({ "type": "object", "value": fields })
}

#[test]
fn review_fixture_matches_function_spec_for_inline_validators()
{
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/v1-review/convex");
    let schema = parse_schema_ast(create_schema_ast(&root.join("schema.ts")).unwrap(), &root.join("schema.ts")).unwrap();
    assert!(
        matches!(&schema.tables.iter().find(|table| table.name == "events").unwrap().shape,
        TableShape::Union(variants) if variants.len() == 2)
    );

    let asts = create_function_asts(vec![
        root.join("users.ts"),
        root.join("admin/users.ts"),
        root.join("validators.ts"),
    ])
    .unwrap();
    let functions = parse_function_ast(asts, Some(&root)).unwrap();
    let snapshot = functions
        .iter()
        .map(|function| {
            let module = function.identity.module.as_ref().unwrap().join("/");
            let args = match &function.args {
                Declared::Declared(fields) => format!(
                    "declared[{}]",
                    fields.iter().map(|field| field.name.as_str()).collect::<Vec<_>>().join(",")
                ),
                Declared::Unknown => "unknown".to_string(),
            };
            let returns = match &function.returns {
                Declared::Declared(_) => "declared",
                Declared::Unknown => "unknown",
            };
            format!(
                "{module}:{} {:?} {:?} args={args} returns={returns}",
                function.identity.export, function.kind, function.visibility
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    insta::assert_snapshot!(snapshot);
    let echo = functions.iter().find(|function| function.identity.export == "echo").unwrap();
    let Declared::Declared(echo_args) = &echo.args else {
        panic!("expected declared echo args")
    };
    let Validator::Object {
        properties: Some(echo_fields),
        ..
    } = &echo_args[0].validator
    else {
        panic!("expected imported payload object")
    };
    assert!(echo_fields.contains_key("count"));
    assert!(echo_fields.contains_key("number"));
    assert!(echo_fields.contains_key("bytes"));
    let compared = functions
        .iter()
        .filter(|function| function.identity.export != "echo")
        .collect::<Vec<_>>();
    let spec: JsonValue =
        serde_json::from_str(include_str!("../../../tests/fixtures/v1-review/inline-function-spec.json")).unwrap();
    let spec = spec.as_array().unwrap();
    assert_eq!(compared.len(), spec.len());
    for function in compared {
        let module = function.identity.module.as_ref().unwrap().join("/");
        let identifier = format!("{module}.js:{}", function.identity.export);
        let expected = spec.iter().find(|entry| entry["identifier"] == identifier).unwrap();
        let args = match &function.args {
            Declared::Declared(fields) => spec_object(fields.iter().map(|field| (field.name.as_str(), &field.validator))),
            Declared::Unknown => json!({ "type": "any" }),
        };
        let returns = match &function.returns {
            Declared::Declared(returns) => spec_validator(&returns.validator),
            Declared::Unknown => JsonValue::Null,
        };
        assert_eq!(args, expected["args"], "{identifier} args");
        assert_eq!(returns, expected["returns"], "{identifier} returns");
    }
}
