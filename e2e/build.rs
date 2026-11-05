use std::env;
use std::path::PathBuf;

fn main()
{
    println!("cargo:rerun-if-env-changed=CONVEX_FUNCTION_SPEC_PATH");
    let fixture_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/v1-review");
    let spec_path = env::var_os("CONVEX_FUNCTION_SPEC_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|| fixture_root.join("function-spec.json"));
    let convex_dir = fixture_root.join("convex");
    let schema_path = convex_dir.join("schema.ts");
    let out_file = PathBuf::from(env::var_os("OUT_DIR").expect("Cargo sets OUT_DIR")).join("convex_api.rs");

    let config = convex_typegen::config::Configuration {
        schema_path,
        out_file,
        convex_dir,
        function_paths: Vec::new(),
        function_spec_path: Some(spec_path),
    };
    for path in convex_typegen::prelude::rcfp(&config).expect("resolve generator inputs") {
        println!("cargo:rerun-if-changed={}", path.display());
    }
    convex_typegen::generate(config).expect("generate OUT_DIR Convex bindings");
}
