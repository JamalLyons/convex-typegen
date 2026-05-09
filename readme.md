# convex-typegen

A blazing fast Rust type generator for [ConvexDB](https://www.convex.dev) schemas and functions.

## Features

- **Blazing Fast**: Efficient AST parsing and type generation using oxc
- **Auto-regeneration**: Types automatically update when schema or function files change
- **Complete Type System**: 
  - Full schema type generation (tables, columns, unions)
  - Function argument types for queries, mutations, and actions
  - Support for all Convex types (arrays, objects, records, literals)
  - Proper handling of optional fields and complex types
- **Type Safety**: 
  - Compile-time type checking
  - Automatic serialization/deserialization
  - Zero runtime overhead
- **Developer Experience**: 
  - Clean, idiomatic Rust code generation
  - Smart function path resolution (e.g., "auth:login")
  - Detailed documentation for generated types

## Quick Start

1. Add dependencies using cargo:

```bash
cargo add convex-typegen
cargo add --build convex-typegen
```

Generated Rust uses **serde** and **serde_json** through `convex-typegen` re-exports (`#[serde(crate = "convex_typegen::serde")]` and `convex_typegen::serde_json` in emitted code), so your app crate does not need its own `serde` / `serde_json` dependencies for the generated file alone. Add them only if you use those crates elsewhere in your code.

2. Add the following to your `build.rs` file:

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

By default, [`Configuration`](https://docs.rs/convex-typegen/latest/convex_typegen/struct.Configuration.html) uses `convex/` next to your `Cargo.toml` and discovers every `*.ts` function file under it (skipping the schema file, `_generated/`, `node_modules/`, and `*.d.ts`). Point `convex_dir` at another directory if your backend is not named `convex`, or set non-empty `function_paths` to override discovery entirely.

3. Run `cargo build` to generate the types.

You can watch a demo video [here](https://youtu.be/42-Ihov48AU) to learn more.

## Supported Types

- **Basic Types**: `string`, `number`, `boolean`, `null`, `int64`, `bytes`
- **Complex Types**: `array`, `object`, `record`, `union`, `optional`
- **Special Types**: `any`, `literal`, `id`
- **Custom Types**: Automatic enum generation for union types
- **`v.object({ ... })`**: If every field maps to the same Rust type (for example all `v.number()` → `BTreeMap<String, f64>`), that map type is used. If fields would need different Rust types, the column or arg is emitted as `serde_json::Value` so the binding stays honest; per-shape structs may be added later.

## Acknowledgments

- Built for use with [ConvexDB](https://convex.dev)
- Powered by [oxc](https://github.com/oxc-project/oxc) for TypeScript parsing

## Related Projects

- [convex rust sdk](https://docs.rs/convex/latest/convex/) - Official Rust client for ConvexDB

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request. For major changes, please open an issue first.

## Versioning

This project follows [Semantic Versioning](https://semver.org/) (SemVer) to manage releases. The versioning format is:

- **MAJOR** version is incremented for incompatible API changes or breaking changes.
- **MINOR** version is incremented for adding new features in a backward-compatible manner.
- **PATCH** version is incremented for backward-compatible bug fixes or minor changes that don't add new features.

For more details, refer to the [CHANGELOG](CHANGELOG.md).

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.