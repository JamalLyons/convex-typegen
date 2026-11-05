# API stability

Quick reference for what is **stable** vs **unstable** in convex-typegen. Full rules: [semver-policy.md](semver-policy.md).

## Stable (1.0+)

| Item | Stability | Notes |
| --- | --- | --- |
| `generate(config) -> Result<(), ConvexTypeGeneratorError>` | Stable | Entry point for build scripts |
| `Configuration` | Stable | Additive fields only in minors; `function_spec_path` is optional and defaults to `None` |
| `Configuration::default()` | Stable | Default paths documented in readme |
| `rcfp(&Configuration) -> Result<Vec<PathBuf>, _>` | Stable | For `cargo:rerun-if-changed` |
| `ConvexTypeGeneratorError` | Stable | Additive variants in minors only |
| `prelude::*` | Stable | See below |
| Feature `verbose` | Stable | Default off |

The generator prelude exports `Configuration`, `generate`, `rcfp`, and
`ConvexTypeGeneratorError` for build scripts.

### Runtime crate

| Export | Role |
| --- | --- |
| `ConvexValue`, `include!` | Derive for Convex wire models and helper for including generated bindings |
| `serde`, `serde_json` | Re-exports used by generated bindings |
| `TypedClient`, `CallError`, `InvalidApplicationData` | Typed calls, subscriptions, and errors over the official SDK |
| `function` | Typed endpoint references, kind markers, and raw args |
| `value_serde::{to_value, from_value, Bytes, Error}` | Direct Serde conversion to and from `convex::Value` |
| `prelude::*` | Serde derives, typed client, codecs, and `ConvexJsonValue`, `ConvexBytes`, and `ConvexValueSerdeError` aliases |

## Unstable / not semver-guaranteed

| Item | Notes |
| --- | --- |
| **Generated `convex_types.rs`** | Type names and fields can change on major crate bumps |
| **Internal modules** | `convex::*` is private; do not depend on it |
| **Exact error `Display` strings** | May improve in patches; match on variants, not text |
| **Oxc / Convex dependency versions** | May affect MSRV or build time; tracked in CHANGELOG |

## Generated code contract

Generated files include a header stating they are regenerated on each build. Do not edit by hand. Stability of generated **names and types** follows **codegen semver** in [semver-policy.md](semver-policy.md), not the library API table above.
