#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc = include_str!("../readme.md")]
//!
//! ## Crate layout
//!
//! - [`config::Configuration`] — input paths, optional explicit function files, and an optional authoritative spec.
//! - [`generate`] — end-to-end pipeline used from `build.rs`.
//! - [`prelude`] — re-exports for `build.rs`.
//! - [`fs::rcfp`] — resolves the same function input paths as [`generate`] (for `cargo:rerun-if-changed`).
//!
//! Parsing and codegen live in the private `convex` module; the public API stays small on purpose.

use crate::config::Configuration;
use crate::convex::codegen::run_codegen;
use crate::convex::ir::GenerationIr;
use crate::convex::parser::{parse_function_ast, parse_schema_ast};
use crate::convex::spec::parse_function_spec;
use crate::convex::{create_function_asts, create_schema_ast};
use crate::error::ConvexTypeGeneratorError;
use crate::fs::rcfp;

pub mod config;
mod convex;
mod error;
mod fs;
pub mod prelude;

/// Runs the full generator: parse the TypeScript schema, resolve function inputs from either Oxc
/// TypeScript sources or the configured committed function spec, lower them to the shared typed
/// model, then emit Rust to [`Configuration::out_file`].
///
/// Pipeline: validate schema path → parse schema TS → either read the configured spec or parse
/// [`fs::rcfp`] TypeScript inputs → structured schema + function model → write generated Rust.
///
/// # Errors
///
/// Surfaced as [`error::ConvexTypeGeneratorError`]: missing paths, Oxc parse/semantic errors, schema
/// shapes or spec records we do not recognize, IO, or malformed JSON.
pub fn generate(config: Configuration) -> Result<(), ConvexTypeGeneratorError>
{
    if !config.schema_path.exists() {
        return Err(ConvexTypeGeneratorError::MissingSchemaFile);
    }

    let schema_path = config
        .schema_path
        .canonicalize()
        .map_err(|e| ConvexTypeGeneratorError::IOError {
            file: config.schema_path.to_string_lossy().to_string(),
            error: e,
        })?;

    let schema_ast = create_schema_ast(&schema_path)?;
    let convex_root = config.convex_dir.canonicalize().ok();
    let functions = if let Some(spec_path) = &config.function_spec_path {
        parse_function_spec(spec_path)?
    } else {
        let function_paths = rcfp(&config)?;
        let function_asts = create_function_asts(function_paths)?;
        parse_function_ast(function_asts, convex_root.as_deref())?
    };
    let ir = GenerationIr {
        schema: parse_schema_ast(schema_ast, &schema_path)?,
        functions,
    };
    run_codegen(&config.out_file, &ir)?;

    Ok(())
}

#[cfg(test)]
mod generate_tests
{
    use std::fs;

    use tempfile::tempdir;

    use super::generate;
    use crate::config::Configuration;

    #[test]
    fn generate_errors_when_schema_path_missing()
    {
        let tmp = tempdir().unwrap();
        let cfg = Configuration {
            schema_path: tmp.path().join("nope/schema.ts"),
            out_file: tmp.path().join("out.rs"),
            ..Default::default()
        };
        let err = generate(cfg).unwrap_err();
        assert!(matches!(err, crate::error::ConvexTypeGeneratorError::MissingSchemaFile));
    }

    /// Chained `.index()` / `.searchIndex()` / `.vectorIndex()` on `defineTable` is peeled before
    /// reading column validators; codegen should still see `users.email`.
    #[test]
    fn generate_accepts_define_table_with_chained_index()
    {
        const SCHEMA: &str = r#"
import { defineSchema, defineTable } from "convex/server";
import { v } from "convex/values";

export default defineSchema({
    users: defineTable({ email: v.string() }).index("by_email", ["email"]),
});
"#;

        let tmp = tempdir().unwrap();
        let convex = tmp.path().join("convex");
        fs::create_dir_all(&convex).unwrap();
        let schema = convex.join("schema.ts");
        fs::write(&schema, SCHEMA).unwrap();

        let out = tmp.path().join("out.rs");
        let cfg = Configuration {
            schema_path: schema.clone(),
            out_file: out.clone(),
            convex_dir: convex,
            function_paths: Vec::new(),
            function_spec_path: None,
        };

        generate(cfg).unwrap();
        let body = fs::read_to_string(&out).unwrap();
        assert!(body.contains("UsersTable"), "expected table struct: {body}");
        assert!(body.contains("pub email: String,"), "expected email column: {body}");
    }

    /// Args structs are `{Module}{Export}Args`, so the same export name in two modules is unique.
    #[test]
    fn generate_duplicate_export_names_produce_unique_args_structs()
    {
        const SCHEMA: &str = r#"
import { defineSchema, defineTable } from "convex/server";
import { v } from "convex/values";

export default defineSchema({
    games: defineTable({ n: v.number() }),
});
"#;

        const MOD_A: &str = r#"
import { query } from "./_generated/server";
import { v } from "convex/values";

export const list = query({
    args: { a: v.number() },
    handler: async (_ctx, _args) => null,
});
"#;

        const MOD_B: &str = r#"
import { query } from "./_generated/server";
import { v } from "convex/values";

export const list = query({
    args: { b: v.string() },
    handler: async (_ctx, _args) => null,
});
"#;

        let tmp = tempdir().unwrap();
        let convex = tmp.path().join("convex");
        fs::create_dir_all(&convex).unwrap();
        let schema = convex.join("schema.ts");
        fs::write(&schema, SCHEMA).unwrap();
        fs::write(convex.join("mod_a.ts"), MOD_A).unwrap();
        fs::write(convex.join("mod_b.ts"), MOD_B).unwrap();

        let out = tmp.path().join("out.rs");
        fs::create_dir_all(out.parent().unwrap()).unwrap();
        let cfg = Configuration {
            schema_path: schema,
            out_file: out.clone(),
            convex_dir: convex,
            function_paths: Vec::new(),
            function_spec_path: None,
        };

        generate(cfg).unwrap();
        let body = fs::read_to_string(&out).unwrap();
        assert!(body.contains("pub struct ModAListArgs"), "expected mod_a:list args struct");
        assert!(body.contains("pub struct ModBListArgs"), "expected mod_b:list args struct");
        assert!(body.contains("\"mod_a:list\""), "expected first module path in emitted Rust");
        assert!(body.contains("\"mod_b:list\""), "expected second module path in emitted Rust");
    }
}
