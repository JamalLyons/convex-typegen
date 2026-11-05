//! Build-time inputs for the generator.
//!
//! Paths are resolved like Cargo does for `build.rs`: relative paths are relative to the **consumer**
//! package root (the crate that depends on `convex-typegen`), not this library’s source tree.

use std::path::PathBuf;

/// Paths and overrides for one codegen run (typically built in `build.rs`).
///
/// Defaults match the standard Convex project layout and write output to Cargo's `OUT_DIR` when
/// available.
///
/// # Example
///
/// ```
/// let config = convex_typegen::config::Configuration {
///     function_spec_path: Some("convex/function-spec.json".into()),
///     ..Default::default()
/// };
/// assert_eq!(config.schema_path, std::path::Path::new("convex/schema.ts"));
/// ```
#[derive(Debug, Clone)]
pub struct Configuration
{
    /// Path to the Convex schema file (default: "convex/schema.ts")
    pub schema_path: PathBuf,

    /// Output file path for generated Rust bindings (defaults to `$OUT_DIR/convex_api.rs`).
    pub out_file: PathBuf,

    /// Convex backend directory (default: `"convex"`, i.e. next to `Cargo.toml` when the build
    /// runs from the package root). When [`function_paths`](Self::function_paths) is empty, all
    /// `*.ts` files under this directory are used as function sources except `schema.ts` (same
    /// file as [`schema_path`](Self::schema_path)), `_generated/`, `node_modules/`, `http.ts`,
    /// `crons.ts`, `convex.config.ts`, and `*.d.ts`.
    pub convex_dir: PathBuf,

    /// When non-empty and [`function_spec_path`](Self::function_spec_path) is `None`, only these
    /// files are parsed as Convex functions (directory discovery is skipped). Use this in tests or
    /// for unusual layouts. Files outside [`convex_dir`](Self::convex_dir) have no root-relative
    /// module identity.
    pub function_paths: Vec<PathBuf>,

    /// Optional committed output from `npx convex function-spec`. When set, it takes precedence
    /// over [`function_paths`](Self::function_paths) and is the complete authoritative function
    /// inventory; TypeScript is still parsed for the schema.
    pub function_spec_path: Option<PathBuf>,
}

impl Default for Configuration
{
    fn default() -> Self
    {
        Self {
            schema_path: PathBuf::from("convex/schema.ts"),
            out_file: std::env::var_os("OUT_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("OUT_DIR"))
                .join("convex_api.rs"),
            convex_dir: PathBuf::from("convex"),
            function_paths: Vec::new(),
            function_spec_path: None,
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
        assert!(c.out_file.ends_with("convex_api.rs"));
        assert!(!c.out_file.starts_with("src"));
        assert_eq!(c.convex_dir, PathBuf::from("convex"));
        assert!(c.function_paths.is_empty());
        assert!(c.function_spec_path.is_none());
    }

    #[test]
    fn configuration_clone_roundtrip()
    {
        let c = Configuration {
            schema_path: PathBuf::from("a/schema.ts"),
            out_file: PathBuf::from("b/out.rs"),
            convex_dir: PathBuf::from("c"),
            function_paths: vec![PathBuf::from("d/e.ts")],
            function_spec_path: None,
        };
        assert_eq!(c.clone().schema_path, c.schema_path);
        assert_eq!(c.clone().function_paths, c.function_paths);
    }
}
