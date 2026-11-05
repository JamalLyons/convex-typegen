use std::env;
use std::path::PathBuf;

fn main()
{
    println!("cargo:rerun-if-env-changed=CONVEX_FUNCTION_SPEC_PATH");
    let fixture_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/v1-review");
    let spec_path = env::var_os("CONVEX_FUNCTION_SPEC_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|| fixture_root.join("function-spec.json"));
    let config = convex_typegen::config::Configuration::default()
        .with_convex_dir(fixture_root.join("convex"))
        .with_function_spec_path(spec_path);
    convex_typegen::build::generate(config).expect("generate OUT_DIR Convex bindings");
}
