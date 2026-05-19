//! TypeScript / TSX → ESTree JSON ([`serde_json::Value`]) via [Oxc](https://oxc.rs).
//!
//! We run the parser **and** [`SemanticBuilder`] so obviously-invalid TS is rejected before we walk
//! the tree. The AST is not serde-serializable; Oxc exposes `to_estree_ts_json` which we deserialize
//! again so the rest of the crate can stay JSON-driven (same representation as [`parser`] consumes).
//!
//! Enable the **`verbose`** Cargo feature to print Oxc diagnostics to stderr in addition to the
//! string folded into [`crate::error::ConvexTypeGeneratorError::ParsingFailed`].

use std::path::PathBuf;

use oxc::allocator::Allocator;
use oxc::diagnostics::OxcDiagnostic;
use oxc::parser::Parser;
use oxc::semantic::SemanticBuilder;
use oxc::span::SourceType;
use serde_json::Value as JsonValue;

use crate::error::ConvexTypeGeneratorError;

/// Maximum source file size accepted by the lexer (DoS mitigation).
pub(crate) const MAX_SOURCE_BYTES: usize = 10 * 1024 * 1024;

/// Read `path`, parse with Oxc, and return the program as ESTree JSON.
pub(crate) fn generate_javascript_ast(path: &PathBuf) -> Result<JsonValue, ConvexTypeGeneratorError>
{
    let path_str = path.to_string_lossy().to_string();
    let allocator = Allocator::default();

    // Read file contents
    let source_text = std::fs::read_to_string(path).map_err(|error| ConvexTypeGeneratorError::IOError {
        file: path_str.clone(),
        error,
    })?;

    if source_text.trim().is_empty() {
        return Err(ConvexTypeGeneratorError::EmptySchemaFile { file: path_str });
    }

    if source_text.len() > MAX_SOURCE_BYTES {
        return Err(ConvexTypeGeneratorError::ParsingFailed {
            file: path_str.clone(),
            details: format!(
                "Source file exceeds maximum size ({} bytes, limit is {} bytes)",
                source_text.len(),
                MAX_SOURCE_BYTES
            ),
        });
    }

    let source_type = SourceType::from_path(path).map_err(|_| ConvexTypeGeneratorError::ParsingFailed {
        file: path_str.clone(),
        details: "Failed to determine source type".to_string(),
    })?;

    let mut errors: Vec<OxcDiagnostic> = Vec::new();

    let ret = Parser::new(&allocator, &source_text, source_type).parse();
    errors.extend(ret.errors);

    if ret.panicked {
        #[cfg(feature = "verbose")]
        for error in &errors {
            eprintln!("{error:?}");
        }
        return Err(ConvexTypeGeneratorError::ParsingFailed {
            file: path_str.clone(),
            details: parsing_failure_details("Parser panicked", &errors),
        });
    }

    if ret.program.is_empty() {
        return Err(ConvexTypeGeneratorError::EmptySchemaFile { file: path_str });
    }

    let semantics = SemanticBuilder::new().with_check_syntax_error(true).build(&ret.program);
    errors.extend(semantics.errors);

    if !errors.is_empty() {
        #[cfg(feature = "verbose")]
        for error in &errors {
            eprintln!("{error:?}");
        }
        return Err(ConvexTypeGeneratorError::ParsingFailed {
            file: path_str,
            details: parsing_failure_details("Semantic analysis failed", &errors),
        });
    }

    // Oxc's `Program` is not `serde::Serialize`; ESTree JSON is produced via `to_estree_*_json`.
    // todo: We might be able to use the `to_estree_js_json` method instead of `to_estree_ts_json` here. We don't really need the typescript fields. Look into this after the refactor to avoid breaking things.
    let estree_json = ret.program.to_estree_ts_json(false);
    serde_json::from_str(&estree_json).map_err(ConvexTypeGeneratorError::SerializationFailed)
}

fn parsing_failure_details(summary: &str, errors: &[OxcDiagnostic]) -> String
{
    let joined = join_oxc_diagnostic_messages(errors);
    if joined.is_empty() {
        summary.to_string()
    } else {
        format!("{summary}: {joined}")
    }
}

fn join_oxc_diagnostic_messages(errors: &[OxcDiagnostic]) -> String
{
    errors
        .iter()
        .map(ToString::to_string)
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("; ")
}

#[cfg(test)]
mod generate_javascript_ast_tests
{
    use std::fs;
    use std::path::PathBuf;

    use tempfile::tempdir;

    use super::{MAX_SOURCE_BYTES, generate_javascript_ast};

    #[test]
    fn parses_simple_export()
    {
        let tmp = tempdir().unwrap();
        let p = tmp.path().join("m.ts");
        fs::write(&p, "export const a = 1;\n").unwrap();
        let ast = generate_javascript_ast(&p).unwrap();
        assert!(ast.get("body").and_then(|b| b.as_array()).is_some());
    }

    #[test]
    fn missing_file_is_io_error()
    {
        let p = PathBuf::from("/nonexistent/convex-typegen/lexer-missing.ts");
        let e = generate_javascript_ast(&p).unwrap_err();
        assert!(matches!(e, crate::error::ConvexTypeGeneratorError::IOError { .. }));
    }

    #[test]
    fn whitespace_only_is_empty_schema_error()
    {
        let tmp = tempdir().unwrap();
        let p = tmp.path().join("w.ts");
        fs::write(&p, " \n  \t ").unwrap();
        assert!(matches!(
            generate_javascript_ast(&p).unwrap_err(),
            crate::error::ConvexTypeGeneratorError::EmptySchemaFile { .. }
        ));
    }

    #[test]
    fn invalid_syntax_returns_parsing_failed()
    {
        let tmp = tempdir().unwrap();
        let p = tmp.path().join("bad.ts");
        fs::write(&p, "export const x = )));\n").unwrap();
        let e = generate_javascript_ast(&p).unwrap_err();
        assert!(matches!(e, crate::error::ConvexTypeGeneratorError::ParsingFailed { .. }));
    }

    #[test]
    fn oversized_file_returns_parsing_failed()
    {
        let tmp = tempdir().unwrap();
        let p = tmp.path().join("big.ts");
        let oversized = "x".repeat(MAX_SOURCE_BYTES + 1);
        fs::write(&p, oversized).unwrap();
        let e = generate_javascript_ast(&p).unwrap_err();
        assert!(matches!(e, crate::error::ConvexTypeGeneratorError::ParsingFailed { .. }));
    }
}

#[cfg(test)]
mod parsing_failure_details_tests
{
    use oxc::diagnostics::OxcDiagnostic;

    use super::{join_oxc_diagnostic_messages, parsing_failure_details};

    #[test]
    fn summary_only_when_no_errors()
    {
        assert_eq!(parsing_failure_details("boom", &[]), "boom");
    }

    #[test]
    fn joins_nonempty_messages()
    {
        let a = OxcDiagnostic::error("first");
        let b = OxcDiagnostic::error("second");
        let s = parsing_failure_details("head", &[a, b]);
        assert!(s.starts_with("head:"));
        assert!(s.contains("first"));
        assert!(s.contains("second"));
    }

    #[test]
    fn join_skips_empty_strings()
    {
        assert_eq!(join_oxc_diagnostic_messages(&[]), "");
    }
}
