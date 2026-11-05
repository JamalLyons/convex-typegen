//! Helpers for offline Cargo build scripts.

use crate::{Configuration, ConvexTypeGeneratorError, FunctionSources, generate_parsed, parse_project};

/// Generate `$OUT_DIR/convex_api.rs` with the standard Convex layout and track the build inputs.
///
/// In a consumer's `build.rs`:
///
/// ```no_run
/// convex_typegen::build::generate_out_dir().expect("generate Convex bindings");
/// ```
pub fn generate_out_dir() -> Result<(), ConvexTypeGeneratorError>
{
    if std::env::var_os("OUT_DIR").is_none() {
        return Err(ConvexTypeGeneratorError::InvalidPath(
            "OUT_DIR is not set; call this from build.rs".into(),
        ));
    }
    generate(Configuration::default())
}

/// Generate bindings with custom configuration and emit Cargo rebuild directives.
///
/// Tracks the schema, its relative imports, and the selected function inputs (including their
/// relative imports for TypeScript sources). Automatic discovery also tracks the Convex directory
/// so adding or removing a module triggers a rebuild. Only local files are read.
///
/// This example keeps the default Cargo output path and uses a committed function spec:
///
/// ```no_run
/// use convex_typegen::build;
/// use convex_typegen::config::Configuration;
///
/// build::generate(
///     Configuration::default()
///         .with_convex_dir("backend/convex")
///         .with_function_spec_path("backend/function-spec.json"),
/// )?;
/// # Ok::<(), convex_typegen::ConvexTypeGeneratorError>(())
/// ```
pub fn generate(config: Configuration) -> Result<(), ConvexTypeGeneratorError>
{
    let parsed = parse_project(&config)?;
    let mut inputs = std::collections::BTreeSet::from([config.resolved_schema_path()]);
    inputs.extend(parsed.schema_modules.keys().map(std::path::PathBuf::from));
    match &parsed.function_sources {
        FunctionSources::Spec(path) => {
            inputs.insert(path.clone());
        }
        FunctionSources::TypeScript { asts, .. } => {
            if config.function_paths.is_empty() {
                inputs.insert(config.convex_dir.clone());
            }
            inputs.extend(config.function_paths.iter().cloned());
            inputs.extend(asts.keys().map(std::path::PathBuf::from));
        }
    }
    for path in inputs {
        println!("cargo:rerun-if-changed={}", path.display());
    }
    generate_parsed(&config.out_file, parsed)
}
