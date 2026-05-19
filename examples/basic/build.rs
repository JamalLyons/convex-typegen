//! Minimal `build.rs`: rerun-if-changed for schema, convex dir, and every discovered TS function,
//! then run [`convex_typegen::generate`] with defaults (`convex/schema.ts` → `src/convex_types.rs`).

use convex_typegen::prelude::*;

fn main() {
    let config = Configuration::default();

    println!("cargo:rerun-if-changed={}", config.schema_path.display());
    println!("cargo:rerun-if-changed={}", config.convex_dir.display());

    for path in rcfp(&config).expect("resolve convex function sources") {
        println!("cargo:rerun-if-changed={}", path.display());
    }

    if let Err(e) = generate(config) {
        panic!("convex-typegen failed: {e}");
    }
}
