# API stability

Quick reference for what is **stable** vs **unstable** in convex-typegen. Full rules: [semver-policy.md](semver-policy.md).

## Stable (1.0+)

| Item | Stability | Notes |
| --- | --- | --- |
| `generate(config) -> Result<(), ConvexTypeGeneratorError>` | Stable | Entry point for build scripts |
| `Configuration` | Stable | Additive fields only in minors |
| `Configuration::default()` | Stable | Default paths documented in readme |
| `rcfp(&Configuration) -> Result<Vec<PathBuf>, _>` | Stable | For `cargo:rerun-if-changed` |
| `ConvexTypeGeneratorError` | Stable | Additive variants in minors only |
| `prelude::*` | Stable | See below |
| `convex_typegen::serde` | Stable | Re-export for generated `#[serde(crate = ...)]` |
| `convex_typegen::serde_json` | Stable | Re-export for generated fallback value types |
| Feature `client` | Stable | Default on; disable for build-only |
| Feature `verbose` | Stable | Default off |

### Prelude (stable when `client` enabled)

| Re-export | Role |
| --- | --- |
| `Configuration`, `generate`, `rcfp` | Build pipeline |
| `ConvexTypeGeneratorError` | Error handling |
| `ConvexJsonValue` | Generated fallback type for unsupported/dynamic shapes |
| `Serialize`, `Deserialize` | Generated derives |
| `ConvexClientExt` | Generated args to Convex values runtime helper |
| `value_serde::{to_value, from_value, Bytes, Error}` | Direct Serde conversion to and from `convex::Value` |
| `ConvexBytes`, `ConvexValueSerdeError` | Prelude aliases for byte fields and conversion errors |

## Unstable / not semver-guaranteed

| Item | Notes |
| --- | --- |
| **Generated `convex_types.rs`** | Type names and fields can change on major crate bumps |
| **Internal modules** | `convex::*` is private; do not depend on it |
| **Exact error `Display` strings** | May improve in patches; match on variants, not text |
| **Oxc / Convex dependency versions** | May affect MSRV or build time; tracked in CHANGELOG |

## Generated code contract

Generated files include a header stating they are regenerated on each build. Do not edit by hand. Stability of generated **names and types** follows **codegen semver** in [semver-policy.md](semver-policy.md), not the library API table above.
