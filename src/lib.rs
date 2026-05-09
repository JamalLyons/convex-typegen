#![doc = include_str!("../readme.md")]
//!
//! ## Crate layout
//!
//! - [`config::Configuration`] — input paths and optional explicit function file list.
//! - [`generate`] — end-to-end pipeline used from `build.rs`.
//! - [`prelude`] — re-exports for `build.rs` and for crates that consume generated code.
//! - [`fs::rcfp`] — resolves the same function source paths as [`generate`] (for `cargo:rerun-if-changed`).
//!
//! Parsing and codegen live in the private `convex` module; the public API stays small on purpose.
//!
//! ## Serde
//!
//! Generated files use `#[serde(crate = "convex_typegen::serde")]` and
//! `convex_typegen::serde_json::…` so application crates do not need direct `serde` / `serde_json`
//! dependencies unless they use those crates themselves.

use crate::config::Configuration;
use crate::convex::codegen::run_codegen;
use crate::convex::parser::{parse_function_ast, parse_schema_ast};
use crate::convex::{create_function_asts, create_schema_ast};
use crate::error::ConvexTypeGeneratorError;
use crate::fs::rcfp;

pub mod config;
mod convex;
mod error;
mod fs;
pub mod prelude;

/// Crate-root re-export for generated `#[serde(crate = "convex_typegen::serde")]`.
pub use serde;
/// Crate-root re-export for generated `TryFrom` impls and `ConvexJsonValue` aliases.
pub use serde_json;

/// Ergonomic helpers for the official Convex Rust client (see [`prelude`]).
pub use crate::convex::ConvexClientExt;

/// Runs the full generator: resolve TypeScript inputs, parse with Oxc to ESTree JSON, walk that
/// JSON into an internal model, then emit Rust to [`Configuration::out_file`].
///
/// Pipeline: validate schema path → canonicalize schema → parse schema TS to ESTree JSON →
/// [`fs::rcfp`] → parse each function file → structured schema + function model → write generated Rust.
///
/// # Errors
///
/// Surfaced as [`error::ConvexTypeGeneratorError`]: missing paths, Oxc parse/semantic errors, schema
/// shapes we do not recognize, IO, or JSON deserialization of the ESTree payload.
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

    let schema_ast = create_schema_ast(schema_path)?;
    let function_paths = rcfp(&config)?;
    let function_asts = create_function_asts(function_paths)?;

    let parsed_schema = parse_schema_ast(schema_ast)?;
    let parsed_functions = parse_function_ast(function_asts)?;

    run_codegen(&config.out_file, (parsed_schema, parsed_functions))?;

    Ok(())
}

#[cfg(test)]
mod generate_tests
{
    use std::fs;

    use tempdir::TempDir;

    use super::generate;
    use crate::config::Configuration;

    #[test]
    fn generate_errors_when_schema_path_missing()
    {
        let tmp = TempDir::new("ctg_no_schema").unwrap();
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

        let tmp = TempDir::new("ctg_chained_index").unwrap();
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
        };

        generate(cfg).unwrap();
        let body = fs::read_to_string(&out).unwrap();
        assert!(body.contains("UsersTable"), "expected table struct: {body}");
        assert!(body.contains("pub email: String,"), "expected email column: {body}");
    }

    /// Codegen names args structs from the **export identifier** only (`ListArgs`), so the same
    /// name in two modules produces **duplicate** Rust items; `generate` still succeeds.
    #[test]
    fn generate_duplicate_export_names_produce_duplicate_args_structs()
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

        let tmp = TempDir::new("ctg_dup_exports").unwrap();
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
        };

        generate(cfg).unwrap();
        let body = fs::read_to_string(&out).unwrap();
        let struct_defs = body.matches("pub struct ListArgs").count();
        assert_eq!(
            struct_defs, 2,
            "expected two `pub struct ListArgs` from mod_a:list and mod_b:list; got {struct_defs}"
        );
        assert!(body.contains("\"mod_a:list\""), "expected first module path in emitted Rust");
        assert!(body.contains("\"mod_b:list\""), "expected second module path in emitted Rust");
    }
}
