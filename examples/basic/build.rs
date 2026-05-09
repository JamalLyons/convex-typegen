use convex_typegen::{generate, Configuration};

fn main() {
    // Rebuild if the schema or games files change
    println!("cargo:rerun-if-changed=convex/schema.ts");
    println!("cargo:rerun-if-changed=convex/games.ts");

    let config = Configuration {
        function_paths: vec![std::path::PathBuf::from("convex/games.ts")],
        ..Default::default()
    };

    // Generate the types
    if let Err(e) = generate(config) {
        panic!("convex-typegen failed: {e}");
    }
}
