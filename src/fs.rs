//! Discover TypeScript endpoint sources in the configured Convex directory.

use std::fs;
use std::path::{Path, PathBuf};

use crate::config::Configuration;
use crate::error::ConvexTypeGeneratorError;

const SKIP_DIR_NAMES: &[&str] = &["_generated", "node_modules"];
const RESERVED_MODULES: &[&str] = &["http.ts", "crons.ts", "convex.config.ts"];

pub(crate) fn function_source_paths(config: &Configuration) -> Result<Vec<PathBuf>, ConvexTypeGeneratorError>
{
    if config.function_paths.is_empty() {
        find_convex_function_source_paths(&config.convex_dir, &config.resolved_schema_path())
    } else {
        Ok(config.function_paths.clone())
    }
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
        if p.file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| RESERVED_MODULES.contains(&name))
        {
            return false;
        }
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
    let entries = fs::read_dir(dir).map_err(|error| ConvexTypeGeneratorError::Io {
        file: dir.to_path_buf(),
        error,
    })?;

    for entry in entries {
        let entry = entry.map_err(|error| ConvexTypeGeneratorError::Io {
            file: dir.to_path_buf(),
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

    use super::{RESERVED_MODULES, find_convex_function_source_paths};

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

        for name in RESERVED_MODULES {
            fs::write(convex.join(name), "export default {};").unwrap();
        }

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
mod function_source_paths_tests
{
    use std::fs;

    use tempfile::tempdir;

    use super::function_source_paths;
    use crate::config::Configuration;

    #[test]
    fn returns_config_function_paths_when_non_empty_without_walking()
    {
        let tmp = tempdir().unwrap();
        let a = tmp.path().join("a.ts");
        let b = tmp.path().join("b.ts");
        fs::write(&a, "export const a = 1;").unwrap();
        fs::write(&b, "export const b = 1;").unwrap();
        let cfg = Configuration::default().with_function_paths(vec![a.clone(), b.clone()]);
        let got = function_source_paths(&cfg).unwrap();
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

        let cfg = Configuration::default().with_convex_dir(convex);

        let names: Vec<_> = function_source_paths(&cfg)
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
