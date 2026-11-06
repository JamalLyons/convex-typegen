# Semantic versioning policy

From **1.0.0** onward, [convex-typegen](https://github.com/JamalLyons/convex-typegen) follows
[SemVer 2.0](https://semver.org/) on the crate version. Earlier 0.x releases shipped breaking
changes in minor versions; that history lives in the
[CHANGELOG](https://github.com/JamalLyons/convex-typegen/blob/master/CHANGELOG.md).

## Two surfaces, two semver stories

| Surface | What it is | Semver |
| --- | --- | --- |
| **Library API** | Rust items you import from `convex_typegen` (`generate`, `Configuration`, errors, prelude, features) or `convex_typegen_runtime` (client helpers, codecs, generated-code support) | Standard semver on the crate version |
| **Generated output** | `convex_api.rs` (or your `out_file`) emitted at build time | **Not** the published Rust API, but intentional shape changes are **breaking for your app** and bump the **crate major** |

Use matching release versions of `convex-typegen` and `convex-typegen-runtime`. Put the generator
in `[build-dependencies]` and the runtime in `[dependencies]`. Regenerate bindings whenever you
upgrade so generated code and runtime stay in sync.

## Library API

These items are semver-stable from 1.0.0:

- `convex_typegen::{generate, check}`
- `convex_typegen::config::Configuration`, its fields, and its setters
- `convex_typegen::build::{generate, generate_out_dir}`
- `convex_typegen::ConvexTypeGeneratorError`
- `convex_typegen::prelude` re-exports
- Generator Cargo feature `verbose`
- Runtime `TypedClient`, `CallError`, `TransportError`, `InvalidApplicationData`, `function`, `model`, `value_serde`, and `prelude`
- Runtime macros `ConvexValue` and `include!`, and crate-root `convex`, `serde`, and `serde_json` re-exports

The runtime wraps the official SDK directly. An incompatible update of a publicly exposed
`convex` 0.x type (for example, `ConvexClient` or `Value`) requires a runtime major release.

See [api-stability.md](api-stability.md) for a per-item table.

## Generated output (codegen semver)

Treat **any intentional change** to generated Rust that breaks consumer code as requiring a **major**
crate bump:

- Renaming or removing `pub struct` / `pub enum` types
- Changing field types on generated structs
- Changing `TryFrom` / `Serialize` behavior for args structs
- Renaming or removing generated `api` references

**Non-breaking codegen** (minor crate bump):

- New generated types for newly added Convex functions or tables only (existing names unchanged)
- Additional `#[allow(...)]` or comments in the generated header
- Bug fixes that make previously uncompilable output compile **without** renaming existing types

When in doubt, bump **major**.

## Version bump rules

| Change | Bump |
| --- | --- |
| New variant on a non-exhaustive public error enum | Minor |
| New optional field on `Configuration` with a default | Minor |
| New Cargo feature, **default off** | Minor |
| New Cargo feature, **default on** | Major (or avoid; use default off) |
| Remove/rename `Configuration` field or `generate` signature change | Major |
| MSRV increase (`rust-version` in `Cargo.toml`) | **Major** |
| Generated type rename or field type change for existing Convex exports | **Major** |
| Dependency-only update with no API/codegen change | Patch or minor (note in CHANGELOG) |

## MSRV policy

- The generator requires Rust **1.97**. The runtime and derive crate require Rust **1.85**, matching the official Convex client. A project that runs the generator in `build.rs` requires Rust 1.97. Pregenerated bindings can use the matching runtime on Rust 1.85.
- Raising MSRV is a **major** version change.
- CI verifies each crate at its declared MSRV.

## Maintainer releases

Default branch is **`master`**.

1. Update `CHANGELOG.md` and `workspace.package.version` in `Cargo.toml`.
2. Add `docs/release-vX.Y.Z.md`, merge to `master`, tag `vX.Y.Z`, and create a GitHub Release.
3. Publish to crates.io with `cargo publish --locked` (or `just publish`). Publish the macro crate before the runtime and keep its exact version requirement in
   `runtime/Cargo.toml` in sync with the workspace version. There is no automated publish workflow.

Contributors: add user-visible changes under `[Unreleased]` in `CHANGELOG.md`.
