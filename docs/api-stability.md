# API stability

Quick reference for what is **stable** vs **unstable** in convex-typegen. Full rules: [semver-policy.md](semver-policy.md).

## Stable (1.0+)

| Item | Stability | Notes |
| --- | --- | --- |
| `generate(config)` | Stable | Write the configured artifact |
| `check(config)` | Stable | Check for drift without writing |
| `config::Configuration` | Stable | Non-exhaustive. Construct with `Default` and chainable setters |
| `build::generate(config)` | Stable | Offline Cargo generation with input tracking |
| `build::generate_out_dir()` | Stable | Standard Convex layout and `$OUT_DIR/convex_api.rs` |
| `ConvexTypeGeneratorError` | Stable | Crate-root error export with a non-exhaustive variant set and fields |
| `prelude::*` | Stable | See below |
| Feature `verbose` | Stable | Default off |

The generator prelude exports `Configuration`, `generate`, `check`, and `ConvexTypeGeneratorError`.

### Runtime crate

| Export | Role |
| --- | --- |
| `ConvexValue`, `include!` | Derive for Convex wire models and helper for including generated bindings |
| `convex` | Re-export of the SDK used by the runtime |
| `serde`, `serde_json` | Re-exports used by generated bindings |
| `TypedClient`, `CallError`, `TransportError`, `InvalidApplicationData` | Typed calls, subscriptions, and errors over the official SDK |
| `function` | Typed endpoint references, kind markers, raw args, and the sealed `FunctionReference` trait |
| `model::{Doc, Id, RawValue, Table}` | Document models, typed IDs, raw wire values, and table markers |
| `value_serde::{to_value, from_value, Bytes, Error}` | Direct Serde conversion to and from `convex::Value` |
| `prelude::*` | Serde derives, typed client, codecs, and `ConvexJsonValue`, `ConvexBytes`, and `ConvexValueSerdeError` aliases |

Runtime errors are non-exhaustive. Match `CallError` with a fallback and include `..`
when matching any variant or destructuring `InvalidApplicationData`.
The runtime exposes official SDK types directly. An incompatible update to a publicly
exposed `convex` 0.x type requires a runtime major release.

## Unstable / not semver-guaranteed

| Item | Notes |
| --- | --- |
| **Generated `convex_api.rs`** | Type names and fields can change on major crate bumps |
| **Internal modules** | `convex_typegen::convex` is private; do not depend on it |
| **Exact error `Display` strings** | May improve in patches; match on variants, not text |
| **Dependency versions** | Private dependency updates can affect build time. Public SDK type and MSRV changes follow the semver policy. |

## Generated code contract

Generated files include a header stating they are regenerated on each build. Do not edit by hand. Stability of generated **names and types** follows **codegen semver** in [semver-policy.md](semver-policy.md), not the library API table above.
