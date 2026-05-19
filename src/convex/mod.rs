//! Oxc → ESTree JSON → ad-hoc JSON traversal → Rust text.
//!
//! This module is intentionally private: the stable surface is [`crate::generate`], [`crate::config`],
//! and [`crate::prelude`]. What lives here:
//!
//! - **Lexer** ([`lexer`]) — parse TS to a [`serde_json::Value`] ESTree tree.
//! - **Parser** ([`parser`]) — interpret Convex `defineSchema` / `defineTable` / `v.*` and exported
//!   `query`/`mutation`/`action` shapes into [`types`].
//! - **Codegen** ([`codegen`]) — stringify that model as `src/convex_types.rs`-style output.
//!
//! ## JSON at the boundaries
//!
//! Column and argument types stay as [`serde_json::Value`] in [`types`] instead of a dedicated Rust
//! ADT: the parser’s output mirrors the nested `v.union`, `v.optional`, etc. tree closely, which keeps
//! codegen’s pattern matching aligned with how Convex encodes validators in TS.
//!
//! ## Client helpers
//!
//! With the **`client`** feature, [`IntoConvexValue`], [`ConvexValueExt`], and [`ConvexClientExt`]
//! bridge `serde_json` and the official Convex client crate.

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde_json::Value as JsonValue;

use crate::convex::lexer::generate_javascript_ast;
use crate::error::ConvexTypeGeneratorError;

#[cfg(feature = "client")]
mod client;
pub(crate) mod codegen;
pub(crate) mod lexer;
pub(crate) mod parser;
pub(crate) mod types;
pub(crate) mod utils;

#[cfg(feature = "client")]
pub use client::{ConvexClientExt, ConvexValueExt, IntoConvexValue};

/// Parse `schema.ts` through Oxc; returns the ESTree program as JSON ([`lexer::generate_javascript_ast`]).
pub(crate) fn create_schema_ast(path: PathBuf) -> Result<JsonValue, ConvexTypeGeneratorError>
{
    // Validate path exists before processing
    if !path.exists() {
        return Err(ConvexTypeGeneratorError::MissingSchemaFile);
    }

    generate_javascript_ast(&path)
}

/// Parse every function source path; keys are **canonical** path strings (UTF-8 lossy).
///
/// [`BTreeMap`] keeps iteration order deterministic for stable codegen. Basenames alone would collide
/// for `convex/a/foo.ts` vs `convex/b/foo.ts`; full canonical keys keep ASTs separate while
/// [`parser::parse_function_ast`] still derives the Convex module segment from the file name.
pub(crate) fn create_function_asts(paths: Vec<PathBuf>) -> Result<BTreeMap<String, JsonValue>, ConvexTypeGeneratorError>
{
    let mut function_asts = BTreeMap::new();

    for path in paths {
        let path_str = path.to_string_lossy().to_string();
        let canonical = path.canonicalize().map_err(|error| ConvexTypeGeneratorError::IOError {
            file: path_str.clone(),
            error,
        })?;

        let key = canonical.to_string_lossy().to_string();
        canonical
            .file_name()
            .ok_or_else(|| ConvexTypeGeneratorError::InvalidPath(key.clone()))?
            .to_str()
            .ok_or_else(|| ConvexTypeGeneratorError::InvalidUnicode(key.clone()))?;

        let function_ast = generate_javascript_ast(&canonical)?;
        function_asts.insert(key, function_ast);
    }

    Ok(function_asts)
}

#[cfg(all(test, feature = "client"))]
mod into_convex_value_tests
{
    use serde_json::json;

    use super::IntoConvexValue;

    #[test]
    fn json_null_bool_string_array_object_roundtrip_shapes()
    {
        use convex::Value as Cv;
        assert!(matches!(json!(null).into_convex_value(), Cv::Null));
        assert!(matches!(json!(true).into_convex_value(), Cv::Boolean(true)));
        assert!(matches!(json!("hi").into_convex_value(), Cv::String(s) if s == "hi"));
        let arr = json!([1, 2]);
        let Cv::Array(v) = arr.into_convex_value() else {
            panic!("expected array");
        };
        assert_eq!(v.len(), 2);
        let obj = json!({ "a": 1 });
        let Cv::Object(m) = obj.into_convex_value() else {
            panic!("expected object");
        };
        assert!(m.contains_key("a"));
    }

    #[test]
    fn json_number_prefers_i64_then_f64()
    {
        use convex::Value as Cv;
        let Cv::Int64(i) = json!(42).into_convex_value() else {
            panic!("expected int64");
        };
        assert_eq!(i, 42);
        let Cv::Float64(f) = json!(1.5).into_convex_value() else {
            panic!("expected float");
        };
        assert!((f - 1.5).abs() < 1e-9);
    }
}

#[cfg(all(test, feature = "client"))]
mod convex_value_ext_tests
{
    use convex::Value as Cv;

    use super::ConvexValueExt;

    #[test]
    fn roundtrips_scalar_kinds()
    {
        assert!(Cv::Null.into_serde_value().is_null());
        assert_eq!(Cv::Boolean(false).into_serde_value(), serde_json::json!(false));
        assert_eq!(Cv::String("x".into()).into_serde_value(), serde_json::json!("x"));
    }

    #[test]
    fn float64_nan_maps_to_json_null()
    {
        let v = Cv::Float64(f64::NAN).into_serde_value();
        assert!(v.is_null());
    }

    #[test]
    fn bytes_map_to_json_number_array()
    {
        let v = Cv::Bytes(vec![1, 2]).into_serde_value();
        assert_eq!(v, serde_json::json!([1, 2]));
    }
}

#[cfg(all(test, feature = "client"))]
mod convex_client_ext_tests
{
    use std::collections::BTreeMap;

    use convex::ConvexClient;

    use super::ConvexClientExt;

    struct ArgsI32(i32);

    impl TryFrom<ArgsI32> for BTreeMap<String, serde_json::Value>
    {
        type Error = serde_json::Error;

        fn try_from(a: ArgsI32) -> Result<Self, Self::Error>
        {
            let mut m = BTreeMap::new();
            m.insert("a".to_string(), serde_json::to_value(a.0)?);
            Ok(m)
        }
    }

    #[test]
    fn prepare_args_converts_tryfrom_map_to_convex_values() -> Result<(), serde_json::Error>
    {
        let m = ConvexClient::prepare_args(ArgsI32(7))?;
        assert_eq!(m.len(), 1);
        Ok(())
    }

    struct ArgsTryFromErr;

    impl TryFrom<ArgsTryFromErr> for BTreeMap<String, serde_json::Value>
    {
        type Error = serde_json::Error;

        fn try_from(_: ArgsTryFromErr) -> Result<Self, Self::Error>
        {
            Err(serde_json::from_str::<serde_json::Value>("{").unwrap_err())
        }
    }

    #[test]
    fn prepare_args_propagates_tryfrom_serde_json_error()
    {
        let err = ConvexClient::prepare_args(ArgsTryFromErr).unwrap_err();
        assert!(!err.to_string().is_empty());
    }
}

#[cfg(test)]
mod create_schema_ast_tests
{
    use std::fs;
    use std::path::PathBuf;

    use tempfile::tempdir;

    use super::create_schema_ast;

    #[test]
    fn errors_when_path_does_not_exist()
    {
        let p = PathBuf::from("/nonexistent/convex-typegen/schema-xyz.ts");
        assert!(matches!(
            create_schema_ast(p).unwrap_err(),
            crate::error::ConvexTypeGeneratorError::MissingSchemaFile
        ));
    }

    #[test]
    fn parses_valid_typescript_file()
    {
        let tmp = tempdir().unwrap();
        let p = tmp.path().join("f.ts");
        fs::write(&p, "export const x: number = 1;\n").unwrap();
        let ast = create_schema_ast(p).unwrap();
        assert!(ast["body"].as_array().is_some());
    }

    #[test]
    fn empty_file_yields_empty_schema_file_error()
    {
        let tmp = tempdir().unwrap();
        let p = tmp.path().join("empty.ts");
        fs::write(&p, "   \n\t  ").unwrap();
        assert!(matches!(
            create_schema_ast(p).unwrap_err(),
            crate::error::ConvexTypeGeneratorError::EmptySchemaFile { .. }
        ));
    }
}

#[cfg(test)]
mod create_function_asts_tests
{
    use std::fs;
    use std::path::PathBuf;

    use tempfile::tempdir;

    use super::create_function_asts;

    #[test]
    fn empty_paths_yields_empty_map()
    {
        let m = create_function_asts(vec![]).unwrap();
        assert!(m.is_empty());
    }

    #[test]
    fn canonical_keys_and_parses_each_file()
    {
        let tmp = tempdir().unwrap();
        let a = tmp.path().join("a.ts");
        fs::write(&a, "export const n = 1;\n").unwrap();
        let m = create_function_asts(vec![a]).unwrap();
        assert_eq!(m.len(), 1);
        let (_k, v) = m.iter().next().unwrap();
        assert!(v["body"].as_array().is_some());
    }

    #[test]
    fn errors_on_nonexistent_path()
    {
        let p = PathBuf::from("/nonexistent/convex-typegen/missing.ts");
        let err = create_function_asts(vec![p]).unwrap_err();
        assert!(matches!(err, crate::error::ConvexTypeGeneratorError::IOError { .. }));
    }
}
