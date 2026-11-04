# convex-typegen

Rust types from your [Convex](https://www.convex.dev) `schema.ts` and function modules. Runs in `build.rs` so generated code stays in sync with your backend.

**Docs:** [docs.rs/convex-typegen](https://docs.rs/convex-typegen)

## Setup

1. Add the runtime as a normal dependency and the matching generator release as a build dependency.

```bash
cargo add convex-typegen-runtime
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

3. `cargo build` writes the generated file (see defaults below). Include it in your crate (`mod convex_types;` or whatever matches `out_file`) and pull in `convex_typegen_runtime::prelude::*` where you call Convex with the generated arg types.

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
| `function_spec_path` | `None` |

Paths are relative to the package directory when Cargo runs the build script.

Function sources: every `*.ts` under `convex_dir`, except the schema file, `_generated/`, `node_modules/`, and `*.d.ts`. Set `function_paths` to a non-empty list to skip discovery and pass files explicitly.

To use committed output from `npx convex function-spec` as the complete function inventory:

```rust,ignore
let config = Configuration {
    function_spec_path: Some("convex/function-spec.json".into()),
    ..Default::default()
};
```

The spec replaces TypeScript function discovery and parsing; the schema remains TypeScript. `rcfp`
returns the spec path so a build script can emit `cargo:rerun-if-changed` for it. Builds only read
the committed JSON and never run the Convex CLI or contact a backend. Refresh the v1 review capture
with `scripts/refresh-function-spec.sh` after deploying that fixture to the documented local backend.

## Features

| Feature | Default | Description |
| --- | --- | --- |
| `verbose` | off | Print Oxc diagnostics to stderr during parse failures. |

## Runtime

`convex-typegen-runtime` uses the official `convex` client. Its
`value_serde::{to_value, from_value}` helpers convert directly against `convex::Value`.
Use `value_serde::Bytes` for byte strings. Generated optional fields omit outer `None`.
Use `Option<Option<T>>` to encode explicit `Null` as `Some(None)`.

Generated code imports Serde and JSON through the runtime. If your own code imports the
official client directly, declare `convex` in your dependencies too.

When migrating, replace the normal `convex-typegen` dependency with `convex-typegen-runtime`,
move runtime imports to `convex_typegen_runtime`, remove any generator `client` feature flags,
and regenerate your bindings. Keep the generator in `[build-dependencies]` for offline codegen.

## Versioning

- [Semantic versioning policy](docs/semver-policy.md) — library API vs generated output, MSRV, and release rules.
- [API stability reference](docs/api-stability.md) — stable vs unstable public items.

Use the same release version of `convex-typegen-runtime` and `convex-typegen`. The runtime supports Rust **1.85**. Building from source with the generator requires Rust **1.95**, including when it runs from your build script. Projects that consume pregenerated bindings can depend on the runtime alone. When resolving a fresh lockfile on Rust 1.85, use Cargo resolver 3 to prefer compatible transitive dependencies. Edition 2024 uses it by default outside a workspace. On **0.x**, breaking codegen may ship in a **minor** (this crate, 0.4.0). From **1.0**, generated type renames require a **major** bump.

## Roadmap

v1 is a generated, fully typed function client: the Rust equivalent of Convex’s `api.*` (typed refs, args and return types, `Id<T>`, `Doc<T>`, subscriptions as streams). The picture, today’s gaps, and the open decisions are in [docs/v1-roadmap.md](docs/v1-roadmap.md).

Contributors and coding agents: start with [AGENTS.md](AGENTS.md), [CONTRIBUTING.md](CONTRIBUTING.md), and [docs/architecture.md](docs/architecture.md).

## License

MIT — see [LICENSE](https://github.com/JamalLyons/convex-typegen/blob/master/LICENSE).
