use std::fs;
use std::path::PathBuf;

use convex_typegen::{generate, Configuration};
use tempdir::TempDir;

fn setup_test_env(schema_content: &str) -> (TempDir, PathBuf, PathBuf)
{
    let temp_dir = TempDir::new("convex_codegen_test").expect("Failed to create temp directory");
    let schema_path = temp_dir.path().join("schema.ts");
    let output_path = temp_dir.path().join("types.rs");

    fs::write(&schema_path, schema_content).expect("Failed to write test schema");

    (temp_dir, schema_path, output_path)
}

#[test]
fn test_generated_types()
{
    let schema_content = r#"
        import { defineSchema, defineTable } from "convex/schema";

        export default defineSchema({
            users: defineTable({
                name: v.string(),
                age: v.number(),
                isActive: v.boolean(),
                tags: v.array(v.string()),
                metadata: v.object({
                    createdAt: v.number(),
                    updatedAt: v.number(),
                }),
            }),
        });
    "#;

    let (_temp_dir, schema_path, output_path) = setup_test_env(schema_content);
    let config = Configuration {
        schema_path,
        out_file: output_path.to_string_lossy().to_string(),
        ..Default::default()
    };

    assert!(generate(config).is_ok());

    // Read and verify generated code
    let generated_code = fs::read_to_string(output_path).expect("Failed to read generated code");
    assert!(generated_code.contains("pub struct UsersTable"));
    assert!(generated_code.contains("pub name: String"));
    assert!(generated_code.contains("pub age: f64"));
    assert!(generated_code.contains("pub isActive: bool"));
    assert!(generated_code.contains("pub tags: Vec<String>"));
    assert!(generated_code.contains("pub metadata: std::collections::BTreeMap<String, f64>"));
}

#[test]
fn test_heterogeneous_object_field_is_serde_json_value()
{
    let schema_content = r#"
        import { defineSchema, defineTable } from "convex/schema";

        export default defineSchema({
            docs: defineTable({
                payload: v.object({
                    title: v.string(),
                    score: v.number(),
                }),
            }),
        });
    "#;

    let (_temp_dir, schema_path, output_path) = setup_test_env(schema_content);
    let config = Configuration {
        schema_path,
        out_file: output_path.to_string_lossy().to_string(),
        ..Default::default()
    };

    assert!(generate(config).is_ok());
    let generated_code = fs::read_to_string(output_path).expect("Failed to read generated code");
    assert!(
        generated_code.contains("pub payload: serde_json::Value"),
        "mixed field types must not use a homogeneous BTreeMap; got:\n{}",
        &generated_code[..generated_code.len().min(2500)]
    );
}
