# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

## [0.3.0] - 2026-05-19

Version 0.3.0 improves stability, supply-chain hygiene, and generated-code correctness for multi-module Convex backends.

### Added

- Optional `verbose` crate feature: when enabled, Oxc parser/semantic diagnostics are printed to stderr with `Debug` formatting (in addition to messages embedded in `ParsingFailed::details`).
- `prelude` module re-exporting commonly used types and traits.
- README for the basic example directory.
- `docs/architecture.md` for contributors.
- **Schema parsing — `defineTable` builder chains:** `parse_schema_ast` peels `defineTable({ ... })` followed by `.index`, `.searchIndex`, or `.vectorIndex` before reading column validators.
- **`examples/advanced`:** richer Convex sample with indexes and `withIndex`.
- **Tests:** integration golden tests (`tests/golden_generate.rs`), build-script smoke test, `generate` coverage for chained `.index()` and cross-module duplicate export names.
- **`client` Cargo feature** (default-on): runtime helpers (`ConvexClientExt`, `IntoConvexValue`, `ConvexValueExt`) depend on the official `convex` crate; disable with `default-features = false` for build-only use.
- **Supply chain:** `deny.toml`, CI jobs for `cargo audit` and `cargo deny`, MSRV job (Rust 1.95), `no-default-features` build job, release workflow for tagged publishes.
- **Community:** `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md`, `SECURITY.md`.
- **Lexer:** maximum source file size (10 MiB) to mitigate accidental or malicious huge inputs.

### Changed

- Dependency updates for [convex](https://docs.rs/convex/latest/convex/), [oxc](https://oxc.rs), [serde](https://serde.rs), and [serde_json](https://serde.rs/json.html).
- **Breaking:** Generated `*Args` types implement `TryFrom<Self> for BTreeMap<String, serde_json::Value>` (with `type Error = serde_json::Error`) instead of infallible `From`. `ConvexClientExt::prepare_args` returns `Result<BTreeMap<String, convex::Value>, serde_json::Error>`.
- **Breaking:** Renamed `JsonValue` to `ConvexJsonValue` and `JsonError` to `ConvexJsonError` in the prelude.
- **Breaking:** Generated args structs are `{Module}{Export}Args` when the export name does not already start with the module segment (e.g. `GamesGetGameArgs`, `ModAListArgs`). Exports already prefixed with the module (e.g. `tasksSearch` in `tasks.ts`) become `TasksSearchArgs`. This prevents duplicate `pub struct` definitions when multiple modules export the same short name (e.g. `list`).
- Crate uses **edition 2024** with `rust-version = "1.95"` in `Cargo.toml`.

### Fixed

- `ParsingFailed::details` includes joined Oxc diagnostic messages.
- Unconditional `eprintln!` for parse/semantic diagnostics removed from default builds (use `verbose`).
- Stable ordering of generated function types via `BTreeMap` keys on canonical paths.
- Function AST map keys are unique per file (canonical absolute paths).
- Generated args-to-JSON conversion no longer uses `unwrap()` on `serde_json::to_value`.
- Heterogeneous `v.object` fields fall back to `ConvexJsonValue` instead of incorrect `BTreeMap<String, T>`.
- Function `args` parsing accepts ESTree `Property` and `ObjectProperty` nodes.
- Generated `TryFrom` omits keys for top-level `v.optional` parameters when `None` (not JSON `null`).
- Codegen rejects duplicate qualified args struct names with `InvalidSchema` instead of emitting uncompilable Rust.

## [0.2.0] - 2025-01-16

### Added

- Added this changelog file for all releases.
- Added `ConvexValueExt` trait to the [convex::Value](https://docs.rs/convex/0.9.0/convex/enum.Value.html) type.

### Changed

- Updated from convex version [0.8.1](https://docs.rs/convex/0.8.1/convex/index.html) to [0.9.0](https://docs.rs/convex/0.9.0/convex/index.html)
- Bumped [oxc](https://oxc.rs) to version 0.46.0
- Removed the use of `.unwrap()` in the typegen crate's own Rust sources (generated output still used `unwrap` until 0.3.0).

### Fixed

- Test generation scripts not deleting generated files.

## [0.1.1] - 2024-11-14

### Fixed

- Cleaned unnecessary documentation comments.
- Removed unused library's dependencies.

## [0.1.0] - 2024-11-13

### Added

- Initial release of the project.
