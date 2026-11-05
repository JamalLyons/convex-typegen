use std::fs;
use std::process::Command;

use tempfile::tempdir;

fn fixture() -> (tempfile::TempDir, std::path::PathBuf, std::path::PathBuf)
{
    let dir = tempdir().unwrap();
    let schema = dir.path().join("schema.ts");
    fs::write(&schema, r#"import { defineSchema, defineTable } from "convex/server"; import { v } from "convex/values"; export default defineSchema({ tasks: defineTable({ text: v.string() }) });"#).unwrap();
    let output = dir.path().join("committed.rs");
    (dir, schema, output)
}

fn cli() -> Command
{
    Command::new(env!("CARGO_BIN_EXE_convex-typegen"))
}

#[test]
fn check_reports_drift_without_writing()
{
    let (dir, schema, output) = fixture();
    fs::write(&output, "old output\n").unwrap();
    let before = fs::read(&output).unwrap();
    let result = cli()
        .args([
            "--check",
            "--schema",
            schema.to_str().unwrap(),
            "--convex-dir",
            dir.path().to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&result.stderr).contains("has drifted"));
    assert_eq!(fs::read(output).unwrap(), before);
}

#[test]
fn diagnostics_include_source_context()
{
    for (label, schema_text, expected) in [
        ("parser", "export default {", "Parser panicked"),
        (
            "validator",
            r#"import { defineSchema, defineTable } from "convex/server"; import { v } from "convex/values"; export default defineSchema({ tasks: defineTable({ value: v.symbol() }) });"#,
            "unsupported validator",
        ),
        (
            "collision",
            r#"import { defineSchema, defineTable } from "convex/server"; import { v } from "convex/values"; export default defineSchema({ foo_bar: defineTable({}), "foo-bar": defineTable({}) });"#,
            "collision",
        ),
    ] {
        let (dir, schema, output) = fixture();
        fs::write(&schema, schema_text).unwrap();
        let result = cli()
            .args([
                "--schema",
                schema.to_str().unwrap(),
                "--convex-dir",
                dir.path().to_str().unwrap(),
                "--output",
                output.to_str().unwrap(),
            ])
            .output()
            .unwrap();
        assert!(!result.status.success(), "{label} unexpectedly succeeded");
        let canonical_dir = dir.path().canonicalize().unwrap();
        let stderr = String::from_utf8_lossy(&result.stderr)
            .replace(canonical_dir.to_str().unwrap(), "<PROJECT>")
            .lines()
            .filter(|line| !line.starts_with("OxcDiagnostic {"))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(stderr.contains(expected), "{label}: {stderr}");
        insta::assert_snapshot!(label, stderr);
        assert!(!output.exists(), "failed generation must not create output");
    }
}

#[test]
fn committed_example_bindings_match_the_generator()
{
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    for example in ["basic", "advanced"] {
        let result = cli()
            .current_dir(root.join("examples").join(example))
            .args(["--check", "--output", "src/convex_types.rs"])
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{example}: {}",
            String::from_utf8_lossy(&result.stderr)
        );
    }
}
