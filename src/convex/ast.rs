//! Spike: direct Oxc [`Program`] inspection vs ESTree JSON round-trip.
//!
//! Not wired into production parsing yet; see [`docs/oxc-ast-evaluation.md`](../../docs/oxc-ast-evaluation.md).

use std::path::Path;

use oxc::allocator::Allocator;
use oxc::ast::ast::{Expression, Program, Statement};
use oxc::parser::Parser;
use oxc::semantic::SemanticBuilder;
use oxc::span::SourceType;

use crate::error::ConvexTypeGeneratorError;

/// Lightweight facts extracted by walking `Program` without JSON serialization.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct ProgramFacts
{
    pub export_default: bool,
    pub export_named_count: usize,
    pub call_expression_count: usize,
}

/// Parse `path` and return program facts from a direct AST walk (spike only).
pub(crate) fn extract_program_facts(path: &Path) -> Result<ProgramFacts, ConvexTypeGeneratorError>
{
    let path_str = path.to_string_lossy().to_string();
    let source_text = std::fs::read_to_string(path).map_err(|error| ConvexTypeGeneratorError::IOError {
        file: path_str.clone(),
        error,
    })?;

    let source_type = SourceType::from_path(path).map_err(|_| ConvexTypeGeneratorError::ParsingFailed {
        file: path_str.clone(),
        details: "Failed to determine source type".to_string(),
    })?;

    let allocator = Allocator::default();
    let ret = Parser::new(&allocator, &source_text, source_type).parse();
    if ret.panicked || ret.program.is_empty() {
        return Err(ConvexTypeGeneratorError::ParsingFailed {
            file: path_str,
            details: "Parser failed or empty program".to_string(),
        });
    }

    let semantics = SemanticBuilder::new().with_check_syntax_error(true).build(&ret.program);
    if !semantics.errors.is_empty() {
        return Err(ConvexTypeGeneratorError::ParsingFailed {
            file: path_str,
            details: "Semantic analysis failed".to_string(),
        });
    }

    Ok(walk_program_facts(&ret.program))
}

fn walk_program_facts(program: &Program<'_>) -> ProgramFacts
{
    let mut facts = ProgramFacts::default();
    for stmt in &program.body {
        match stmt {
            Statement::ExportDefaultDeclaration(_) => {
                facts.export_default = true;
            }
            Statement::ExportNamedDeclaration(_) => {
                facts.export_named_count += 1;
            }
            _ => {}
        }
        count_call_expressions_stmt(stmt, &mut facts);
    }
    facts
}

fn count_call_expressions_stmt(stmt: &Statement<'_>, facts: &mut ProgramFacts)
{
    match stmt {
        Statement::ExportNamedDeclaration(decl) => {
            if let Some(declaration) = &decl.declaration
                && let oxc::ast::ast::Declaration::VariableDeclaration(var_decl) = declaration
            {
                for declarator in &var_decl.declarations {
                    if let Some(init) = &declarator.init {
                        count_call_expressions_expr(init, facts);
                    }
                }
            }
        }
        Statement::ExportDefaultDeclaration(decl) => {
            if let Some(expr) = decl.declaration.as_expression() {
                count_call_expressions_expr(expr, facts);
            }
        }
        _ => {}
    }
}

fn count_call_expressions_expr(expr: &Expression<'_>, facts: &mut ProgramFacts)
{
    match expr {
        Expression::CallExpression(call) => {
            facts.call_expression_count += 1;
            if let Expression::Identifier(ident) = &call.callee {
                let _name = ident.name.as_str();
            }
            for arg in &call.arguments {
                if let Some(arg_expr) = arg.as_expression() {
                    count_call_expressions_expr(arg_expr, facts);
                }
            }
        }
        Expression::ObjectExpression(obj) => {
            for prop in &obj.properties {
                if let oxc::ast::ast::ObjectPropertyKind::ObjectProperty(p) = prop {
                    count_call_expressions_expr(&p.value, facts);
                }
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod ast_spike_tests
{
    use std::fs;
    use std::path::PathBuf;
    use std::time::Instant;

    use tempfile::tempdir;

    use super::extract_program_facts;
    use crate::convex::lexer::generate_javascript_ast;

    const SAMPLE: &str = r#"
import { defineSchema, defineTable } from "convex/server";
import { v } from "convex/values";
export default defineSchema({
  items: defineTable({ name: v.string() }),
});
"#;

    #[test]
    fn direct_walk_finds_export_default_and_calls()
    {
        let tmp = tempdir().unwrap();
        let p = tmp.path().join("schema.ts");
        fs::write(&p, SAMPLE).unwrap();
        let facts = extract_program_facts(&p).unwrap();
        assert!(facts.export_default);
        assert!(facts.call_expression_count >= 2);
    }

    #[test]
    fn json_path_and_direct_walk_on_same_file()
    {
        let tmp = tempdir().unwrap();
        let p = tmp.path().join("schema.ts");
        fs::write(&p, SAMPLE).unwrap();
        let ast = generate_javascript_ast(&p).unwrap();
        assert!(ast["body"].as_array().is_some());
        let facts = extract_program_facts(&p).unwrap();
        assert!(facts.export_default);
    }

    /// Wall-time comparison for maintainers (not a hard CI perf gate).
    #[test]
    fn benchmark_json_vs_direct_on_advanced_schema()
    {
        let schema = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/advanced/convex/schema.ts");
        if !schema.exists() {
            return;
        }
        let iterations = 50u32;
        let t0 = Instant::now();
        for _ in 0..iterations {
            let _ = generate_javascript_ast(&schema).unwrap();
        }
        let json_elapsed = t0.elapsed();
        let t1 = Instant::now();
        for _ in 0..iterations {
            let _ = extract_program_facts(&schema).unwrap();
        }
        let direct_elapsed = t1.elapsed();
        eprintln!("ast spike ({iterations}x advanced/schema.ts): json={json_elapsed:?} direct={direct_elapsed:?}");
        assert!(json_elapsed.as_secs() < 60);
        assert!(direct_elapsed.as_secs() < 60);
    }
}
