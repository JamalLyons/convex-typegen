mod codegen;
pub mod convex;
mod discover;
pub mod errors;
pub mod prelude;

/// Re-export of **serde** so generated code can use `#[serde(crate = "convex_typegen::serde")]` and
/// your application crate does not need its own `serde` dependency for those derives.
pub use serde;
/// Re-export of **serde_json** so generated `TryFrom` bodies can call `convex_typegen::serde_json::…`
/// without a direct `serde_json` dependency in your application crate.
pub use serde_json;

use std::path::PathBuf;

use codegen::generate_code;
use convex::{create_functions_ast, create_schema_ast, parse_function_ast, parse_schema_ast};
use errors::ConvexTypeGeneratorError;

/// Configuration options for the type generator.
#[derive(Debug, Clone)]
pub struct Configuration {
    /// Path to the Convex schema file (default: "convex/schema.ts")
    pub schema_path: PathBuf,

    /// Output file path for generated Rust types (default: "src/convex_types.rs")
    pub out_file: PathBuf,

    /// Convex backend directory (default: `"convex"`, i.e. next to `Cargo.toml` when the build
    /// runs from the package root). When [`function_paths`](Self::function_paths) is empty, all
    /// `*.ts` files under this directory are used as function sources except `schema.ts` (same
    /// file as [`schema_path`](Self::schema_path)), `_generated/`, `node_modules/`, and `*.d.ts`.
    pub convex_dir: PathBuf,

    /// When non-empty, only these files are parsed as Convex functions (directory discovery is
    /// skipped). Use this in tests or for unusual layouts.
    pub function_paths: Vec<PathBuf>,
}

impl Default for Configuration {
    fn default() -> Self {
        Self {
            schema_path: PathBuf::from("convex/schema.ts"),
            out_file: PathBuf::from("src/convex_types.rs"),
            convex_dir: PathBuf::from("convex"),
            function_paths: Vec::new(),
        }
    }
}

/// Resolves which TypeScript files will be parsed for Convex queries, mutations, and actions.
///
/// This is the same list [`generate`](crate::generate) uses. Intended for `build.rs`
/// `cargo:rerun-if-changed` lines.
pub fn resolved_function_paths(config: &Configuration) -> Result<Vec<PathBuf>, ConvexTypeGeneratorError> {
    if !config.function_paths.is_empty() {
        return Ok(config.function_paths.clone());
    }
    discover::discover_function_paths(&config.convex_dir, &config.schema_path)
}

/// Generates Rust types from Convex schema and function definitions.
///
/// # Arguments
/// * `config` - Configuration options for the type generation process
///
/// # Returns
/// * `Ok(())` if type generation succeeds
/// * `Err(ConvexTypeGeneratorError)` if an error occurs during generation
///
/// # Errors
/// This function can fail for several reasons:
/// * Schema file not found
/// * Invalid schema structure
/// * IO errors when reading/writing files
/// * Parse errors in schema or function files
pub fn generate(config: Configuration) -> Result<(), ConvexTypeGeneratorError> {
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
    let function_paths = resolved_function_paths(&config)?;
    let functions_ast = create_functions_ast(function_paths)?;

    let parsed_schema = parse_schema_ast(schema_ast)?;
    let parsed_functions = parse_function_ast(functions_ast)?;

    generate_code(&config.out_file, (parsed_schema, parsed_functions))?;

    Ok(())
}
