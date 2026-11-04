//! Adapter for the committed JSON emitted by `convex function-spec`.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde_json::Value;

use crate::convex::ir::{
    Declared, Field, Function, FunctionIdentity, FunctionKind, Provenance, ReturnValidator, Visibility,
};
use crate::convex::validator::{ConvexValidator as Validator, Literal};
use crate::error::ConvexTypeGeneratorError;

fn invalid(file: &Path, context: &str, message: impl Into<String>) -> ConvexTypeGeneratorError
{
    ConvexTypeGeneratorError::InvalidSchema {
        context: format!("{} ({context})", file.display()),
        details: message.into(),
    }
}

fn string<'a>(value: &'a Value, file: &Path, context: &str, key: &str) -> Result<&'a str, ConvexTypeGeneratorError>
{
    value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| invalid(file, context, format!("missing string `{key}`")))
}

fn parse_validator(value: &Value, file: &Path, context: &str) -> Result<Validator, ConvexTypeGeneratorError>
{
    let kind = string(value, file, context, "type")?;
    let child = |key: &str| -> Result<Validator, ConvexTypeGeneratorError> {
        let value = value
            .get(key)
            .ok_or_else(|| invalid(file, context, format!("`{kind}` is missing `{key}`")))?;
        parse_validator(value, file, &format!("{context}.{key}"))
    };
    Ok(match kind {
        "string" => Validator::String,
        "number" => Validator::Number,
        "boolean" => Validator::Boolean,
        "null" => Validator::Null,
        "bigint" => Validator::Int64,
        "bytes" => Validator::Bytes,
        "any" => Validator::Any,
        "id" => {
            let table = value
                .get("tableName")
                .and_then(Value::as_str)
                .filter(|table| !table.is_empty())
                .ok_or_else(|| invalid(file, context, "id validator requires a non-empty string `tableName`"))?;
            Validator::Id(Some(table.to_owned()))
        }
        "array" => Validator::Array(Box::new(child("value")?)),
        "object" => {
            let fields = value
                .get("value")
                .and_then(Value::as_object)
                .ok_or_else(|| invalid(file, context, "object `value` must be a field map"))?;
            let mut properties = BTreeMap::new();
            for (name, field) in fields {
                let field_context = format!("{context}.{name}");
                let optional = field
                    .get("optional")
                    .and_then(Value::as_bool)
                    .ok_or_else(|| invalid(file, &field_context, "field is missing boolean `optional`"))?;
                let field_type = field
                    .get("fieldType")
                    .ok_or_else(|| invalid(file, &field_context, "field is missing `fieldType`"))?;
                let parsed = parse_validator(field_type, file, &field_context)?;
                properties.insert(
                    name.clone(),
                    if optional {
                        Validator::Optional(Box::new(parsed))
                    } else {
                        parsed
                    },
                );
            }
            Validator::Object {
                struct_name: None,
                properties: Some(properties),
            }
        }
        "record" => {
            let key_type = child("keys")?;
            let values = value
                .get("values")
                .ok_or_else(|| invalid(file, context, "record is missing `values`"))?;
            // Convex 1.49 serializes record values with the same field envelope as object fields.
            let value_type = if let Some(field_type) = values.get("fieldType") {
                let optional = values
                    .get("optional")
                    .and_then(Value::as_bool)
                    .ok_or_else(|| invalid(file, context, "record value is missing boolean `optional`"))?;
                let parsed = parse_validator(field_type, file, &format!("{context}.values"))?;
                if optional {
                    Validator::Optional(Box::new(parsed))
                } else {
                    parsed
                }
            } else {
                parse_validator(values, file, &format!("{context}.values"))?
            };
            Validator::Record {
                key_type: Box::new(key_type),
                value_type: Box::new(value_type),
            }
        }
        "union" => {
            let members = value
                .get("value")
                .and_then(Value::as_array)
                .ok_or_else(|| invalid(file, context, "union `value` must be an array"))?;
            if members.is_empty() {
                return Err(invalid(file, context, "union must contain at least one validator"));
            }
            Validator::Union(
                members
                    .iter()
                    .enumerate()
                    .map(|(i, item)| parse_validator(item, file, &format!("{context}.union[{i}]")))
                    .collect::<Result<_, _>>()?,
            )
        }
        "literal" => {
            let literal = value
                .get("value")
                .ok_or_else(|| invalid(file, context, "literal is missing `value`"))?;
            let literal = if let Some(s) = literal.as_str() {
                Literal::String(s.to_owned())
            } else if let Some(b) = literal.as_bool() {
                Literal::Boolean(b)
            } else if let Some(n) = literal.as_f64() {
                Literal::Float64(n)
            } else {
                return Err(invalid(file, context, "literal value must be string, boolean, or number"));
            };
            Validator::Literal(Some(literal))
        }
        unsupported => return Err(invalid(file, context, format!("unsupported validator type `{unsupported}`"))),
    })
}

fn parse_function(entry: &Value, file: &Path, index: usize) -> Result<Option<Function>, ConvexTypeGeneratorError>
{
    let entry_context = format!("functions[{index}]");
    let kind_name = string(entry, file, &entry_context, "functionType")?;
    if kind_name == "HttpAction" {
        return Ok(None);
    }
    let context = string(entry, file, &entry_context, "identifier")?;
    let (module_part, export) = context
        .rsplit_once(':')
        .ok_or_else(|| invalid(file, context, "identifier must be `<module>.js:<export>`"))?;
    if context.matches(':').count() != 1 || export.is_empty() || module_part.is_empty() || export.contains('/') {
        return Err(invalid(file, context, "invalid function identity"));
    }
    let module_part = module_part
        .strip_suffix(".js")
        .ok_or_else(|| invalid(file, context, "module identifier must end in `.js`"))?;
    let segments: Vec<String> = module_part.split('/').map(str::to_owned).collect();
    if segments.iter().any(|s| s.is_empty() || s == "." || s == "..") {
        return Err(invalid(file, context, "module path contains an invalid segment"));
    }
    let kind = match kind_name {
        "Query" => FunctionKind::Query,
        "Mutation" => FunctionKind::Mutation,
        "Action" => FunctionKind::Action,
        other => return Err(invalid(file, context, format!("unsupported function type `{other}`"))),
    };
    let visibility = match entry.get("visibility").and_then(|v| v.get("kind")).and_then(Value::as_str) {
        Some("public") => Visibility::Public,
        Some("internal") => Visibility::Internal,
        Some(other) => return Err(invalid(file, context, format!("unsupported visibility `{other}`"))),
        None => return Err(invalid(file, context, "missing visibility.kind")),
    };
    let args_value = entry.get("args").ok_or_else(|| invalid(file, context, "missing `args`"))?;
    // The CLI emits `any` for omitted args and cannot distinguish that from an explicit top-level
    // `v.any()` declaration, so both use the IR's unknown case.
    let args = if args_value.get("type").and_then(Value::as_str) == Some("any") {
        Declared::Unknown
    } else {
        let parsed = parse_validator(args_value, file, &format!("{context}.args"))?;
        match parsed {
            Validator::Object {
                properties: Some(fields),
                ..
            } => Declared::Declared(
                fields
                    .into_iter()
                    .map(|(name, validator)| Field {
                        name,
                        validator,
                        provenance: Provenance {
                            source: file.display().to_string(),
                            span: None,
                        },
                    })
                    .collect(),
            ),
            _ => return Err(invalid(file, context, "top-level args must be `any` or an object")),
        }
    };
    let returns_value = entry
        .get("returns")
        .ok_or_else(|| invalid(file, context, "missing `returns`"))?;
    let returns = if returns_value.is_null() {
        Declared::Unknown
    } else {
        Declared::Declared(ReturnValidator {
            validator: parse_validator(returns_value, file, &format!("{context}.returns"))?,
            provenance: Provenance {
                source: file.display().to_string(),
                span: None,
            },
        })
    };
    Ok(Some(Function {
        identity: FunctionIdentity {
            module: Some(segments),
            export: export.to_owned(),
        },
        legacy_module_name: module_part.rsplit('/').next().unwrap_or(module_part).to_owned(),
        kind,
        visibility,
        args,
        returns,
        provenance: Provenance {
            source: format!("{}#{context}", file.display()),
            span: None,
        },
    }))
}

pub(crate) fn parse_function_spec(path: &Path) -> Result<Vec<Function>, ConvexTypeGeneratorError>
{
    let text = fs::read_to_string(path).map_err(|error| ConvexTypeGeneratorError::IOError {
        file: path.display().to_string(),
        error,
    })?;
    let root: Value =
        serde_json::from_str(&text).map_err(|error| invalid(path, "root", format!("invalid JSON: {error}")))?;
    let entries = root
        .as_array()
        .or_else(|| root.get("functions").and_then(Value::as_array))
        .ok_or_else(|| invalid(path, "root", "expected a function array or object with `functions` array"))?;
    let mut functions = Vec::new();
    let mut identities = std::collections::BTreeSet::new();
    for (index, entry) in entries.iter().enumerate() {
        if let Some(function) = parse_function(entry, path, index)? {
            let key = (function.identity.module.clone(), function.identity.export.clone());
            if !identities.insert(key) {
                return Err(invalid(path, &function.provenance.source, "duplicate function identity"));
            }
            functions.push(function);
        }
    }
    functions.sort_by(|a, b| {
        a.identity
            .module
            .cmp(&b.identity.module)
            .then(a.identity.export.cmp(&b.identity.export))
    });
    Ok(functions)
}

#[cfg(test)]
mod tests
{
    use std::fs;

    use tempfile::tempdir;

    use super::parse_function_spec;
    use crate::convex::ir::{Declared, FunctionKind, Visibility};
    use crate::convex::validator::{ConvexValidator as Validator, Literal};

    fn read_spec(contents: &str) -> Result<Vec<crate::convex::ir::Function>, crate::error::ConvexTypeGeneratorError>
    {
        let dir = tempdir().unwrap();
        let path = dir.path().join("function-spec.json");
        fs::write(&path, contents).unwrap();
        parse_function_spec(&path)
    }

    #[test]
    fn lowers_nested_identity_metadata_and_unknowns()
    {
        let mut functions = read_spec(r#"[{"identifier":"admin/users.js:list","functionType":"Query","visibility":{"kind":"internal"},"args":{"type":"any"},"returns":null},{"identifier":"users.js:nullValue","functionType":"Mutation","visibility":{"kind":"public"},"args":{"type":"object","value":{}},"returns":{"type":"null"}}]"#).unwrap();
        assert_eq!(functions.len(), 2);
        assert_eq!(
            functions[0]
                .identity
                .module
                .as_deref()
                .unwrap()
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            ["admin", "users"]
        );
        assert_eq!(functions[0].kind, FunctionKind::Query);
        assert_eq!(functions[0].visibility, Visibility::Internal);
        assert!(matches!(functions[0].args, Declared::Unknown));
        assert!(matches!(functions[0].returns, Declared::Unknown));
        assert!(matches!(functions[1].args, Declared::Declared(ref fields) if fields.is_empty()));
        assert!(matches!(functions[1].returns, Declared::Declared(ref r) if r.validator == Validator::Null));
        for function in &mut functions {
            function.provenance.source = "function-spec.json".into();
            if let Declared::Declared(fields) = &mut function.args {
                for field in fields {
                    field.provenance.source = "function-spec.json".into();
                }
            }
            if let Declared::Declared(returns) = &mut function.returns {
                returns.provenance.source = "function-spec.json".into();
            }
        }
        let shape: Vec<_> = functions
            .iter()
            .map(|f| {
                (
                    f.identity.module.as_ref().unwrap().join("/"),
                    f.identity.export.clone(),
                    f.kind,
                    f.visibility,
                    f.args.clone(),
                    f.returns.clone(),
                )
            })
            .collect();
        insta::assert_debug_snapshot!("function-spec-ir", shape);
    }

    #[test]
    fn lowers_record_optional_and_exact_literal_types()
    {
        let functions = read_spec(r#"[{"identifier":"users.js:shape","functionType":"Query","visibility":{"kind":"public"},"args":{"type":"object","value":{"record":{"fieldType":{"type":"record","keys":{"type":"string"},"values":{"type":"bigint"}},"optional":false},"tag":{"fieldType":{"type":"literal","value":"a"},"optional":true}}},"returns":{"type":"boolean"}}]"#).unwrap();
        let fields = functions[0].params();
        assert!(matches!(fields[0].validator, Validator::Record { .. }));
        assert!(
            matches!(fields[1].validator, Validator::Optional(ref inner) if **inner == Validator::Literal(Some(Literal::String("a".into()))))
        );
    }

    #[test]
    fn skips_http_actions_and_rejects_bad_or_duplicate_entries()
    {
        let funcs = read_spec(r#"[{"functionType":"HttpAction","path":"/hook","method":"POST"}]"#).unwrap();
        assert!(funcs.is_empty());
        let error = read_spec(r#"[{"identifier":"users.js:f","functionType":"Query","visibility":{"kind":"public"},"args":{"type":"symbol"},"returns":null}]"#).unwrap_err().to_string();
        assert!(error.contains("function-spec.json") && error.contains("unsupported validator type `symbol`"));
        let dup = r#"[{"identifier":"users.js:f","functionType":"Query","visibility":{"kind":"public"},"args":{"type":"any"},"returns":null},{"identifier":"users.js:f","functionType":"Query","visibility":{"kind":"public"},"args":{"type":"any"},"returns":null}]"#;
        assert!(
            read_spec(dup)
                .unwrap_err()
                .to_string()
                .contains("duplicate function identity")
        );
        let bad_id = r#"[{"identifier":"users.js:f","functionType":"Query","visibility":{"kind":"public"},"args":{"type":"object","value":{"task":{"fieldType":{"type":"id"},"optional":false}}},"returns":null}]"#;
        let message = read_spec(bad_id).unwrap_err().to_string();
        assert!(message.contains("users.js:f.args.task") && message.contains("requires a non-empty string `tableName`"));
        let missing_identity =
            r#"[{"functionType":"Query","visibility":{"kind":"public"},"args":{"type":"any"},"returns":null}]"#;
        assert!(read_spec(missing_identity).unwrap_err().to_string().contains("functions[0]"));
    }

    #[test]
    fn lowers_the_sanitized_local_function_spec_capture()
    {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/v1-review/function-spec.json");
        let mut functions = parse_function_spec(&path).unwrap();
        assert_eq!(functions.len(), 12, "HTTP actions are not callable functions");
        let echo = functions.iter().find(|f| f.identity.export == "echo").unwrap();
        assert!(
            matches!(echo.params().first().map(|f| &f.validator), Some(Validator::Object { .. })),
            "imported validator must be resolved by the capture"
        );
        assert!(functions.iter().any(|f| {
            f.identity
                .module
                .as_ref()
                .unwrap()
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>()
                == ["admin", "users"]
        }));
        assert!(functions.iter().any(|f| f.visibility == Visibility::Internal));

        for function in &mut functions {
            function.provenance.source = "function-spec.json".into();
            if let Declared::Declared(fields) = &mut function.args {
                for field in fields {
                    field.provenance.source = "function-spec.json".into();
                }
            }
            if let Declared::Declared(returns) = &mut function.returns {
                returns.provenance.source = "function-spec.json".into();
            }
        }
        let shape: Vec<_> = functions
            .iter()
            .map(|f| {
                (
                    f.identity.module.as_ref().unwrap().join("/"),
                    f.identity.export.clone(),
                    f.kind,
                    f.visibility,
                    f.args.clone(),
                    f.returns.clone(),
                )
            })
            .collect();
        insta::assert_debug_snapshot!("v1-function-spec-capture-ir", shape);
    }
}
