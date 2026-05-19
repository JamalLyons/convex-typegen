# Default: show available recipes.
default:
    @just --list

# Build the library crate (this package has no `main`; use `example` to execute something).
build *ARGS:
    cargo build --all-features --release {{ ARGS }}

# Run the workspace tests for `convex-typegen`.
test *ARGS:
    cargo test {{ ARGS }}

# Clippy on all targets and features (warnings denied).
lint:
    cargo clippy --all-targets --all-features -- -D warnings

# Apply Rustfmt using nightly so options in `rustfmt.toml` apply (`rustup toolchain install nightly`).
fmt:
    cargo +nightly fmt

# Fail if the tree is not rustfmt-clean (same as `fmt` but check-only).
fmt-check:
    cargo +nightly fmt --check

# Clippy + tests (formatting: use `fmt-check` or `fmt` when you want rustfmt enforced).
check: lint test

install-example-deps:
    cd examples/basic && bun install
    cd examples/advanced && pnpm install

# Build and run the `basic` example (Convex URL / keys typically come from `.env` via dotenvy).
example *ARGS:
    cargo run --manifest-path examples/basic/Cargo.toml {{ ARGS }}

# Richer Convex surface + Rust smoke test (`examples/advanced/readme.md`).
example-advanced *ARGS:
    cargo run --manifest-path examples/advanced/Cargo.toml {{ ARGS }}

# Generate and open the documentation for this package.
doc:
    cargo doc --all-features --no-deps --open

publish:
    cargo publish

publish-test:
    cargo publish --dry-run --allow-dirty
