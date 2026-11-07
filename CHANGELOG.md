# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

## [1.0.0] - 2026-11-07

See the [migration guide](docs/guide/migration-v0.4.md) for the breaking changes.

### Added

- Lead the README, docs landing page, crate docs, v1 roadmap, and agent notes with end-to-end type
  safety messaging; add [what's guaranteed and what isn't](https://ctg.jamallyons.com/docs/type-safety.html).

- Re-export the Convex SDK from `convex-typegen-runtime`.

- Polish the launch README with a compiled typed query/subscription example, project badges,
  and documentation links. Add issue forms, a PR checklist, and contributor label conventions.

- Add an mdBook site with a verified first-project tutorial, task guides, a configuration reference,
  accessible light and dark themes, a docs.rs API reference, and GitHub Pages deployment.
- Add the local self-hosted Convex typed-client E2E gate (`just e2e`) and user guides for getting
  started, supported validators and source limits, v1 concepts, and migration from v0.4.
- Generate bindings in Cargo's `OUT_DIR` by default, with atomic writes and a non-writing CLI
  `--check` mode. Add source-aware diagnostics and migrate examples to `include!`.

- Add safe agent-worktree cleanup and stale Rust build-output cleanup with `just clean-worktrees` and a side-effect-free dry run.

- Add the runtime's `ConvexValue` derive and `include!()` helper. Generated models
  use the same derive as handwritten models while preserving their wire behavior.
  Regenerate bindings against the matching runtime. See [macros](https://ctg.jamallyons.com/docs/macros.html).

- **Breaking generated output:** Emit nested, typed public function references such as
  `api::admin::users::list`. Declared args and returns now live beside the reference
  (e.g. `api::admin::users::ListArgs`). References and model names omit a redundant leading
  module name, and `path()` replaces `FUNCTION_PATH`.
  Undeclared validators use explicit `RawArgs` and `RawValue` fallbacks. See
  [generated models](https://ctg.jamallyons.com/docs/generated-models.html) for current naming.

- Add a Criterion harness in the unpublished `convex-typegen-bench` workspace crate. `just bench`
  measures generated-project discovery/parsing, IR lowering, codegen and end-to-end generation;
  the benchmark-only `bench-internals` feature exposes those production stages to the harness.

- Generate Serde args, returns, typed table IDs, and full documents, including union tables.
  Validated literals and unions preserve the Convex wire kinds. See
  [generated models](https://ctg.jamallyons.com/docs/generated-models.html).

- Document pipeline CPU and heap hot spots and add a replayable profiling runner for
  synthetic projects. See [performance profiling](docs/performance.md#pipeline-profiling-findings-64).

- Generate seed-replayable synthetic Convex projects with `just gen-project`. See
  [performance tooling](docs/performance.md).

- Read a committed Convex function-spec JSON as the authoritative function inventory with
  `Configuration::with_function_spec_path`. Schema parsing still uses TypeScript.

- Resolve function builders through recognized Convex server imports and aliases. Resolve static
  local/imported validator constants and object spreads with bounded cycle diagnostics, preserve
  root-relative module identities, and skip reserved endpoint modules.

- Parse Convex 1.49.0 validator factories, including numeric aliases, nullable values, commit
  timestamps and opaque function handles. Accept indexed object and union tables and earlier
  same-file validator consts. The IR preserves quoted wire keys and exact literal kinds.

- Direct Serde conversion to and from `convex::Value` via `value_serde::{to_value, from_value}`. It
  preserves int64/float64 wire kinds, non-finite floats, bytes, nested arrays and records, and
  reports conversion failures with a value path. `value_serde::Bytes` uses Serde's byte-string
  contract.

### Changed

- Derive `PartialEq` on generated models and `Eq` where their payloads support it.
  Generated string-literal enums also derive `Copy` and `Hash`.

- Mark consuming configuration builders, error context, and function error-type
  adapters as `#[must_use]`.

- **Breaking error API.** Every variant of `ConvexTypeGeneratorError` and `CallError`
  is non-exhaustive. `CallError::Transport` wraps a runtime-owned `TransportError`
  that preserves the original message and error source chain.

- Ship a smaller generator crate archive and add package metadata and READMEs for
  the runtime and derive crates.

- Refine the documentation site’s tutorial, generated-models and support guides,
  and links to the Convex SDK. Document the v1 semver policy and public Rust API,
  including use of the SDK alongside the typed client.

- Update the derive macro's parser dependency to syn 3.

- **Breaking generator API.** Construct non-exhaustive `Configuration` with setters and use
  `build::generate(config)` for Cargo input tracking. The default schema follows `convex_dir`.
  Match the crate-root `ConvexTypeGeneratorError` with a fallback arm. Missing schemas report
  the attempted path and I/O errors use the `Io` variant with a path.

- **Breaking runtime API.** Match extensible call errors with a fallback arm and `..` for
  extensible fields. Use `FunctionRef` constructors for handwritten endpoints.

- **Breaking generated output.** Distinct string-literal unions now use semantic unit variants
  and preserve their exact wire strings. Punctuation-only and empty literals use a separate
  deterministic naming fallback. See the [migration guide](docs/guide/migration-v0.4.md).

- **Breaking generated output.** Unions of distinct string literals plus null generate
  `Option<Enum>`. Optional fields use `Option<Option<Enum>>` to preserve omitted keys
  and explicit null. Top-level nullable enum returns expose variants on `*ReturnValue`.
  See the [migration guide](docs/guide/migration-v0.4.md).

- **Breaking generator MSRV:** Raise the generator and Oxc-dependent build tooling from Rust
  1.95 to **1.97**, required by Oxc 0.157. Runtime and derive MSRV remain **1.85**.

- **Breaking default generated filename:** Rename `convex_types.rs` to `convex_api.rs`; the file contains generated API references and the typed client surface, and is written to `OUT_DIR` by default. There is no compatibility alias or fallback.

- **Breaking runtime API:** Replace `ConvexClientExt::prepare_args` with
  [`TypedClient`](https://ctg.jamallyons.com/typed-client.html) for typed queries, mutations, actions,
  and query subscriptions. Function references can declare typed application
  errors, and malformed payloads retain their original Convex value.

- **Breaking generated output:** Regenerate bindings for canonical path-scoped names, snake_case
  fields, typed IDs and union values. Optional nullable fields use an outer `Option` for presence
  and a union member for null. Unknown shapes and `v.any()` use `model::RawValue`.

- Build the generator, runtime, examples, and tooling in a single Cargo workspace.

- **Breaking:** Use `convex-typegen` (Rust 1.97) as a build dependency and `convex-typegen-runtime` (Rust 1.85) as a normal dependency. The runtime now owns the typed client, value codecs, and generated-code Serde imports. The generator’s `client` feature and runtime re-exports were removed. Regenerate existing bindings with the matching release.

- **Breaking:** Generated `TryFrom<Args> for BTreeMap<String, serde_json::Value>` implementations were removed.
- **Breaking:** Removed the lossy `IntoConvexValue` and `ConvexValueExt` APIs and the `ConvexJsonError` prelude alias. Generated `v.bytes()` fields now use `ConvexBytes` so they serialize as Convex byte strings.

- Route offline generation through a single typed model for schemas, validators, and functions.
  Preserve source locations, canonical function identity, and known return metadata.

- **Breaking:** Malformed validator expressions that previously produced incomplete types now
  fail with a file, byte span and validator name. This includes unsupported inline return validators.

### Fixed

- Fix `cargo deny` license check for the readme example crate (`license.workspace = true`).

- Docs: The [0.3.0] supply-chain bullet incorrectly claimed a GitHub Actions release workflow for tagged publishes. Git history shows only `.github/workflows/rust.yml` (CI quality checks) was ever added; `release.yml` was referenced in early `CONTRIBUTING.md` text but never committed. Crates.io publishing has always been manual (`cargo publish` / `just publish`), as documented in `CONTRIBUTING.md` and [semver-policy.md](docs/semver-policy.md).

## [0.4.0] - 2026-08-15

Named `v.object` structs in generated code, plus semver/API-stability docs. Library API is unchanged; **generated output is breaking** (allowed in a 0.x minor — see [semver-policy.md](docs/semver-policy.md)).

### Added

- [docs/semver-policy.md](docs/semver-policy.md) and [docs/api-stability.md](docs/api-stability.md): library API vs generated-output semver, MSRV rules, and 1.0 readiness checklist.
- [docs/oxc-ast-evaluation.md](docs/oxc-ast-evaluation.md): evaluation of direct Oxc AST traversal vs ESTree JSON round-trip (hybrid path recommended).
- Internal `ConvexValidator` IR (`src/convex/validator.rs`) and `structName` hints on parsed `v.object` trees.
- Named nested `v.object` structs in generated code (schema columns, function args, nested objects, array element objects) with structural dedupe and collision errors.
- Oxc AST spike module (`src/convex/ast.rs`) for future migration benchmarking.

### Changed

- **Breaking (codegen):** `v.object({ ... })` validators emit dedicated `pub struct` types (e.g. `ProjectsSettings`, `IntegrationsMirrorFlags`, `TasksSearchFilter`) instead of `ConvexJsonValue` or a homogeneous `BTreeMap` where a stable name applies. Pin the same `convex-typegen` version in `[dependencies]` and `[build-dependencies]`.



## [0.3.0] - 2026-05-19

Version 0.3.0 improves stability, supply-chain hygiene, and generated-code correctness for multi-module Convex backends.

### Added

- Optional `verbose` crate feature: when enabled, Oxc parser/semantic diagnostics are printed to stderr with `Debug` formatting (in addition to messages embedded in `ParsingFailed::details`).
- `prelude` module re-exporting commonly used types and traits.
- README for the basic example directory.
- `docs/architecture.md` for contributors.
- **Schema parsing —** `defineTable` **builder chains:** `parse_schema_ast` peels `defineTable({ ... })` followed by `.index`, `.searchIndex`, or `.vectorIndex` before reading column validators.
- `examples/advanced`**:** richer Convex sample with indexes and `withIndex`.
- **Tests:** integration golden tests (`tests/golden_generate.rs`), build-script smoke test, `generate` coverage for chained `.index()` and cross-module duplicate export names.
- `client` **Cargo feature** (default-on): runtime helpers (`ConvexClientExt`, `IntoConvexValue`, `ConvexValueExt`) depend on the official `convex` crate; disable with `default-features = false` for build-only use.
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
