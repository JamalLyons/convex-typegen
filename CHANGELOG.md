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
- Removed the use of `.unwrap()` in the codebase.

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
- Removed the use of `.unwrap()` in the codebase.

### Fixed
- Test generation scripts not deleting generated files.

## [0.3.0] - 2026-05-09
Version 0.3.0 is a major release that improves the stability and consistency across the library. Major bug fixes, and refactorings have been made to the codebase.

### Added

### Changed
- Dependency version updates for [convex](https://docs.rs/convex/latest/convex/), [oxc](https://oxc.rs), [serde](https://serde.rs), and [serde_json](https://serde.rs/json.html).

### Fixed
- Stable ordering of generated function argument types and `FUNCTION_PATH` blocks: function ASTs are keyed by canonicalized source paths and collected in a `BTreeMap`, so output no longer depends on hash iteration order (cleaner diffs and a clearer `cargo:rerun-if-changed` story).
- Function AST map keys are unique per file: canonical absolute paths prevent two different modules with the same basename (for example `convex/a/foo.ts` and `convex/b/foo.ts`) from colliding or replacing each other.