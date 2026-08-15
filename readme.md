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

```rust,ignore
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
```

3. `cargo build` writes the generated file (see defaults below). Include it in your crate (`mod convex_types;` or whatever matches `out_file`) and pull in `convex_typegen::prelude::*` where you call Convex with the generated arg types.

A full minimal setup lives in [examples/basic](https://github.com/JamalLyons/convex-typegen/tree/master/examples/basic) in this repo.

## Generated names

Function argument structs are named **`{Module}{Export}Args`** when the export does not already start with the module file name (without `.ts`). Examples:

| Module file | Export | Generated struct |
| --- | --- | --- |
| `games.ts` | `getGame` | `GamesGetGameArgs` |
| `mod_a.ts` | `list` | `ModAListArgs` |
| `tasks.ts` | `tasksSearch` | `TasksSearchArgs` |

`FUNCTION_PATH` strings are unchanged (e.g. `"games:getGame"`).

Nested `v.object({ ... })` validators emit their own `pub struct` (nesting depth under 8). Empty objects and deeper nesting still fall back to `ConvexJsonValue`.

| Source | Example | Generated struct |
| --- | --- | --- |
| Schema column | `projects.settings` | `ProjectsSettings` |
| Function arg object | `integrations.ts` `integrationsMirror` arg `flags` | `IntegrationsMirrorFlags` |
| Nested object field | parent `ProjectsSettings` + field `theme` | `ProjectsSettingsTheme` |
| Array of objects | column `items` whose elements are objects | `{Table}{Column}Element` |

Identical shapes under the same name are reused; conflicting shapes under the same name fail codegen with `InvalidSchema`.

## Defaults

| Field | Default |
| --- | --- |
| `schema_path` | `convex/schema.ts` |
| `out_file` | `src/convex_types.rs` |
| `convex_dir` | `convex` |

Paths are relative to the package directory when Cargo runs the build script.

Function sources: every `*.ts` under `convex_dir`, except the schema file, `_generated/`, `node_modules/`, and `*.d.ts`. Set `function_paths` to a non-empty list to skip discovery and pass files explicitly.

## Features

| Feature | Default | Description |
| --- | --- | --- |
| `client` | on | Re-exports `ConvexClientExt`, `IntoConvexValue`, `ConvexValueExt` (pulls in the `convex` crate). |
| `verbose` | off | Print Oxc diagnostics to stderr during parse failures. |

Build-only usage (smaller dependency tree):

```toml
[build-dependencies]
convex-typegen = { version = "0.4", default-features = false }
```

## Serde

Generated code pulls serde and serde_json through `convex-typegen`, so a minimal app crate does not need those as direct dependencies unless you use them yourself.

## Versioning

- [Semantic versioning policy](docs/semver-policy.md) — library API vs generated output, MSRV, and release rules.
- [API stability reference](docs/api-stability.md) — stable vs unstable public items.

Pin the same crate version in `[dependencies]` and `[build-dependencies]`. On **0.x**, breaking codegen may ship in a **minor** (this crate, 0.4.0). From **1.0**, generated type renames require a **major** bump.

## License

MIT — see [LICENSE](https://github.com/JamalLyons/convex-typegen/blob/master/LICENSE).
