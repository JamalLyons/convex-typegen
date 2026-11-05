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
/// let config = convex_typegen::config::Configuration::default()
///     .with_convex_dir("backend")
///     .with_function_spec_path("backend/function-spec.json");
/// assert_eq!(
///     config.resolved_schema_path(),
///     std::path::Path::new("backend/schema.ts")
/// );
/// ```
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Configuration
{
    /// Explicit schema path. When unset, use `schema.ts` under [`Self::convex_dir`].
    pub schema_path: Option<PathBuf>,

    /// Output file path for generated Rust bindings (defaults to `$OUT_DIR/convex_api.rs`).
    pub out_file: PathBuf,

    /// Convex backend directory (default: `"convex"`, i.e. next to `Cargo.toml` when the build
    /// runs from the package root). When [`function_paths`](Self::function_paths) is empty, all
    /// `*.ts` files under this directory are used as function sources except the
    /// [resolved schema](Self::resolved_schema_path), `_generated/`, `node_modules/`, `http.ts`,
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
            schema_path: None,
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

impl Configuration
{
    /// Override the schema path, resolved relative to the consumer package.
    pub fn with_schema_path(mut self, path: impl Into<PathBuf>) -> Self
    {
        self.schema_path = Some(path.into());
        self
    }

    /// Set the generated Rust output path.
    pub fn with_out_file(mut self, path: impl Into<PathBuf>) -> Self
    {
        self.out_file = path.into();
        self
    }

    /// Set the Convex root for discovery, module names, and the default schema path.
    pub fn with_convex_dir(mut self, path: impl Into<PathBuf>) -> Self
    {
        self.convex_dir = path.into();
        self
    }

    /// Set the explicit function source list. An empty list enables discovery.
    pub fn with_function_paths(mut self, paths: Vec<PathBuf>) -> Self
    {
        self.function_paths = paths;
        self
    }

    /// Use a committed function spec as the authoritative function inventory.
    pub fn with_function_spec_path(mut self, path: impl Into<PathBuf>) -> Self
    {
        self.function_spec_path = Some(path.into());
        self
    }

    /// Return the configured schema override or `<convex_dir>/schema.ts`.
    pub fn resolved_schema_path(&self) -> PathBuf
    {
        self.schema_path.clone().unwrap_or_else(|| self.convex_dir.join("schema.ts"))
    }
}

#[cfg(test)]
mod tests
{
    use super::*;

    #[test]
    fn schema_defaults_follow_the_convex_root()
    {
        let config = Configuration::default();
        assert_eq!(config.resolved_schema_path(), PathBuf::from("convex/schema.ts"));
        assert!(config.out_file.ends_with("convex_api.rs"));
        let config = config.with_convex_dir("backend");
        assert_eq!(config.resolved_schema_path(), PathBuf::from("backend/schema.ts"));
        let config = config.with_schema_path("models/schema.ts").with_convex_dir("other");
        assert_eq!(config.resolved_schema_path(), PathBuf::from("models/schema.ts"));
    }
}
