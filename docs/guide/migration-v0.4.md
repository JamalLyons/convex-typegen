# Migrate from v0.4

Historical release notes. For current usage, see the [v1 documentation](https://jamallyons.github.io/convex-typegen/).

v1 is a clean break from v0.4. There are **no aliases, re-export shims, compatibility features, or
legacy output paths**. Regenerate your bindings with the matching v1 generator and update imports
and calls using this map.

| v0.4 name or location | v1 replacement |
| --- | --- |
| One `convex-typegen` crate in both `[dependencies]` and `[build-dependencies]` | `convex-typegen-runtime` in `[dependencies]`; matching `convex-typegen` in `[build-dependencies]` |
| `convex_typegen::prelude::*` for generation and runtime client helpers | Generator APIs remain under `convex_typegen`; runtime APIs come from `convex_typegen_runtime` and its `prelude` |
| `convex_typegen::serde` and `convex_typegen::serde_json` in generated/application code | `convex_typegen_runtime::serde` and `convex_typegen_runtime::serde_json` |
| `convex_typegen::prelude::{Deserialize, Serialize, ConvexJsonValue, ConvexJsonError}` | Import Serde through `convex_typegen_runtime::serde` or the runtime prelude. `ConvexJsonValue` remains a JSON alias in the runtime prelude for JSON work; generated arbitrary Convex values use `model::RawValue`. `ConvexJsonError` has no replacement alias. |
| `convex_types.rs` default generated output | `$OUT_DIR/convex_api.rs` by default, included with `convex_typegen_runtime::include!()` |
| `mod convex_types;` for the default generated source file | `mod convex_api { convex_typegen_runtime::include!(); }` (or choose another Rust module name) |
| `Configuration::default()` and `generate(config)` called directly in a minimal build script | `convex_typegen::build::generate_out_dir()` for the default offline Cargo flow; use `build::generate(config)` for overrides/spec input |
| Flat `{Module}{Export}Args` names such as `GamesGetGameArgs` (v0.4 did not generate return models) | Path-scoped references and types in the same module, e.g. `api::games::get_game` with `api::games::GetGameArgs` and the new `GetGameReturn`; `admin/users:list` becomes `api::admin::users::list` with `ListArgs` / `ListReturn` |
| `FUNCTION_PATH` associated string | Generated reference `.path()`, e.g. `api::admin::users::list.path()` |
| `ConvexClientExt::prepare_args` and string-based `client.query`/`mutation`/`action` | `TypedClient` methods taking the generated reference and its associated args |
| `IntoConvexValue`, `ConvexValueExt`, and `ConvexJsonError` | Removed; use direct `value_serde::{to_value, from_value}` with `convex::Value` |
| Generated `TryFrom<Args> for BTreeMap<String, serde_json::Value>` | Removed; pass the args struct to `TypedClient`, which encodes Convex values directly |
| `ConvexJsonValue` for arbitrary/undeclared values | `model::RawValue`; undeclared args use `function::RawArgs` |
| Unstructured SDK errors | `CallError<E>` separates encode, decode, transport, server-message, and application errors; declare a thrown payload with `.with_error::<E>()` |
| The generator's default `client` feature and runtime re-exports from the generator | Removed. The generator is build-only; depend on the runtime for client/codecs/macros |
| Generator MSRV 1.95 for all crate use | Runtime MSRV 1.85; generator/build-script MSRV 1.97 |

## Upgrade steps

1. Replace the normal generator dependency with `convex-typegen-runtime`, and keep the matching
   `convex-typegen` release under `[build-dependencies]`.
2. Replace the old handwritten `build.rs` with `convex_typegen::build::generate_out_dir()` when
   using default paths. This writes to `OUT_DIR` and tracks the schema, Convex directory, and
   discovered function inputs. For a committed function spec or custom paths, use
   `build::generate(Configuration::default().with_convex_dir(...))` as shown in [getting started](https://jamallyons.github.io/convex-typegen/generation.html#use-a-custom-layout-or-monorepo).
3. Replace the old `mod convex_types;` declaration with the runtime `include!` macro, then
   regenerate bindings. Update flat names, paths, changed field types, and imports from compiler
   diagnostics. v1 generated names are scoped to the Convex module path.
4. Replace `prepare_args` and untyped SDK calls with `TypedClient`. Use
   `.with_error::<Payload>()` for a known application-error payload and handle
   `CallError::Application` with `..` for extensible fields and a fallback match arm.
   Use `FunctionRef::new` for handwritten endpoint references.
5. Run `cargo check` and the local self-hosted fixture gate, `just e2e`, from a clean checkout.

If you tried a v1 development build, replace configuration struct literals with
`Configuration::default().with_...` setters. An unset schema follows `convex_dir`.
Use `build::generate(config)` in place of manual `rcfp` tracking. Match
`convex_typegen::ConvexTypeGeneratorError` with a fallback arm. A missing schema
reports its attempted path and I/O errors use the `Io` variant.

Regenerated unions of distinct string literals use unit variants such as `Role::Admin`
in place of `Role::Branch1(RoleBranch1)`. The original string is still the wire value.

For unions of distinct string literals plus null, replace positional string
branches with `Some(Role::Admin)` and the null branch with `None`. On optional
fields, use `None` for absence, `Some(None)` for explicit null, and
`Some(Some(Role::Admin))` for a string. For a top-level nullable enum return
such as `GetRoleReturn`, match `Option` and use variants on `GetRoleReturnValue`.
The original string values, nulls, and omitted keys retain their wire meanings.

The v1 changes are a breaking runtime API and generated-output change under the
[semver policy](../semver-policy.md). The migration is intentionally manual; there is no shim to
make old imports compile.
