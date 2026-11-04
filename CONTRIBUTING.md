# Contributing to convex-typegen

Thank you for your interest in contributing.

The v1 target — a generated, fully typed function client — is [docs/v1-roadmap.md](docs/v1-roadmap.md). Coding agents should follow [AGENTS.md](AGENTS.md): read the roadmap, architecture, and semver policy first; test only against a local self-hosted Convex backend; keep `build.rs` offline; do not publish or merge.

## Development setup

1. Install current stable Rust and nightly for `rustfmt`. Install Rust 1.85 for the runtime MSRV check.
2. Clone the repo and run from the workspace root:

```bash
just check          # clippy + tests
just fmt-check      # requires nightly rustfmt
just runtime-msrv   # requires Rust 1.85
```

Or manually:

```bash
cargo +nightly fmt --all --check
cargo clippy --workspace --locked --all-targets --all-features -- -D warnings
cargo test --workspace --locked --all-features
cargo build --workspace --locked --all-features
cargo +1.85 check --locked -p convex-typegen-runtime
cargo +1.85 test --locked -p convex-typegen-runtime
```

## Pull requests

- Keep changes focused; include tests for behavior changes.
- Run `just fmt` before pushing if you changed Rust sources.
- Update `CHANGELOG.md` under `[Unreleased]` for user-visible changes.
- Follow [docs/semver-policy.md](docs/semver-policy.md) when choosing version bumps (library API vs generated output).
- The generator MSRV is **1.95** and the runtime MSRV is **1.85**. CI enforces both. Use **nightly** rustfmt (`just fmt` / `just fmt-check`) for unstable options in `rustfmt.toml`.

## Golden tests

Integration snapshots live in `tests/golden_generate.rs`. After intentional codegen output changes:

```bash
cargo insta test --accept --all-features
```

## Publishing (maintainers)

Default branch is **`master`**. Land work on `dev`, then open a PR into `master`.

**CI (`.github/workflows/rust.yml`)** runs rustfmt, clippy, tests, MSRV, `cargo audit`, and `cargo deny` on pushes to `master`/`dev` and on pull requests. It is not triggered by version tags and does **not** publish to crates.io. There is no separate release or publish workflow in this repository.

**Release steps** (see also [docs/semver-policy.md](docs/semver-policy.md#releases)):

1. Set `workspace.package.version` in `Cargo.toml` (update `Cargo.lock` with `cargo check` or your usual flow) and move `[Unreleased]` notes into a dated `CHANGELOG.md` heading.
2. Add GitHub Release notes as `docs/release-vX.Y.Z.md` (see existing `docs/release-v0.3.0.md` and `docs/release-v0.4.0.md`).
3. Merge to `master`, tag `vX.Y.Z` on that commit, and push the tag (tags are for GitHub Release metadata only—they do not trigger an automated crates.io publish).
4. Create the GitHub Release on GitHub, pasting or linking content from `docs/release-vX.Y.Z.md`.
5. Publish manually: `cargo publish --locked` or `just publish` (optional dry run: `just publish-test`). Requires a [crates.io](https://crates.io) API token in your local Cargo credentials.
