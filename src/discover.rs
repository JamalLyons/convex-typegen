use std::fs;
use std::path::{Path, PathBuf};

use crate::errors::ConvexTypeGeneratorError;

/// Directory names to skip when scanning for Convex function sources.
const SKIP_DIR_NAMES: &[&str] = &["_generated", "node_modules"];

/// Collect `*.ts` files under `convex_dir`, excluding declaration files and the schema file.
///
/// If `convex_dir` does not exist, returns an empty list (no error). If it exists but is not a
/// directory, returns [`ConvexTypeGeneratorError::InvalidPath`].
pub(crate) fn discover_function_paths(convex_dir: &Path, schema_path: &Path) -> Result<Vec<PathBuf>, ConvexTypeGeneratorError> {
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
        if let Some(ref sch) = schema_canonical {
            if let Ok(canon) = p.canonicalize() {
                if &canon == sch {
                    return false;
                }
            }
        }
        true
    });

    paths.sort();
    Ok(paths)
}

fn walk_ts_files(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), ConvexTypeGeneratorError> {
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
mod tests {
    use super::*;
    use std::io::Write;
    use tempdir::TempDir;

    #[test]
    fn discover_skips_generated_schema_and_d_ts() {
        let tmp = TempDir::new("convex_discover").unwrap();
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

        let paths = discover_function_paths(&convex, &schema).unwrap();
        let names: Vec<_> = paths.iter().map(|p| p.file_name().unwrap().to_str().unwrap()).collect();
        assert_eq!(names, vec!["api.ts", "foo.ts"]);
    }
}
