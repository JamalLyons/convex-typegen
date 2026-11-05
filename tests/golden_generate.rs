//! Golden tests for stable codegen output.

use std::fs;

use convex_typegen::config::Configuration;
use convex_typegen::generate;
use tempfile::tempdir;

const MINIMAL_SCHEMA: &str = r#"
import { defineSchema, defineTable } from "convex/server";
import { v } from "convex/values";

export default defineSchema({
    items: defineTable({ name: v.string() }),
});
"#;

const MINIMAL_FN: &str = r#"
import { query } from "./_generated/server";
import { v } from "convex/values";

export const getItem = query({
    args: { id: v.string() },
    handler: async (_ctx, _args) => null,
});
"#;

const DUP_MOD_A: &str = r#"
import { query } from "./_generated/server";
import { v } from "convex/values";

export const list = query({
    args: { a: v.number() },
    handler: async (_ctx, _args) => null,
});
"#;

const DUP_MOD_B: &str = r#"
import { query } from "./_generated/server";
import { v } from "convex/values";

export const list = query({
    args: { b: v.string() },
    handler: async (_ctx, _args) => null,
});
"#;

const NESTED_OBJECT_SCHEMA: &str = r#"
import { defineSchema, defineTable } from "convex/server";
import { v } from "convex/values";

export default defineSchema({
    projects: defineTable({
        settings: v.object({
            theme: v.string(),
            notifyEmail: v.boolean(),
        }),
    }),
});
"#;

const NESTED_OBJECT_FN: &str = r#"
import { action } from "./_generated/server";
import { v } from "convex/values";

export const integrationsMirror = action({
    args: {
        flags: v.object({
            verbose: v.boolean(),
            trace: v.boolean(),
        }),
    },
    handler: async (_ctx, _args) => null,
});
"#;

const OPTIONAL_FN: &str = r#"
import { mutation } from "./_generated/server";
import { v } from "convex/values";

export const update = mutation({
    args: { flag: v.optional(v.boolean()) },
    handler: async (_ctx, _args) => null,
});
"#;

fn run_generate(schema: &str, extra_files: &[(&str, &str)]) -> String
{
    let tmp = tempdir().unwrap();
    let convex = tmp.path().join("convex");
    fs::create_dir_all(&convex).unwrap();
    let schema_path = convex.join("schema.ts");
    fs::write(&schema_path, schema).unwrap();
    for (name, body) in extra_files {
        let path = convex.join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, body).unwrap();
    }
    let out = tmp.path().join("out.rs");
    let cfg = Configuration {
        schema_path,
        out_file: out.clone(),
        convex_dir: convex,
        function_paths: Vec::new(),
        function_spec_path: None,
    };
    generate(cfg).unwrap();
    fs::read_to_string(&out).unwrap()
}

#[test]
fn golden_minimal_schema_and_function()
{
    let body = run_generate(MINIMAL_SCHEMA, &[("api.ts", MINIMAL_FN)]);
    insta::assert_snapshot!("minimal_schema_and_function", body);
}

#[test]
fn golden_duplicate_exports_unique_struct_names()
{
    let body = run_generate(
        MINIMAL_SCHEMA,
        &[
            ("users.ts", DUP_MOD_A),
            ("admin/users.ts", DUP_MOD_B),
            ("type/9-tools.ts", OPTIONAL_FN),
        ],
    );
    insta::assert_snapshot!("nested_api_duplicate_modules", body);
}

#[test]
fn golden_named_nested_object_structs()
{
    let body = run_generate(NESTED_OBJECT_SCHEMA, &[("integrations.ts", NESTED_OBJECT_FN)]);
    assert!(body.contains("pub struct ProjectsDocSettings"));
    assert!(body.contains("pub settings: ProjectsDocSettings"));
    assert!(body.contains("pub struct MirrorArgsFlags"));
    assert!(body.contains("pub flags: MirrorArgsFlags"));
    assert!(body.contains("pub const mirror:"));
    assert!(body.contains("\"integrations:integrationsMirror\""));
    insta::assert_snapshot!("scoped_function_model_names", body);
}

#[test]
fn golden_optional_root_omits_key_during_serde()
{
    let body = run_generate(MINIMAL_SCHEMA, &[("opts.ts", OPTIONAL_FN)]);
    assert!(body.contains("pub struct UpdateArgs"));
    assert!(body.contains("#[convex(rename = \"flag\", optional)]\n            pub flag: Option<bool>"));
}

#[test]
fn golden_v1_review_sources_and_spec()
{
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/v1-review");
    let tmp = tempdir().unwrap();
    for (name, spec) in [
        ("v1_review_sources", None),
        ("v1_review_spec", Some(fixture.join("function-spec.json"))),
    ] {
        let out = tmp.path().join("models.rs");
        generate(Configuration {
            schema_path: fixture.join("convex/schema.ts"),
            convex_dir: fixture.join("convex"),
            out_file: out.clone(),
            function_paths: Vec::new(),
            function_spec_path: spec,
        })
        .unwrap();
        insta::assert_snapshot!(name, fs::read_to_string(out).unwrap());
    }
}

#[test]
fn reports_identifier_collisions_with_both_wire_names()
{
    let tmp = tempdir().unwrap();
    let schema_path = tmp.path().join("schema.ts");
    let out_file = tmp.path().join("models.rs");
    fs::write(&out_file, "previous output").unwrap();
    fs::write(
        &schema_path,
        r#"export default defineSchema({ items: defineTable({ nested: v.object({ camelCase: v.string(), "camel-case": v.string() }) }) });"#,
    ).unwrap();
    let err = generate(Configuration {
        schema_path,
        convex_dir: tmp.path().into(),
        out_file: out_file.clone(),
        ..Configuration::default()
    })
    .unwrap_err()
    .to_string();
    assert!(err.contains("Rust identifier collision for `camel_case`"), "{err}");
    assert!(err.contains("camelCase") && err.contains("camel-case"), "{err}");
    assert_eq!(fs::read_to_string(out_file).unwrap(), "previous output");
}

#[test]
fn reports_module_spelling_collisions()
{
    let tmp = tempdir().unwrap();
    let convex = tmp.path();
    let schema_path = convex.join("schema.ts");
    fs::write(&schema_path, MINIMAL_SCHEMA).unwrap();
    fs::write(convex.join("foo-bar.ts"), DUP_MOD_A).unwrap();
    fs::write(convex.join("foo_bar.ts"), DUP_MOD_B).unwrap();
    let err = generate(Configuration {
        schema_path,
        convex_dir: convex.into(),
        out_file: convex.join("models.rs"),
        ..Configuration::default()
    })
    .unwrap_err()
    .to_string();
    assert!(err.contains("Rust identifier collision for `foo_bar`"), "{err}");
    assert!(err.contains("foo-bar") && err.contains("foo_bar"), "{err}");
}

#[test]
fn scoped_names_match_across_inputs_and_report_shortened_ref_collisions()
{
    let tmp = tempdir().unwrap();
    let convex = tmp.path().join("convex");
    fs::create_dir_all(convex.join("admin")).unwrap();
    let schema_path = convex.join("schema.ts");
    fs::write(&schema_path, MINIMAL_SCHEMA).unwrap();
    fs::write(
        convex.join("admin/user-tools.ts"),
        r#"
import { query } from "../_generated/server";
import { v } from "convex/values";
export const userToolsFind = query({
    args: {},
    returns: v.object({ detail: v.object({ ok: v.boolean() }) }),
    handler: () => ({ detail: { ok: true } }),
});
"#,
    )
    .unwrap();

    let spec = tmp.path().join("function-spec.json");
    let mut functions = serde_json::json!([{
        "identifier": "admin/user-tools.js:userToolsFind",
        "functionType": "Query",
        "visibility": {"kind": "public"},
        "args": {"type": "object", "value": {}},
        "returns": {"type": "object", "value": {
            "detail": {"optional": false, "fieldType": {"type": "object", "value": {
                "ok": {"optional": false, "fieldType": {"type": "boolean"}}
            }}}
        }}
    }]);
    fs::write(&spec, functions.to_string()).unwrap();

    let out_file = tmp.path().join("models.rs");
    let mut config = Configuration {
        schema_path,
        convex_dir: convex,
        out_file: out_file.clone(),
        ..Configuration::default()
    };
    generate(config.clone()).unwrap();
    let from_source = fs::read_to_string(&out_file).unwrap();
    config.function_spec_path = Some(spec.clone());
    generate(config.clone()).unwrap();
    assert_eq!(fs::read_to_string(&out_file).unwrap(), from_source);
    for expected in [
        "pub mod user_tools",
        "pub const find:",
        "pub struct FindArgs",
        "pub struct FindReturnDetail",
        "\"admin/user-tools:userToolsFind\"",
    ] {
        assert!(from_source.contains(expected), "missing {expected}");
    }

    functions.as_array_mut().unwrap().push(serde_json::json!({
        "identifier": "admin/user-tools.js:find",
        "functionType": "Query",
        "visibility": {"kind": "public"},
        "args": {"type": "any"},
        "returns": null
    }));
    fs::write(&spec, functions.to_string()).unwrap();
    let err = generate(config).unwrap_err().to_string();
    assert!(err.contains("Rust identifier collision for `find`"), "{err}");
    assert!(
        err.contains("admin/user-tools:find") && err.contains("admin/user-tools:userToolsFind"),
        "{err}"
    );
    assert_eq!(fs::read_to_string(out_file).unwrap(), from_source);
}
