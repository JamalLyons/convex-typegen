mod codegen;
pub mod convex;
pub mod errors;

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

    /// Paths to Convex function files for generating function argument types
    pub function_paths: Vec<PathBuf>,
}

impl Default for Configuration {
    fn default() -> Self {
        Self {
            schema_path: PathBuf::from("convex/schema.ts"),
            out_file: PathBuf::from("src/convex_types.rs"),
            function_paths: Vec::new(),
        }
    }
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
    let functions_ast = create_functions_ast(config.function_paths)?;

    let parsed_schema = parse_schema_ast(schema_ast)?;
    let parsed_functions = parse_function_ast(functions_ast)?;

    generate_code(&config.out_file, (parsed_schema, parsed_functions))?;

    Ok(())
}
