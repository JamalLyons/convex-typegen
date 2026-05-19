//! Smoke test mirroring a consumer `build.rs` without compiling a nested crate.

use std::fs;

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
    };

    let paths = rcfp(&config).expect("resolve convex function sources");
    assert_eq!(paths.len(), 1);
    assert!(paths[0].ends_with("api.ts"));

    generate(config).expect("generate");
    let body = fs::read_to_string(&out).unwrap();
    assert!(body.contains("ApiPingArgs"));
    assert!(body.contains("TTable"));
}
