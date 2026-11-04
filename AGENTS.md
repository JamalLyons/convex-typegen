# Agent notes

Read these before changing the crate:

- [docs/v1-roadmap.md](docs/v1-roadmap.md) — v1 vision, known gaps, open decisions
- [docs/architecture.md](docs/architecture.md) — pipeline and where to edit
- [docs/semver-policy.md](docs/semver-policy.md) — library API vs generated output

## Build, test, lint, format

Commands below match the `Justfile` and `.github/workflows/rust.yml`. Run them from the crate root.

```bash
just check       # clippy (warnings denied) + cargo test
just lint        # cargo clippy --all-targets --all-features -- -D warnings
just test        # cargo test
just build       # cargo build --all-features --release
just fmt         # nightly rustfmt, including the runtime and both examples
just fmt-check   # same, check only
```

CI equivalents:

```bash
cargo +nightly fmt --all --check
cargo +nightly fmt --manifest-path runtime/Cargo.toml --check
cargo +nightly fmt --manifest-path examples/basic/Cargo.toml --check
cargo +nightly fmt --manifest-path examples/advanced/Cargo.toml --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --all-features
cargo build --locked --no-default-features
cargo test --locked --no-default-features
cargo +1.85.0 build --locked --manifest-path runtime/Cargo.toml
cargo +1.85.0 test --locked --manifest-path runtime/Cargo.toml
```

The generator MSRV is **1.95**. The `msrv` job installs Rust 1.95.0 and runs clippy, tests, and a `no-default-features` build. The `runtime-msrv` job builds and tests `runtime/Cargo.toml` on Rust **1.85**. `rustfmt.toml` uses unstable options, `toolchain.toml` pins nightly, and both `just fmt` and the Rustfmt job run `cargo +nightly fmt`. Stable rustfmt will reject the config.

After an intentional codegen change, refresh snapshots with `cargo insta test --accept --all-features` (`tests/golden_generate.rs`).

## Testing against Convex

Run every validation that needs a backend against a **self-hosted Convex backend on the local machine**. Use the open-source [convex-backend](https://github.com/get-convex/convex-backend), usually its Docker image, and point the CLI at it with `CONVEX_SELF_HOSTED_URL` and `CONVEX_SELF_HOSTED_ADMIN_KEY`. Docs: [self-hosting](https://docs.convex.dev/self-hosting), [testing the open-source backend](https://docs.convex.dev/testing/convex-backend).

Never use a Convex cloud account or anything paid. Do not create, log into, or deploy to Convex cloud.

`build.rs` must never hit the network. This crate’s generator and any consumer `build.rs` you add stay offline. A function spec or fixture is a committed file, or the output of an explicit CLI or Just step.

## Changes

- One issue per PR. Keep the diff scoped to that issue.
- Put user-visible changes under `[Unreleased]` in `CHANGELOG.md`. Update `docs/` when behavior or the public contract changes.
- Flag breaking changes per [docs/semver-policy.md](docs/semver-policy.md). Library API and generated names are different surfaces. On 0.x, a breaking codegen change may ship in a minor; from 1.0, generated renames and an MSRV bump are majors.
- Do not publish to crates.io (`cargo publish`, `just publish`). Do not merge pull requests.
