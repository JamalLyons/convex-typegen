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
    run_generate_with(schema, extra_files, true)
}

fn run_generate_with(schema: &str, extra_files: &[(&str, &str)], nullable_as_option: bool) -> String
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
    let cfg = Configuration::default()
        .with_schema_path(schema_path)
        .with_out_file(out.clone())
        .with_convex_dir(convex)
        .with_nullable_as_option(nullable_as_option);
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
        let mut config = Configuration::default()
            .with_convex_dir(fixture.join("convex"))
            .with_out_file(out.clone());
        if let Some(spec) = spec {
            config = config.with_function_spec_path(spec);
        }
        generate(config).unwrap();
        let generated = fs::read_to_string(out).unwrap();
        assert!(!generated.contains("Branch2(())"), "{name} kept a null branch:\n{generated}");
        insta::assert_snapshot!(name, generated);
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
    let err = generate(
        Configuration::default()
            .with_schema_path(schema_path)
            .with_convex_dir(tmp.path())
            .with_out_file(out_file.clone()),
    )
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
    let err = generate(
        Configuration::default()
            .with_schema_path(schema_path)
            .with_convex_dir(convex)
            .with_out_file(convex.join("models.rs")),
    )
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
    let config = Configuration::default()
        .with_schema_path(schema_path)
        .with_convex_dir(convex)
        .with_out_file(out_file.clone());
    generate(config.clone()).unwrap();
    let from_source = fs::read_to_string(&out_file).unwrap();
    let config = config.with_function_spec_path(&spec);
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

#[test]
fn golden_string_literal_enums()
{
    let schema = r#"
import { defineSchema, defineTable } from "convex/server";
import { v } from "convex/values";
export default defineSchema({
    users: defineTable({
        role: v.union(v.literal("admin"), v.literal("member")),
    }),
});
"#;
    let endpoint = r#"
import { query } from "./_generated/server";
import { v } from "convex/values";
const role = v.union(v.literal("member"), v.literal("admin"));
export const roleName = query({
    args: { role, nested: v.optional(v.object({ selected: v.optional(v.nullable(role)), role: v.nullable(role) })) },
    returns: v.union(v.literal(""), v.literal("!"), v.literal("9lives"), v.literal("self"),
                     v.literal("type"), v.literal("Admin"), v.literal("admin"), v.literal("🦀"), v.null()),
    handler: () => "",
});
"#;
    insta::assert_snapshot!("string_literal_enums", run_generate(schema, &[("users.ts", endpoint)]));
}

#[test]
fn non_identifier_literals_do_not_rename_semantic_variants()
{
    for extra in ["", r#", v.literal("!"), v.literal("wire21")"#] {
        let schema = format!(
            r#"export default defineSchema({{
                items: defineTable({{ kind: v.union(v.literal("type"), v.literal("other"){extra}) }}),
            }});"#
        );
        let generated = run_generate(&schema, &[]);
        assert!(generated.contains("#[convex(literal = \"type\")]\n    Type,"));
        if !extra.is_empty() {
            assert!(generated.contains("#[convex(literal = \"!\")]\n    _Wire21,"));
            assert!(generated.contains("#[convex(literal = \"wire21\")]\n    Wire21,"));
        }
    }
}

#[test]
fn nullable_enum_names_survive_reordering_and_duplicate_literals_keep_union_branches()
{
    for literals in [
        r#"v.literal("read-only"), v.null(), v.literal("type")"#,
        r#"v.literal("type"), v.literal("read-only"), v.null()"#,
    ] {
        let schema = format!(
            r#"export default defineSchema({{
                items: defineTable({{
                    kind: v.union({literals}),
                    duplicate: v.union(v.literal("same"), v.null(), v.literal("same")),
                    mixed: v.union(v.literal("some"), v.null(), v.string()),
                    nullOnly: v.union(v.null(), v.null()),
                    single: v.nullable(v.literal("only")),
                }}),
            }});"#
        );
        let generated = run_generate(&schema, &[]);
        assert!(generated.contains("pub kind: Option<ItemsDocKind>"));
        assert!(generated.contains("#[convex(literal = \"read-only\")]\n    ReadOnly,"));
        assert!(generated.contains("#[convex(literal = \"type\")]\n    Type,"));
        assert!(generated.contains("pub single: Option<ItemsDocSingle>"));
        assert!(generated.contains("Branch3(ItemsDocDuplicateBranch3)"));
        assert!(generated.contains("pub mixed: ItemsDocMixed"));
        assert!(generated.contains("pub null_only: ItemsDocNullOnly"));
    }
}

const NULLABLE_SCHEMA: &str = r#"
import { defineSchema, defineTable } from "convex/server";
import { v } from "convex/values";
export default defineSchema({
    notes: defineTable({
        title: v.nullable(v.string()),
        pinnedAt: v.optional(v.nullable(v.int64())),
        parent: v.union(v.null(), v.id("notes")),
        meta: v.nullable(v.object({ words: v.int64() })),
        tags: v.array(v.nullable(v.string())),
        scores: v.record(v.string(), v.nullable(v.float64())),
        doubled: v.union(v.string(), v.null(), v.null()),
        nested: v.nullable(v.union(v.boolean(), v.null())),
        role: v.nullable(v.union(v.literal("owner"), v.literal("viewer"))),
        mixed: v.union(v.string(), v.int64(), v.null()),
        nullOnly: v.union(v.null(), v.null()),
    }),
});
"#;

const NULLABLE_FN: &str = r#"
import { query } from "./_generated/server";
import { v } from "convex/values";
const note = v.object({
    _id: v.id("notes"),
    _creationTime: v.number(),
    title: v.nullable(v.string()),
    pinnedAt: v.optional(v.nullable(v.int64())),
    parent: v.union(v.null(), v.id("notes")),
    meta: v.nullable(v.object({ words: v.int64() })),
    tags: v.array(v.nullable(v.string())),
    scores: v.record(v.string(), v.nullable(v.float64())),
    doubled: v.union(v.string(), v.null(), v.null()),
    nested: v.nullable(v.union(v.boolean(), v.null())),
    role: v.nullable(v.union(v.literal("owner"), v.literal("viewer"))),
    mixed: v.union(v.string(), v.int64(), v.null()),
    nullOnly: v.union(v.null(), v.null()),
});
export const get = query({
    args: { id: v.id("notes"), cursor: v.optional(v.nullable(v.string())) },
    returns: v.nullable(note),
    handler: () => null,
});
export const summary = query({
    args: {},
    returns: v.nullable(v.object({ count: v.int64() })),
    handler: () => null,
});
export const lastTitle = query({ args: {}, returns: v.nullable(v.string()), handler: () => null });
"#;

#[test]
fn golden_nullable_as_option()
{
    let generated = run_generate(NULLABLE_SCHEMA, &[("notes.ts", NULLABLE_FN)]);
    for expected in [
        "pub title: Option<String>",
        "#[convex(rename = \"pinnedAt\", optional)]\n    pub pinned_at: Option<Option<i64>>",
        "pub parent: Option<Id<tables::Notes>>",
        "pub meta: Option<NotesDocMeta>",
        "pub struct NotesDocMeta {",
        "pub tags: Vec<Option<String>>",
        "pub scores: ::std::collections::BTreeMap<String, Option<f64>>",
        "pub doubled: Option<String>",
        "pub nested: Option<bool>",
        "pub role: Option<NotesDocRole>",
        "pub mixed: NotesDocMixed",
        "pub null_only: NotesDocNullOnly",
        "pub cursor: Option<Option<String>>",
        "pub type GetReturn = Option<::convex_typegen_runtime::model::Doc<super::super::tables::Notes>>;",
        "pub struct SummaryReturnValue {",
        "pub type SummaryReturn = Option<SummaryReturnValue>;",
        "pub type LastTitleReturn = Option<String>;",
    ] {
        assert!(generated.contains(expected), "missing {expected:?} in:\n{generated}");
    }
    for removed in [
        "NotesDocTitle",
        "NotesDocMetaBranch1",
        "GetReturnValue",
        "LastTitleReturnValue",
    ] {
        assert!(!generated.contains(removed), "unexpected {removed:?} in:\n{generated}");
    }
    insta::assert_snapshot!("nullable_as_option", generated);

    // Opting out keeps the 1.0 shape.
    let legacy = run_generate_with(NULLABLE_SCHEMA, &[("notes.ts", NULLABLE_FN)], false);
    assert!(legacy.contains("pub title: NotesDocTitle"));
    assert!(legacy.contains("pub enum GetReturn {"));
    assert!(legacy.contains("Branch2(())"));
    assert!(legacy.contains("pub role: Option<NotesDocRole>"));
    insta::assert_snapshot!("nullable_branches_legacy", legacy);
}
