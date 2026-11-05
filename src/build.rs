//! Helpers for offline Cargo build scripts.
use std::path::PathBuf;

use crate::{Configuration, ConvexTypeGeneratorError, generate, rcfp};

/// Generate `$OUT_DIR/convex_types.rs` and emit Cargo invalidation directives for each input.
///
/// This helper emits Cargo rebuild directives for the schema and discovered function sources.
/// It reads only local files; it does not run the Convex CLI or contact a backend.
///
/// # Example
///
/// Add this to the consumer crate's `build.rs`:
///
/// ```no_run
/// convex_typegen::build::generate_out_dir().expect("generate Convex bindings");
/// ```
pub fn generate_out_dir() -> Result<(), ConvexTypeGeneratorError>
{
    let out_dir = std::env::var_os("OUT_DIR")
        .map(PathBuf::from)
        .ok_or_else(|| ConvexTypeGeneratorError::InvalidPath("OUT_DIR is not set; call this from build.rs".into()))?;
    let config = Configuration {
        out_file: out_dir.join("convex_types.rs"),
        ..Configuration::default()
    };
    println!("cargo:rerun-if-changed={}", config.schema_path.display());
    println!("cargo:rerun-if-changed={}", config.convex_dir.display());
    for path in rcfp(&config)? {
        println!("cargo:rerun-if-changed={}", path.display());
    }
    generate(config)
}
