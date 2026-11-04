//! Oxc → ESTree JSON → typed IR → Rust text.
//!
//! This module is intentionally private: the stable surface is [`crate::generate`], [`crate::config`],
//! and [`crate::prelude`]. What lives here:
//!
//! - **Lexer** ([`lexer`]) — parse TS to a [`serde_json::Value`] ESTree tree.
//! - **Parser** ([`parser`]) — interpret Convex `defineSchema` / `defineTable` / `v.*` and exported
//!   `query`/`mutation`/`action` shapes into [`ir`].
//! - **Function spec** ([`spec`]) — validate committed `convex function-spec` JSON and lower it to
//!   the same function IR.
//! - **Source-neutral IR** ([`ir`]) — typed validators, canonical function identity, metadata,
//!   optional field presence, and source provenance shared by adapters and codegen.
//! - **Validator model** ([`validator`]) — typed `v.*` tree used by the source-neutral IR.
//! - **Codegen** ([`codegen`]) — stringify that model as `src/convex_types.rs`-style output.
//! - **AST spike** ([`ast`]) — direct Oxc `Program` walks; not used by [`crate::generate`] yet.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde_json::Value as JsonValue;

use crate::convex::lexer::generate_javascript_ast;
use crate::error::ConvexTypeGeneratorError;

#[allow(dead_code)]
pub(crate) mod ast;
pub(crate) mod codegen;
pub(crate) mod ir;
pub(crate) mod lexer;
pub(crate) mod parser;
pub(crate) mod spec;
pub(crate) mod utils;
pub(crate) mod validator;

/// Parse `schema.ts` through Oxc; returns the ESTree program as JSON ([`lexer::generate_javascript_ast`]).
pub(crate) fn create_schema_ast(path: &Path) -> Result<JsonValue, ConvexTypeGeneratorError>
{
    // Validate path exists before processing
    if !path.exists() {
        return Err(ConvexTypeGeneratorError::MissingSchemaFile);
    }

    generate_javascript_ast(path)
}

/// Parse every function source path; keys are **canonical** path strings (UTF-8 lossy).
///
/// [`BTreeMap`] keeps iteration order deterministic for stable codegen. Basenames alone would collide
/// for `convex/a/foo.ts` vs `convex/b/foo.ts`; full canonical keys keep ASTs separate while
/// [`parser::parse_function_ast`] resolves root-relative module identity.
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
            create_schema_ast(&p).unwrap_err(),
            crate::error::ConvexTypeGeneratorError::MissingSchemaFile
        ));
    }

    #[test]
    fn parses_valid_typescript_file()
    {
        let tmp = tempdir().unwrap();
        let p = tmp.path().join("f.ts");
        fs::write(&p, "export const x: number = 1;\n").unwrap();
        let ast = create_schema_ast(&p).unwrap();
        assert!(ast["body"].as_array().is_some());
    }

    #[test]
    fn empty_file_yields_empty_schema_file_error()
    {
        let tmp = tempdir().unwrap();
        let p = tmp.path().join("empty.ts");
        fs::write(&p, "   \n\t  ").unwrap();
        assert!(matches!(
            create_schema_ast(&p).unwrap_err(),
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
