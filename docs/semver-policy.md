# Semantic versioning policy

This document defines how [convex-typegen](https://github.com/JamalLyons/convex-typegen) versions releases. It applies from **0.4.0** onward and is the target policy for **1.0.0**.

## Two surfaces, two semver stories

| Surface | What it is | Semver |
| --- | --- | --- |
| **Library API** | Rust items you import from `convex_typegen` (`generate`, `Configuration`, errors, prelude, features) or `convex_typegen_runtime` (client helpers, codecs, generated-code support) | Standard [SemVer 2.0](https://semver.org/) on the crate version |
| **Generated output** | `convex_types.rs` (or your `out_file`) emitted at build time | **Not** the crate’s public API, but changes are **breaking for your app** — we bump the **crate major** when we intentionally change generated shapes |

Matching release versions of `convex-typegen` and `convex-typegen-runtime` are supported. Declare the generator in `[build-dependencies]` and the runtime in `[dependencies]`. After upgrading, regenerate the bindings against the matching runtime.

## Library API (stable at 1.0)

These items are covered by semver once we tag **1.0.0**:

- `convex_typegen::generate`
- `convex_typegen::config::Configuration` and its fields (additive fields only in minors)
- `convex_typegen::fs::rcfp`
- `convex_typegen::error::ConvexTypeGeneratorError` (new variants = minor; removed/renamed variants = major)
- `convex_typegen::prelude` re-exports
- Generator Cargo feature `verbose`
- Runtime `TypedClient`, `CallError`, `InvalidApplicationData`, `function`, `value_serde`, and `prelude`
- Runtime macros `ConvexValue` and `include!`, and crate-root `serde` / `serde_json` re-exports

See [api-stability.md](api-stability.md) for a per-item table.

## Generated output (codegen semver)

Treat **any intentional change** to generated Rust that breaks consumer code as requiring a **major** crate bump:

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
| New `ConvexTypeGeneratorError` variant | Minor |
| New optional field on `Configuration` with a default | Minor |
| New Cargo feature, **default off** | Minor |
| New Cargo feature, **default on** | Major (or avoid; use default off) |
| Remove/rename `Configuration` field or `generate` signature change | Major |
| MSRV increase (`rust-version` in `Cargo.toml`) | **Major** |
| Generated type rename or field type change for existing Convex exports | **Major** |
| Dependency-only update with no API/codegen change | Patch or minor (note in CHANGELOG) |

## MSRV policy

- The generator requires Rust **1.95**. The runtime and derive crate require Rust **1.85**, matching the official Convex client. A project that runs the generator in `build.rs` requires Rust 1.95. Pregenerated bindings can use the matching runtime on Rust 1.85.
- Raising MSRV is a **major** version change.
- CI verifies each crate at its declared MSRV.

## Pre-1.0 history

Versions **0.x** may ship breaking changes in **minor** releases when needed for rapid improvement. From **1.0.0** onward, this policy applies strictly.

## 1.0.0 readiness checklist

- [x] Semver and API stability documents published
- [x] Oxc AST path evaluated ([oxc-ast-evaluation.md](oxc-ast-evaluation.md))
- [x] Named nested object structs implemented (see CHANGELOG)
- [x] CHANGELOG and release notes reference this policy
- [ ] Tag `v1.0.0` after codegen behavior is stable enough to avoid an immediate follow-up major

## Releases

Maintainers (default branch is **`master`**):

1. Update `CHANGELOG.md` and `workspace.package.version` in `Cargo.toml`.
2. Add `docs/release-vX.Y.Z.md`, merge to `master`, tag `vX.Y.Z`, and create a GitHub Release.
3. Publish to crates.io with `cargo publish --locked` (or `just publish`). Publish the macro crate before the runtime and keep its exact version requirement in
   `runtime/Cargo.toml` in sync with the workspace version. There is no automated publish workflow.

Contributors: add user-visible changes under `[Unreleased]` in `CHANGELOG.md`.
