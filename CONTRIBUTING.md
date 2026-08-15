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
- Follow [docs/semver-policy.md](docs/semver-policy.md) when choosing version bumps (library API vs generated output).
- MSRV is **1.95** (`rust-version` in `Cargo.toml`); CI enforces it. Use **nightly** rustfmt (`just fmt` / `just fmt-check`) for unstable options in `rustfmt.toml`.

## Golden tests

Integration snapshots live in `tests/golden_generate.rs`. After intentional codegen output changes:

```bash
cargo insta test --accept --all-features
```

## Publishing (maintainers)

Default branch is **`master`**. Land work on `dev`, then open a PR into `master`. Tagging does **not** publish to crates.io by itself (there is no release workflow).

1. Set `version` in `Cargo.toml` (Cargo.lock follows) and move `[Unreleased]` notes into a dated `CHANGELOG.md` heading.
2. Add GitHub Release notes as `docs/release-vX.Y.Z.md`.
3. Merge to `master`, tag `vX.Y.Z` on that commit, and push the tag.
4. Create the GitHub Release from `docs/release-vX.Y.Z.md`.
5. Publish: `cargo publish --locked` (or `just publish`). Requires a crates.io API token locally.
