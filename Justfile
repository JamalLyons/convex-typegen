# Default: show available recipes.
default:
    @just --list

# Build the entire workspace in release mode.
build *ARGS:
    cargo build --workspace --locked --all-features --release {{ ARGS }}

# Run workspace tests.
test *ARGS:
    cargo test --workspace --locked --all-features {{ ARGS }}

# Run the typed-client E2E against a local self-hosted Convex backend in Docker.
e2e SCENARIO="fixture":
    scripts/e2e.sh {{ quote(SCENARIO) }}

# Clippy on all targets and features (warnings denied).
lint:
    cargo clippy --workspace --locked --all-targets --all-features -- -D warnings

# Apply Rustfmt using nightly so options in `rustfmt.toml` apply (`rustup toolchain install nightly`).
fmt:
    cargo +nightly fmt --all

# Fail if the tree is not rustfmt-clean (same as `fmt` but check-only).
fmt-check:
    cargo +nightly fmt --all --check

# Clippy + tests (formatting: use `fmt-check` or `fmt` when you want rustfmt enforced).
check: lint-scripts lint test

# Run the generator MSRV CI checks, including pinned compiler diagnostic snapshots.
generator-msrv:
    bash scripts/generator-msrv.sh

# Check and test the runtime on its minimum supported Rust version.
runtime-msrv:
    cargo +1.85.0 build --locked -p convex-typegen-macros
    cargo +1.85.0 check --locked -p convex-typegen-runtime
    cargo +1.85.0 test --locked -p convex-typegen-runtime
    bash scripts/runtime-msrv-consumer.sh

install-example-deps:
    cd examples/basic && pnpm install --frozen-lockfile
    cd examples/advanced && pnpm install --frozen-lockfile

# Run the `basic` notes CLI, e.g. `just example list` (needs a local backend; see `examples/basic/readme.md`).
example *ARGS:
    cargo run -p basic {{ ARGS }}

# Run the `advanced` Axum bookmark server (needs a local backend; see `examples/advanced/readme.md`).
example-advanced *ARGS:
    cargo run -p advanced {{ ARGS }}

# Generate and open the documentation for this package.
doc:
    cargo doc -p convex-typegen --all-features --no-deps --open

# Build and serve the mdBook documentation site locally.
docs:
    mdbook build docs/site
    mdbook serve docs/site

publish:
    cargo publish --locked -p convex-typegen-macros
    just wait-for-crate convex-typegen-macros
    cargo publish --locked -p convex-typegen-runtime
    just wait-for-crate convex-typegen-runtime
    cargo publish --locked -p convex-typegen

publish-test:
    cargo publish --dry-run -p convex-typegen-macros
    cargo package --list -p convex-typegen-macros
    cargo package --list -p convex-typegen-runtime
    cargo package --list -p convex-typegen

[private]
wait-for-crate crate:
    #!/usr/bin/env bash
    set -euo pipefail
    crate={{ quote(crate) }}
    package_id="$(cargo pkgid -p "$crate")"
    version="${package_id##*[@#]}"
    for attempt in {1..12}; do
        if curl -fsS --max-time 10 "https://index.crates.io/co/nv/$crate" | \
            jq -e -s --arg version "$version" 'any(.[]; .vers == $version and .yanked == false)' >/dev/null; then
            exit 0
        fi
        sleep 5
    done
    echo "$crate $version is not available in the crates.io index" >&2
    exit 1

# Generate a synthetic Convex project. Pass a seed to replay a run.
gen-project TIER SEED="":
    cargo run --locked --release -p xtask -- {{ quote(TIER) }} {{ if SEED == "" { "" } else { "--seed " + quote(SEED) } }}

# Benchmark parse/discovery, IR lowering, codegen and end-to-end for generated projects.
bench *ARGS:
    cargo bench --locked -p convex-typegen-bench --bench pipeline -- {{ ARGS }}

# Remove merged worktrees and stale build output (never main/current).
clean-worktrees *ARGS:
    bash scripts/clean-worktrees.sh {{ ARGS }}

# Preview cleanup using cached refs, without fetching or pruning.
clean-worktrees-dry *ARGS:
    bash scripts/clean-worktrees.sh --dry-run {{ ARGS }}

# Check the cleanup script when ShellCheck is installed.
lint-scripts:
    @if command -v shellcheck >/dev/null 2>&1; then shellcheck scripts/clean-worktrees.sh; else echo "shellcheck not installed; skipping shell lint"; fi
