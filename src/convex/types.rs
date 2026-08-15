//! Intermediate representation between [`crate::convex::parser`] and [`crate::convex::codegen`].
//!
//! Table and function metadata are strongly typed; **column / parameter validators** stay as
//! [`serde_json::Value`] trees with a `"type"` string discriminator and optional `"structName"` on
//! `v.object`. See [`crate::convex::validator::ConvexValidator`] for the typed IR used in codegen dedupe.

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

/// Parsed `defineSchema` body: ordered list of tables.
///
/// A schema can contain many tables. https://docs.convex.dev/database/schemas
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct ConvexSchema
{
    pub tables: Vec<ConvexTable>,
}

/// A table in the convex schema.
///
/// A table can contain many columns.
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct ConvexTable
{
    /// The name of the table.
    pub name: String,
    /// The columns in the table.
    pub columns: Vec<ConvexColumn>,
}

/// A column in the convex schema.
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct ConvexColumn
{
    /// The name of the column.
    pub name: String,
    /// Normalized `v.*` subtree (see module docs).
    pub data_type: JsonValue,
}

/// A collection of all convex functions.
pub(crate) type ConvexFunctions = Vec<ConvexFunction>;

/// One exported `query`, `mutation`, or `action` binding.
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct ConvexFunction
{
    /// Rust-friendly export name (`export const foo = …`).
    pub name: String,
    pub params: Vec<ConvexFunctionParam>,
    /// Callee identifier from TS: `"query"`, `"mutation"`, or `"action"` (informational for now).
    pub type_: String,
    /// Convex module path segment: file stem without `.ts` (`api` for `convex/api.ts`).
    pub file_name: String,
}

/// A parameter in a convex function.
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct ConvexFunctionParam
{
    pub name: String,
    pub data_type: JsonValue,
}

#[cfg(test)]
mod serde_roundtrip_tests
{
    use serde_json::json;

    use super::{ConvexColumn, ConvexFunction, ConvexFunctionParam, ConvexSchema, ConvexTable};

    #[test]
    fn schema_tables_columns_roundtrip()
    {
        let schema = ConvexSchema {
            tables: vec![ConvexTable {
                name: "t".into(),
                columns: vec![ConvexColumn {
                    name: "c".into(),
                    data_type: json!({ "type": "string" }),
                }],
            }],
        };
        let s = serde_json::to_string(&schema).unwrap();
        let back: ConvexSchema = serde_json::from_str(&s).unwrap();
        assert_eq!(back.tables.len(), 1);
        assert_eq!(back.tables[0].columns[0].data_type["type"], "string");
    }

    #[test]
    fn convex_function_roundtrip()
    {
        let f = ConvexFunction {
            name: "q".into(),
            params: vec![ConvexFunctionParam {
                name: "x".into(),
                data_type: json!({ "type": "number" }),
            }],
            type_: "query".into(),
            file_name: "api".into(),
        };
        let s = serde_json::to_string(&f).unwrap();
        let back: ConvexFunction = serde_json::from_str(&s).unwrap();
        assert_eq!(back.type_, "query");
        assert_eq!(back.file_name, "api");
    }
}
