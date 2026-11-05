//! Smoke test mirroring a consumer `build.rs` without compiling a nested crate.

use std::fs;
use std::process::Command;

use convex_typegen::prelude::*;
use tempfile::tempdir;

#[test]
fn rcfp_and_generate_succeed_with_minimal_layout()
{
    let tmp = tempdir().unwrap();
    let convex = tmp.path().join("convex");
    fs::create_dir_all(&convex).unwrap();

    let schema = convex.join("schema.ts");
    fs::write(
        &schema,
        r#"
import { defineSchema, defineTable } from "convex/server";
import { v } from "convex/values";

export default defineSchema({
    t: defineTable({ x: v.number() }),
});
"#,
    )
    .unwrap();

    fs::write(
        convex.join("api.ts"),
        r#"
import { query } from "./_generated/server";
import { v } from "convex/values";

export const ping = query({
    args: {},
    handler: async (_ctx, _args) => null,
});
"#,
    )
    .unwrap();

    let out = tmp.path().join("src/convex_types.rs");
    fs::create_dir_all(out.parent().unwrap()).unwrap();

    let config = Configuration {
        schema_path: schema.clone(),
        out_file: out.clone(),
        convex_dir: convex.clone(),
        function_paths: Vec::new(),
        function_spec_path: None,
    };

    let paths = rcfp(&config).expect("resolve convex function sources");
    assert_eq!(paths.len(), 1);
    assert!(paths[0].ends_with("api.ts"));

    generate(config).expect("generate");
    let body = fs::read_to_string(&out).unwrap();
    assert!(body.contains("PingArgs"));
    assert!(body.contains("TDoc"));
}

#[test]
fn configured_spec_is_authoritative_and_tracked()
{
    let tmp = tempdir().unwrap();
    let convex = tmp.path().join("convex");
    fs::create_dir_all(&convex).unwrap();
    let schema = convex.join("schema.ts");
    fs::write(&schema, r#"import { defineSchema, defineTable } from "convex/server"; export default defineSchema({ tasks: defineTable({ text: v.string() }) });"#).unwrap();
    // The configured spec must make this irrelevant, even when TS function input is unusable.
    fs::write(convex.join("ignored.ts"), "not valid TypeScript !!!").unwrap();
    fs::write(
        convex.join("only_in_ts.ts"),
        "export const fromTs = query({ args: {}, handler: () => null });",
    )
    .unwrap();
    let spec = convex.join("function-spec.json");
    fs::write(&spec, r#"{"functions":[{"identifier":"tasks.js:ping","functionType":"Query","visibility":{"kind":"public"},"args":{"type":"object","value":{}},"returns":{"type":"string"}}]}"#).unwrap();
    let out = tmp.path().join("src/convex_types.rs");
    fs::create_dir_all(out.parent().unwrap()).unwrap();
    let config = Configuration {
        schema_path: schema,
        out_file: out.clone(),
        convex_dir: convex,
        function_paths: Vec::new(),
        function_spec_path: Some(spec.clone()),
    };
    assert_eq!(rcfp(&config).unwrap(), vec![spec.clone()]);
    generate(config.clone()).expect("generate from spec without parsing TypeScript functions");
    let body = fs::read_to_string(&out).unwrap();
    assert!(body.contains("PingArgs"), "expected spec function: {body}");
    assert!(body.contains("tasks:ping"), "expected normalized function path: {body}");
    assert!(!body.contains("Ignored"), "must not parse TypeScript function inputs: {body}");
    assert!(
        !body.contains("OnlyInTs"),
        "spec is authoritative over TS-only functions: {body}"
    );

    fs::write(&spec, r#"{"functions":[]}"#).unwrap();
    generate(config).expect("an empty spec is an authoritative empty function set");
    let body = fs::read_to_string(&out).unwrap();
    assert!(
        !body.contains("PingArgs") && !body.contains("tasks:ping"),
        "empty spec must produce no functions: {body}"
    );
}

#[test]
fn consumer_builds_offline_and_keeps_source_tree_clean()
{
    let temp = tempdir().unwrap();
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    fs::create_dir_all(temp.path().join("src")).unwrap();
    fs::create_dir_all(temp.path().join("convex")).unwrap();
    fs::write(
        temp.path().join("Cargo.toml"),
        format!(
            r#"[package]
name = "out-dir-smoke"
version = "0.1.0"
edition = "2024"
[dependencies]
convex-typegen-runtime = {{ path = {:?} }}
[build-dependencies]
convex-typegen = {{ path = {:?} }}
"#,
            root.join("runtime").display().to_string(),
            root.display().to_string()
        ),
    )
    .unwrap();
    fs::write(
        temp.path().join("build.rs"),
        "fn main() { convex_typegen::build::generate_out_dir().unwrap(); }",
    )
    .unwrap();
    fs::write(
        temp.path().join("src/lib.rs"),
        "pub mod convex_types { include!(concat!(env!(\"OUT_DIR\"), \"/convex_types.rs\")); }",
    )
    .unwrap();
    fs::write(temp.path().join("convex/schema.ts"), r#"import { defineSchema, defineTable } from "convex/server"; import { v } from "convex/values"; export default defineSchema({ tasks: defineTable({ text: v.string() }) });"#).unwrap();
    let status = Command::new(env!("CARGO"))
        .args(["check", "--offline", "--manifest-path"])
        .arg(temp.path().join("Cargo.toml"))
        .env("CARGO_NET_OFFLINE", "true")
        .env("CARGO_TARGET_DIR", root.join("target"))
        .status()
        .unwrap();
    assert!(status.success());
    assert!(!temp.path().join("src/convex_types.rs").exists());
    assert!(fs::read_dir(temp.path().join("target")).is_err());
}
