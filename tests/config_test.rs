use std::fs;
use std::path::PathBuf;

use convex_typegen::errors::ConvexTypeGeneratorError;
use convex_typegen::{generate, Configuration};
use tempdir::TempDir;

fn setup_test_dir() -> TempDir {
    TempDir::new("convex_typegen_test").expect("Failed to create temp directory")
}

#[test]
fn test_configuration_default() {
    let config = Configuration::default();
    assert_eq!(config.schema_path, PathBuf::from("convex/schema.ts"));
    assert_eq!(config.out_file, PathBuf::from("src/convex_types.rs"));
    assert_eq!(config.convex_dir, PathBuf::from("convex"));
    assert!(config.function_paths.is_empty());
}

#[test]
fn test_missing_schema_file() {
    let temp_dir = setup_test_dir();
    let config = Configuration {
        schema_path: temp_dir.path().join("nonexistent.ts"),
        ..Default::default()
    };

    match generate(config) {
        Err(ConvexTypeGeneratorError::MissingSchemaFile) => (),
        other => panic!("Expected MissingSchemaFile error, got {:?}", other),
    }
}

#[test]
fn test_empty_schema_file() {
    let temp_dir = setup_test_dir();
    let schema_path = temp_dir.path().join("schema.ts");
    fs::write(&schema_path, "").unwrap();

    let config = Configuration {
        schema_path,
        ..Default::default()
    };

    match generate(config) {
        Err(ConvexTypeGeneratorError::EmptySchemaFile { .. }) => (),
        other => panic!("Expected EmptySchemaFile error, got {:?}", other),
    }
}
