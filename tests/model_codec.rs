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
    fs::write(
        convex.join("models.ts"),
        r#"
        import { query } from "./_generated/server";
        import { v } from "convex/values";
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
    generate(Configuration {
        schema_path: convex.join("schema.ts"),
        convex_dir: convex.into(),
        out_file: out,
        ..Configuration::default()
    })
    .unwrap();

    let cases = trybuild::TestCases::new();
    cases.pass("tests/ui/model_round_trip.rs");
    cases.compile_fail("tests/ui/wrong_table_id.rs");
    cases.compile_fail("tests/ui/unescaped_identifier.rs");
    cases.compile_fail("tests/ui/wrong_function_args.rs");
    cases.compile_fail("tests/ui/internal_function.rs");
    cases.compile_fail("tests/ui/wrong_function_kind.rs");
    cases.compile_fail("tests/ui/subscribe_non_query.rs");
}
