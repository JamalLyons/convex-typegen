//! Smoke tests for build scripts and an offline external consumer.

use std::fs;
use std::process::Command;

use convex_typegen::prelude::*;
use tempfile::tempdir;

#[test]
fn build_helper_succeeds_with_minimal_layout()
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

    let out = tmp.path().join("src/convex_api.rs");
    fs::create_dir_all(out.parent().unwrap()).unwrap();

    let config = Configuration::default().with_out_file(out.clone()).with_convex_dir(convex);

    convex_typegen::build::generate(config).expect("generate");
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
    let out = tmp.path().join("src/convex_api.rs");
    fs::create_dir_all(out.parent().unwrap()).unwrap();
    let config = Configuration::default()
        .with_schema_path(schema)
        .with_out_file(out.clone())
        .with_convex_dir(convex)
        .with_function_spec_path(spec.clone());
    convex_typegen::build::generate(config.clone()).expect("generate from spec without parsing TypeScript functions");
    let body = fs::read_to_string(&out).unwrap();
    assert!(body.contains("PingArgs"), "expected spec function: {body}");
    assert!(body.contains("tasks:ping"), "expected normalized function path: {body}");
    assert!(!body.contains("Ignored"), "must not parse TypeScript function inputs: {body}");
    assert!(
        !body.contains("OnlyInTs"),
        "spec is authoritative over TS-only functions: {body}"
    );

    fs::write(&spec, r#"{"functions":[]}"#).unwrap();
    convex_typegen::build::generate(config).expect("an empty spec is an authoritative empty function set");
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
    fs::create_dir_all(temp.path().join("backend")).unwrap();
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
        r#"fn main() {
    convex_typegen::build::generate_out_dir().unwrap();
    use convex_typegen::{ConvexTypeGeneratorError, build, config::Configuration};
    let out = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    let config = Configuration::default()
        .with_convex_dir("backend")
        .with_out_file(out.join("custom.rs"));
    let missing = config.clone().with_schema_path("missing.ts");
    match convex_typegen::generate(missing) {
        Err(ConvexTypeGeneratorError::MissingSchemaFile { path, .. }) => {
            assert_eq!(path, std::path::Path::new("missing.ts"));
        }
        other => panic!("expected missing schema, got {other:?}"),
    }
    build::generate(config).unwrap();
}"#,
    )
    .unwrap();
    fs::write(
        temp.path().join("src/lib.rs"),
        r#"
pub mod convex_api {
    convex_typegen_runtime::include!();
}
pub mod custom {
    convex_typegen_runtime::include!("custom.rs");
}
pub fn role() -> custom::TasksDocRole {
    custom::TasksDocRole::Admin
}
pub fn invalid(error: convex_typegen_runtime::CallError<()>) -> Option<String> {
    use convex_typegen_runtime::{CallError, InvalidApplicationData};
    match error {
        CallError::Application {
            message,
            data: Err(InvalidApplicationData { source, .. }),
            ..
        } => Some(format!("{message}: {source}")),
        _ => None,
    }
}
"#,
    )
    .unwrap();
    fs::write(temp.path().join("convex/schema.ts"), r#"import { defineSchema, defineTable } from "convex/server"; import { v } from "convex/values"; export default defineSchema({ tasks: defineTable({ text: v.string() }) });"#).unwrap();
    fs::write(
        temp.path().join("backend/schema.ts"),
        r#"
        import { defineSchema, defineTable } from "convex/server";
        import { role } from "../role";
        export default defineSchema({ tasks: defineTable({ role }) });
    "#,
    )
    .unwrap();
    let role_path = temp.path().join("role.ts");
    fs::write(
        &role_path,
        r#"import { v } from "convex/values"; export const role = v.union(v.literal("admin"), v.literal("member"));"#,
    )
    .unwrap();
    let check = || {
        Command::new(env!("CARGO"))
            .args(["check", "--offline", "--manifest-path"])
            .arg(temp.path().join("Cargo.toml"))
            .env("CARGO_NET_OFFLINE", "true")
            .env("CARGO_TARGET_DIR", root.join("target"))
            .output()
            .unwrap()
    };
    let initial = check();
    assert!(initial.status.success(), "{}", String::from_utf8_lossy(&initial.stderr));
    // The schema import is outside both Convex roots. Changing it must invalidate Cargo.
    fs::write(
        &role_path,
        r#"import { v } from "convex/values"; export const role = v.union(v.literal("viewer"), v.literal("member"));"#,
    )
    .unwrap();
    let changed = check();
    assert!(!changed.status.success());
    let stderr = String::from_utf8_lossy(&changed.stderr);
    assert!(stderr.contains("error[E0599]") && stderr.contains("`Admin`"), "{stderr}");
    assert!(!temp.path().join("src/convex_api.rs").exists());
    assert!(fs::read_dir(temp.path().join("target")).is_err());
}

#[test]
fn cli_resolves_schema_under_convex_dir_and_accepts_an_override()
{
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::create_dir(root.join("backend")).unwrap();
    fs::write(
        root.join("backend/schema.ts"),
        "export default defineSchema({ inferred: defineTable({}) });",
    )
    .unwrap();
    fs::write(
        root.join("alternate.ts"),
        "export default defineSchema({ explicit: defineTable({}) });",
    )
    .unwrap();
    let run = |flags: &[&str]| {
        let output = Command::new(env!("CARGO_BIN_EXE_convex-typegen"))
            .current_dir(root)
            .args(["--convex-dir", "backend", "--output", "bindings.rs"])
            .args(flags)
            .output()
            .unwrap();
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
        fs::read_to_string(root.join("bindings.rs")).unwrap()
    };

    assert!(run(&[]).contains("pub struct InferredDoc"));
    let overridden = run(&["--schema", "alternate.ts"]);
    assert!(overridden.contains("pub struct ExplicitDoc"));
    assert!(!overridden.contains("pub struct InferredDoc"));
}
