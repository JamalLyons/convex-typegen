# Changelog

All notable changes to this project will be documented in this file.

[//]: # (## [Unreleased])

[//]: # (### Added)

[//]: # (- Description of new features or changes.)

[//]: # ()
[//]: # (### Changed)

[//]: # (- Description of changes to existing features.)

[//]: # ()
[//]: # (### Fixed)

[//]: # (- Description of bug fixes.)

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

## [0.3.0] - 2026-05-09
Version 0.3.0 is a major release that improves the stability and consistency across the library. Major bug fixes, and refactorings have been made to the codebase.

### Added
- Optional `verbose` crate feature: when enabled, Oxc parser/semantic diagnostics are printed to stderr with `Debug` formatting (in addition to messages embedded in `ParsingFailed::details`).
- Added `prelude` module that re-exports the most commonly used types and traits for convenience.
- README.md for basic example directory.

### Changed
- Dependency version updates for [convex](https://docs.rs/convex/latest/convex/), [oxc](https://oxc.rs), [serde](https://serde.rs), and [serde_json](https://serde.rs/json.html).
- **Breaking:** Generated `*Args` types implement `TryFrom<Self> for BTreeMap<String, serde_json::Value>` (with `type Error = serde_json::Error`) instead of infallible `From`, so `serde_json::to_value` failures are not hidden behind `unwrap()`. `ConvexClientExt::prepare_args` now returns `Result<BTreeMap<String, convex::Value>, serde_json::Error>`.
- **Breaking:** Renamed `JsonValue` to `ConvexJsonValue` and `JsonError` to `ConvexJsonError` to avoid confusion with the `serde_json` crate.

### Fixed
- `ParsingFailed::details` for parser panic and semantic-check failures now includes joined Oxc diagnostic messages (primary message text) instead of only a generic summary, so callers and build logs can see what went wrong without lossy `Debug` output.
- Unconditional `eprintln!` for parse/semantic diagnostics was removed from default builds (use the `verbose` feature when stderr echo is desired).
- Stable ordering of generated function argument types and `FUNCTION_PATH` blocks: function ASTs are keyed by canonicalized source paths and collected in a `BTreeMap`, so output no longer depends on hash iteration order (cleaner diffs and a clearer `cargo:rerun-if-changed` story).
- Function AST map keys are unique per file: canonical absolute paths prevent two different modules with the same basename (for example `convex/a/foo.ts` and `convex/b/foo.ts`) from colliding or replacing each other.
- Generated args-to-JSON conversion no longer uses `serde_json::to_value(...).unwrap()`, avoiding panics when a field cannot serialize to JSON.
- `v.object({ ... })` types: heterogeneous objects (fields that do not share one Rust value type) are no longer mis-typed as `BTreeMap<String, T>` from a single sampled field; they are emitted as `serde_json::Value` until dedicated structs exist.