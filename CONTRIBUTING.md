# Contributing to convex-typegen

Thank you for your interest in contributing.

## Development setup

1. Install Rust (stable) and nightly (for `rustfmt` — see `toolchain.toml`).
2. Clone the repo and run from the crate root:

```bash
just check          # clippy + tests
just fmt-check      # requires nightly rustfmt
```

Or manually:

```bash
cargo +nightly fmt --all --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo build --no-default-features
```

## Pull requests

- Keep changes focused; include tests for behavior changes.
- Run `just fmt` before pushing if you changed Rust sources.
- Update `CHANGELOG.md` under `[Unreleased]` for user-visible changes.
- MSRV is **1.78** (`rust-version` in `Cargo.toml`); CI enforces it.

## Golden tests

Integration snapshots live in `tests/golden_generate.rs`. After intentional codegen output changes:

```bash
cargo insta test --accept --all-features
```

## Publishing (maintainers)

1. Bump version in `Cargo.toml` and finalize `CHANGELOG.md`.
2. Tag `vX.Y.Z` and push; the [release workflow](.github/workflows/release.yml) publishes to crates.io when `CARGO_REGISTRY_TOKEN` is configured.

Manual fallback: `cargo publish --locked`.
