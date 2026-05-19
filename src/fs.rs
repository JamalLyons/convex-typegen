//! Discover Convex function modules on disk for `build.rs`.
//!
//! With an empty [`crate::config::Configuration::function_paths`], walks `convex_dir` for `*.ts`,
//! skips `_generated/`, `node_modules/`, `*.d.ts`, and the canonical [`Configuration::schema_path`]
//! so the schema file is never treated as a function source. A missing `convex_dir` returns an
//! empty `Vec` (no error) so callers can still point [`Configuration::function_paths`] at explicit files.

use std::fs;
use std::path::{Path, PathBuf};

use crate::config::Configuration;
use crate::error::ConvexTypeGeneratorError;

const SKIP_DIR_NAMES: &[&str] = &["_generated", "node_modules"];

/// **R**esolved **C**onvex **f**unction **p**aths — identical set to what [`crate::generate`] parses.
///
/// Use from `build.rs` with `cargo:rerun-if-changed` on each path so TS edits invalidate the build.
pub fn rcfp(config: &Configuration) -> Result<Vec<PathBuf>, ConvexTypeGeneratorError>
{
    if !config.function_paths.is_empty() {
        return Ok(config.function_paths.clone());
    }
    find_convex_function_source_paths(&config.convex_dir, &config.schema_path)
}

/// Collect `*.ts` files under `convex_dir`, excluding declaration files and the schema file.
///
/// If `convex_dir` does not exist, returns an empty list (no error). If it exists but is not a
/// directory, returns [`ConvexTypeGeneratorError::InvalidPath`].
fn find_convex_function_source_paths(convex_dir: &Path, schema_path: &Path)
-> Result<Vec<PathBuf>, ConvexTypeGeneratorError>
{
    if !convex_dir.exists() {
        return Ok(Vec::new());
    }
    if !convex_dir.is_dir() {
        return Err(ConvexTypeGeneratorError::InvalidPath(format!(
            "convex_dir is not a directory: {}",
            convex_dir.display()
        )));
    }

    let schema_canonical = schema_path.canonicalize().ok();

    let mut paths = Vec::new();
    walk_ts_files(convex_dir, &mut paths)?;

    paths.retain(|p| {
        if p.to_string_lossy().ends_with(".d.ts") {
            return false;
        }
        if let Some(ref sch) = schema_canonical
            && let Ok(canon) = p.canonicalize()
            && &canon == sch
        {
            return false;
        }
        true
    });

    paths.sort();
    Ok(paths)
}

fn walk_ts_files(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), ConvexTypeGeneratorError>
{
    let entries = fs::read_dir(dir).map_err(|error| ConvexTypeGeneratorError::IOError {
        file: dir.to_string_lossy().to_string(),
        error,
    })?;

    for entry in entries {
        let entry = entry.map_err(|error| ConvexTypeGeneratorError::IOError {
            file: dir.to_string_lossy().to_string(),
            error,
        })?;
        let path = entry.path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        if path.is_dir() {
            if SKIP_DIR_NAMES.contains(&name) {
                continue;
            }
            walk_ts_files(&path, out)?;
        } else if path.extension().and_then(|e| e.to_str()) == Some("ts") {
            out.push(path);
        }
    }

    Ok(())
}

#[cfg(test)]
mod find_convex_function_source_paths_tests
{
    use std::fs;
    use std::io::Write;

    use tempfile::tempdir;

    use super::find_convex_function_source_paths;

    #[test]
    fn skips_generated_schema_and_d_ts_and_sorts()
    {
        let tmp = tempdir().unwrap();
        let convex = tmp.path().join("convex");
        fs::create_dir_all(convex.join("_generated")).unwrap();
        fs::create_dir_all(convex.join("sub")).unwrap();

        let schema = convex.join("schema.ts");
        let mut f = fs::File::create(&schema).unwrap();
        writeln!(f, "export default {{}}").unwrap();

        let mut f = fs::File::create(convex.join("api.ts")).unwrap();
        writeln!(f, "export const q = query({{}});").unwrap();

        let mut f = fs::File::create(convex.join("_generated/server.d.ts")).unwrap();
        writeln!(f, "export type X = 1;").unwrap();

        let mut f = fs::File::create(convex.join("sub/foo.ts")).unwrap();
        writeln!(f, "export const m = mutation({{}});").unwrap();

        let paths = find_convex_function_source_paths(&convex, &schema).unwrap();
        let names: Vec<_> = paths.iter().map(|p| p.file_name().unwrap().to_str().unwrap()).collect();
        assert_eq!(names, vec!["api.ts", "foo.ts"]);
    }

    #[test]
    fn missing_convex_dir_yields_empty_vec()
    {
        let tmp = tempdir().unwrap();
        let convex = tmp.path().join("nope");
        let schema = tmp.path().join("schema.ts");
        assert!(find_convex_function_source_paths(&convex, &schema).unwrap().is_empty());
    }

    #[test]
    fn convex_dir_not_directory_returns_invalid_path()
    {
        let tmp = tempdir().unwrap();
        let convex = tmp.path().join("convex");
        fs::write(&convex, b"not a dir").unwrap();
        let schema = tmp.path().join("schema.ts");
        let err = find_convex_function_source_paths(&convex, &schema).unwrap_err();
        assert!(matches!(err, crate::error::ConvexTypeGeneratorError::InvalidPath(_)));
    }
}

#[cfg(test)]
mod rcfp_tests
{
    use std::fs;

    use tempfile::tempdir;

    use super::rcfp;
    use crate::config::Configuration;

    #[test]
    fn returns_config_function_paths_when_non_empty_without_walking()
    {
        let tmp = tempdir().unwrap();
        let a = tmp.path().join("a.ts");
        let b = tmp.path().join("b.ts");
        fs::write(&a, "//").unwrap();
        fs::write(&b, "//").unwrap();
        let cfg = Configuration {
            function_paths: vec![a.clone(), b.clone()],
            ..Default::default()
        };
        let got = rcfp(&cfg).unwrap();
        assert_eq!(got, vec![a, b]);
    }

    #[test]
    fn delegates_to_discovery_when_function_paths_empty()
    {
        let tmp = tempdir().unwrap();
        let convex = tmp.path().join("convex");
        fs::create_dir_all(&convex).unwrap();
        let schema = convex.join("schema.ts");
        fs::write(&schema, "export default {}\n").unwrap();
        let api = convex.join("z.ts");
        fs::write(&api, "export const x = 1;\n").unwrap();

        let cfg = Configuration {
            convex_dir: convex.clone(),
            schema_path: schema,
            function_paths: Vec::new(),
            ..Default::default()
        };

        let names: Vec<_> = rcfp(&cfg)
            .unwrap()
            .into_iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, vec!["z.ts"]);
    }
}

#[cfg(test)]
mod walk_ts_files_tests
{
    use std::fs;

    use tempfile::tempdir;

    use super::walk_ts_files;

    #[test]
    fn collects_ts_recursively_skipping_named_dirs()
    {
        let tmp = tempdir().unwrap();
        let root = tmp.path();
        fs::create_dir_all(root.join("node_modules/pkg")).unwrap();
        fs::create_dir_all(root.join("_generated")).unwrap();
        fs::create_dir_all(root.join("nested")).unwrap();

        fs::write(root.join("root.ts"), "//\n").unwrap();
        fs::write(root.join("nested/inner.ts"), "//\n").unwrap();
        fs::write(root.join("node_modules/pkg/ignored.ts"), "//\n").unwrap();
        fs::write(root.join("_generated/x.ts"), "//\n").unwrap();

        let mut out = Vec::new();
        walk_ts_files(root, &mut out).unwrap();
        out.sort();
        let rel: Vec<_> = out
            .into_iter()
            .map(|p| p.strip_prefix(root).unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(rel, vec!["nested/inner.ts", "root.ts"]);
    }
}
