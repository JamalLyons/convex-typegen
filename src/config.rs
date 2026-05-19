//! Build-time inputs for the generator.
//!
//! Paths are resolved like Cargo does for `build.rs`: relative paths are relative to the **consumer**
//! package root (the crate that depends on `convex-typegen`), not this library’s source tree.

use std::path::PathBuf;

/// Paths and overrides for one codegen run (typically built in `build.rs`).
#[derive(Debug, Clone)]
pub struct Configuration
{
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

impl Default for Configuration
{
    fn default() -> Self
    {
        Self {
            schema_path: PathBuf::from("convex/schema.ts"),
            out_file: PathBuf::from("src/convex_types.rs"),
            convex_dir: PathBuf::from("convex"),
            function_paths: Vec::new(),
        }
    }
}

#[cfg(test)]
mod default_tests
{
    use super::*;

    #[test]
    fn default_paths_match_convex_layout_convention()
    {
        let c = Configuration::default();
        assert_eq!(c.schema_path, PathBuf::from("convex/schema.ts"));
        assert_eq!(c.out_file, PathBuf::from("src/convex_types.rs"));
        assert_eq!(c.convex_dir, PathBuf::from("convex"));
        assert!(c.function_paths.is_empty());
    }

    #[test]
    fn configuration_clone_roundtrip()
    {
        let c = Configuration {
            schema_path: PathBuf::from("a/schema.ts"),
            out_file: PathBuf::from("b/out.rs"),
            convex_dir: PathBuf::from("c"),
            function_paths: vec![PathBuf::from("d/e.ts")],
        };
        assert_eq!(c.clone().schema_path, c.schema_path);
        assert_eq!(c.clone().function_paths, c.function_paths);
    }
}
