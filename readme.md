# convex-typegen

Rust types from your [Convex](https://www.convex.dev) `schema.ts` and function modules. Runs in `build.rs` so generated code stays in sync with your backend.

**Docs:** [docs.rs/convex-typegen](https://docs.rs/convex-typegen)

## Setup

1. Add the crate (library + build script):

```bash
cargo add convex-typegen
cargo add --build convex-typegen
```

2. Add `build.rs`:

```rust
use convex_typegen::prelude::*;

fn main() {
    let config = Configuration::default();

    println!("cargo:rerun-if-changed={}", config.schema_path.display());
    println!("cargo:rerun-if-changed={}", config.convex_dir.display());
    for path in resolved_function_paths(&config).expect("resolve convex function sources") {
        println!("cargo:rerun-if-changed={}", path.display());
    }

    if let Err(e) = generate(config) {
        panic!("convex-typegen failed: {e}");
    }
}
```

3. `cargo build` writes the generated file (see defaults below). Include it in your crate (`mod convex_types;` or whatever matches `out_file`) and pull in `convex_typegen::prelude::*` where you call Convex with the generated arg types.

A full minimal setup lives under [`examples/basic/`](examples/basic/) in this repo.

## Defaults

| Field | Default |
| --- | --- |
| `schema_path` | `convex/schema.ts` |
| `out_file` | `src/convex_types.rs` |
| `convex_dir` | `convex` |

Paths are relative to the package directory when Cargo runs the build script.

Function sources: every `*.ts` under `convex_dir`, except the schema file, `_generated/`, `node_modules/`, and `*.d.ts`. Set `function_paths` to a non-empty list to skip discovery and pass files explicitly.

## Serde

Generated code pulls serde and serde_json through `convex-typegen`, so a minimal app crate does not need those as direct dependencies unless you use them yourself.

## License

MIT — see [LICENSE](LICENSE).
