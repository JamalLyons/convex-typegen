#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc = include_str!("../readme.md")]
//!
//! ## Crate layout
//!
//! - [`config::Configuration`] — input paths, optional explicit function files, and an optional authoritative spec.
//! - [`generate`] — end-to-end pipeline used from `build.rs`.
//! - [`prelude`] — re-exports for `build.rs`.
//! - [`build`] — offline Cargo helpers that also track generation inputs.
//!
//! Parsing and codegen live in the private `convex` module; the public API stays small on purpose.
//!
//! # Example
//!
//! In a consumer's `build.rs`, generate the default `OUT_DIR/convex_api.rs` file:
//!
//! ```no_run
//! fn main()
//! {
//!     if let Err(error) = convex_typegen::build::generate_out_dir() {
//!         panic!("convex-typegen failed: {error}");
//!     }
//! }
//! ```
//!
//! For custom paths or a committed function spec, configure the generator directly:
//!
//! ```no_run
//! let config = convex_typegen::config::Configuration::default().with_convex_dir("backend");
//! convex_typegen::build::generate(config)?;
//! # Ok::<(), convex_typegen::prelude::ConvexTypeGeneratorError>(())
//! ```

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::config::Configuration;
use crate::convex::codegen::generate_code;
use crate::convex::create_function_asts;
use crate::convex::ir::GenerationIr;
use crate::convex::parser::{parse_function_ast, parse_schema_ast_with_modules};
use crate::convex::spec::parse_function_spec;
pub use crate::error::ConvexTypeGeneratorError;
use crate::fs::function_source_paths;

pub mod build;
pub mod config;
mod convex;
mod error;
mod fs;
pub mod prelude;

#[derive(Clone)]
struct ParsedProject
{
    schema_path: PathBuf,
    schema_ast: serde_json::Value,
    schema_modules: BTreeMap<String, serde_json::Value>,
    function_sources: FunctionSources,
}

#[derive(Clone)]
enum FunctionSources
{
    TypeScript
    {
        asts: BTreeMap<String, serde_json::Value>,
        convex_root: Option<PathBuf>,
    },
    Spec(PathBuf),
}

struct LoweredProject(GenerationIr);

fn parse_project(config: &Configuration) -> Result<ParsedProject, ConvexTypeGeneratorError>
{
    let schema_input = config.resolved_schema_path();
    let schema_path = schema_input.canonicalize().map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            ConvexTypeGeneratorError::MissingSchemaFile {
                path: schema_input.clone(),
            }
        } else {
            ConvexTypeGeneratorError::Io {
                file: schema_input.clone(),
                error,
            }
        }
    })?;
    let schema_modules = create_function_asts(vec![schema_path.clone()])?;
    let schema_ast = schema_modules
        .get(schema_path.to_string_lossy().as_ref())
        .cloned()
        .ok_or_else(|| ConvexTypeGeneratorError::InvalidPath(schema_path.to_string_lossy().into_owned()))?;
    let function_sources = if let Some(spec_path) = &config.function_spec_path {
        FunctionSources::Spec(spec_path.clone())
    } else {
        let convex_root = config.convex_dir.canonicalize().ok();
        let function_paths = function_source_paths(config)?;
        let asts = create_function_asts(function_paths)?;
        FunctionSources::TypeScript { asts, convex_root }
    };

    Ok(ParsedProject {
        schema_path,
        schema_ast,
        schema_modules,
        function_sources,
    })
}

fn lower_project(parsed: ParsedProject) -> Result<LoweredProject, ConvexTypeGeneratorError>
{
    let functions = match parsed.function_sources {
        FunctionSources::TypeScript { asts, convex_root } => parse_function_ast(asts, convex_root.as_deref())?,
        FunctionSources::Spec(spec_path) => parse_function_spec(&spec_path)?,
    };
    let schema = parse_schema_ast_with_modules(parsed.schema_ast, &parsed.schema_path, Some(&parsed.schema_modules))?;
    Ok(LoweredProject(GenerationIr { schema, functions }))
}

/// Runs the full generator: parse the TypeScript schema, resolve function inputs from either Oxc
/// TypeScript sources or the configured committed function spec, lower them to the shared typed
/// model, then emit Rust to [`Configuration::out_file`].
///
/// Pipeline: validate schema path → parse schema TS → either read the configured spec or parse
/// discovered TypeScript inputs → structured schema + function model → write generated Rust.
///
/// # Errors
///
/// Surfaced as [`ConvexTypeGeneratorError`]: missing paths, Oxc parse/semantic errors, schema
/// shapes or spec records we do not recognize, IO, or malformed JSON.
///
/// # Example
///
/// This is typically called from a Cargo build script. The example compiles without requiring a
/// Convex project to be present while rustdoc runs.
///
/// ```no_run
/// let config = convex_typegen::config::Configuration::default();
/// convex_typegen::generate(config)?;
/// # Ok::<(), convex_typegen::prelude::ConvexTypeGeneratorError>(())
/// ```
pub fn generate(config: Configuration) -> Result<(), ConvexTypeGeneratorError>
{
    let parsed = parse_project(&config)?;
    generate_parsed(&config.out_file, parsed)
}

fn generate_parsed(path: &Path, parsed: ParsedProject) -> Result<(), ConvexTypeGeneratorError>
{
    let lowered = lower_project(parsed)?;
    let output = generate_code(&lowered.0)?;
    write_atomic(path, output.as_bytes())
}

/// Check whether generated output matches the configured artifact without writing files.
///
/// ```no_run
/// let config = convex_typegen::config::Configuration::default();
/// let is_current = convex_typegen::check(config)?;
/// # Ok::<(), convex_typegen::prelude::ConvexTypeGeneratorError>(())
/// ```
pub fn check(config: Configuration) -> Result<bool, ConvexTypeGeneratorError>
{
    let parsed = parse_project(&config)?;
    let lowered = lower_project(parsed)?;
    let generated = generate_code(&lowered.0)?;
    let existing = match std::fs::read_to_string(&config.out_file) {
        Ok(existing) => existing,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            eprintln!("{} is missing; run convex-typegen to generate it", config.out_file.display());
            return Ok(false);
        }
        Err(error) => {
            return Err(ConvexTypeGeneratorError::Io {
                file: config.out_file,
                error,
            });
        }
    };
    if existing == generated {
        return Ok(true);
    }
    let old = existing.lines().collect::<Vec<_>>();
    let new = generated.lines().collect::<Vec<_>>();
    let first = old
        .iter()
        .zip(&new)
        .position(|(a, b)| a != b)
        .unwrap_or(old.len().min(new.len()));
    eprintln!(
        "{} has drifted: output differs at line {} ({} lines currently, {} generated); run convex-typegen to update it",
        config.out_file.display(),
        first + 1,
        old.len(),
        new.len()
    );
    Ok(false)
}

fn write_atomic(path: &Path, contents: &[u8]) -> Result<(), ConvexTypeGeneratorError>
{
    use std::io::Write;
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);
    let parent = path.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new("."));
    std::fs::create_dir_all(parent).map_err(|error| ConvexTypeGeneratorError::Io {
        file: parent.to_path_buf(),
        error,
    })?;
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    let (temp_path, mut file) = loop {
        let suffix = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
        let candidate = parent.join(format!(".{name}.{}.{}.tmp", std::process::id(), suffix));
        match std::fs::OpenOptions::new().write(true).create_new(true).open(&candidate) {
            Ok(file) => break (candidate, file),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(ConvexTypeGeneratorError::Io { file: candidate, error });
            }
        }
    };
    let result = file.write_all(contents).and_then(|()| file.sync_all()).and_then(|()| {
        drop(file);
        std::fs::rename(&temp_path, path)
    });
    if let Err(error) = result {
        let _ = std::fs::remove_file(&temp_path);
        return Err(ConvexTypeGeneratorError::Io {
            file: path.to_path_buf(),
            error,
        });
    }
    Ok(())
}

/// Hidden access to production pipeline stages for the unpublished benchmark harness.
#[cfg(feature = "bench-internals")]
#[doc(hidden)]
pub mod bench_support
{
    use std::path::Path;

    use crate::config::Configuration;
    use crate::error::ConvexTypeGeneratorError;

    #[derive(Clone)]
    pub struct ParsedProject(super::ParsedProject);

    pub struct LoweredProject(super::LoweredProject);

    pub fn parse(config: &Configuration) -> Result<ParsedProject, ConvexTypeGeneratorError>
    {
        super::parse_project(config).map(ParsedProject)
    }

    pub fn lower(project: ParsedProject) -> Result<LoweredProject, ConvexTypeGeneratorError>
    {
        super::lower_project(project.0).map(LoweredProject)
    }

    pub fn codegen(path: &Path, project: &LoweredProject) -> Result<(), ConvexTypeGeneratorError>
    {
        super::write_atomic(path, super::generate_code(&project.0.0)?.as_bytes())
    }
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
        let cfg = Configuration::default()
            .with_schema_path(tmp.path().join("nope/schema.ts"))
            .with_out_file(tmp.path().join("out.rs"));
        let err = generate(cfg).unwrap_err();
        assert!(
            matches!(err, crate::ConvexTypeGeneratorError::MissingSchemaFile { path } if path == tmp.path().join("nope/schema.ts"))
        );
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
        let cfg = Configuration::default()
            .with_schema_path(schema)
            .with_out_file(out.clone())
            .with_convex_dir(convex);

        generate(cfg).unwrap();
        let body = fs::read_to_string(&out).unwrap();
        assert!(body.contains("UsersDoc"), "expected table struct: {body}");
        assert!(body.contains("pub email: String,"), "expected email column: {body}");
    }

    /// Each module scopes its own `{Export}Args` structs.
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
        let cfg = Configuration::default()
            .with_schema_path(schema)
            .with_out_file(out.clone())
            .with_convex_dir(convex);

        generate(cfg).unwrap();
        let body = fs::read_to_string(&out).unwrap();
        assert_eq!(body.matches("pub struct ListArgs").count(), 2);
        assert!(body.contains("\"mod_a:list\""), "expected first module path in emitted Rust");
        assert!(body.contains("\"mod_b:list\""), "expected second module path in emitted Rust");
    }
}
