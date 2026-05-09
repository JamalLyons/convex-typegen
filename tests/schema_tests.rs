use std::fs;
use std::path::PathBuf;

use convex_typegen::errors::ConvexTypeGeneratorError;
use convex_typegen::{generate, Configuration};
use tempdir::TempDir;

fn setup_test_schema(content: &str) -> (TempDir, PathBuf)
{
    let temp_dir = TempDir::new("convex_schema_test").expect("Failed to create temp directory");
    let schema_path = temp_dir.path().join("schema.ts");
    fs::write(&schema_path, content).expect("Failed to write test schema");
    (temp_dir, schema_path)
}

#[test]
fn test_valid_schema()
{
    let schema_content = r#"
        import { defineSchema, defineTable } from "convex/schema";

        export default defineSchema({
            messages: defineTable({
                author: v.string(),
                body: v.string(),
                timestamp: v.number(),
            }),
        });
    "#;

    let (_temp_dir, schema_path) = setup_test_schema(schema_content);

    let temp_file = TempDir::new("convex_codegen_test").expect("Failed to create temp directory");
    let output_path = temp_file.path().join("test_output.rs");

    let config = Configuration {
        schema_path,
        out_file: output_path,
        ..Default::default()
    };

    assert!(generate(config).is_ok());
}

#[test]
fn test_invalid_schema_syntax()
{
    let schema_content = r#"
        import { defineSchema, defineTable } from "convex/schema";

        export default defineSchema({
            messages: defineTable({
                author: v.string()  // Missing v import
                body: v.string()    // Missing comma
                timestamp: number   // Invalid type reference
            }),
        });
    "#;

    let (_temp_dir, schema_path) = setup_test_schema(schema_content);
    let config = Configuration {
        schema_path,
        ..Default::default()
    };

    match generate(config) {
        Err(ConvexTypeGeneratorError::ParsingFailed { .. }) => (),
        other => panic!("Expected ParsingFailed error, got {:?}", other),
    }
}

#[test]
fn test_missing_schema_file()
{
    let temp_dir = TempDir::new("convex_typegen_test").expect("Failed to create temp directory");
    let config = Configuration {
        schema_path: temp_dir.path().join("nonexistent.ts"),
        ..Default::default()
    };

    match generate(config) {
        Err(ConvexTypeGeneratorError::MissingSchemaFile) => (),
        other => panic!("Expected MissingSchemaFile error, got {:?}", other),
    }
}
