//! Compile and exercise the actual bindings generated from the reference project.

use std::fs;
use std::path::Path;

use convex_typegen::config::Configuration;
use convex_typegen::generate;
use tempfile::tempdir;

#[test]
fn generated_models_compile_and_preserve_wire_semantics()
{
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixture = root.join("tests/fixtures/v1-review/convex");
    let temp = tempdir().unwrap();
    let convex = temp.path();
    fs::create_dir(convex.join("admin")).unwrap();
    for file in ["schema.ts", "users.ts", "admin/users.ts", "validators.ts"] {
        fs::copy(fixture.join(file), convex.join(file)).unwrap();
    }
    let schema = convex.join("schema.ts");
    let source = fs::read_to_string(&schema).unwrap().replace(
        "export default defineSchema({",
        r#"export default defineSchema({
            users: defineTable({ role: v.union(v.literal("admin"), v.null(), v.literal("member")) }),"#,
    );
    fs::write(schema, source).unwrap();
    fs::write(
        convex.join("models.ts"),
        r#"
        import { query } from "./_generated/server";
        import { v } from "convex/values";
        const roles = v.union(v.literal("admin"), v.literal("member"));
        export const role = query({
            args: { role: roles, nested: v.optional(v.object({
                selected: v.optional(v.nullable(roles)), maybe: v.nullable(roles),
            })) },
            returns: v.union(v.literal("member"), v.literal("admin")),
            handler: (_ctx, { role }) => role,
        });
        export const edges = query({
            args: { roles: v.array(v.nullable(v.literal("admin"))) },
            returns: v.union(v.literal(""), v.literal("!"), v.literal("9lives"),
                v.literal("self"), v.literal("type"), v.literal("Admin"), v.literal("admin"),
                v.literal("read-only"), v.literal("read_only"), v.literal("🦀"), v.null()),
            handler: () => "",
        });
        export const measurements = query({
            args: { readings: v.optional(v.array(v.record(v.string(), v.object({ value: v.float64() })))) },
            handler: () => null,
        });
        export const maybe = query({
            args: {
                parent: v.union(v.null(), v.id("users")),
                meta: v.optional(v.nullable(v.object({ words: v.int64() }))),
            },
            returns: v.nullable(v.object({ title: v.nullable(v.string()) })),
            handler: () => null,
        });
        export const roundTrip = query({
            args: v.object({
                userId: v.id("users"),
                camelCase: v.object({
                    "some-key": v.optional(v.nullable(v.string())),
                    "type": v.boolean(),
                    "9lives": v.int64(),
                    "self": v.string(),
                }),
                literal: v.union(v.literal("1"), v.union(v.literal(1), v.literal(1n)), v.literal(true)),
                buffer: v.union(v.bytes(), v.array(v.int64())),
                byKey: v.record(v.union(v.literal("one"), v.literal("two")), v.bytes()),
                byId: v.record(v.id("tasks"), v.int64()),
                raw: v.optional(v.any()),
            }),
            returns: v.union(v.number(), v.int64()),
            handler: () => 1,
        });
        "#,
    )
    .unwrap();
    fs::create_dir(convex.join("type")).unwrap();
    fs::write(
        convex.join("type/9-tools.ts"),
        r#"
        import { query } from "../_generated/server";
        import { v } from "convex/values";
        export const value = query({ args: {}, returns: v.literal("test"), handler: () => "test" });
        "#,
    )
    .unwrap();
    let out = root.join("target/model-tests/models.rs");
    fs::create_dir_all(out.parent().unwrap()).unwrap();
    generate(Configuration::default().with_convex_dir(convex).with_out_file(out)).unwrap();

    let cases = trybuild::TestCases::new();
    cases.pass("tests/ui/model_round_trip.rs");
    // Exact diagnostics are tested by scripts/generator-msrv.sh on Rust 1.97.0.
    if std::env::var_os("CONVEX_TYPEGEN_UI_SNAPSHOTS").is_some() {
        cases.compile_fail("tests/ui/convex_value_unsupported.rs");
        cases.compile_fail("tests/ui/wrong_table_id.rs");
        cases.compile_fail("tests/ui/unescaped_identifier.rs");
        cases.compile_fail("tests/ui/wrong_function_args.rs");
        cases.compile_fail("tests/ui/internal_function.rs");
        cases.compile_fail("tests/ui/wrong_function_kind.rs");
        cases.compile_fail("tests/ui/subscribe_non_query.rs");
    }
}
