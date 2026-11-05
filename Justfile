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
e2e:
    scripts/e2e.sh

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

# Check and test the runtime on its minimum supported Rust version.
runtime-msrv:
    cargo +1.85 check --locked -p convex-typegen-runtime
    cargo +1.85 test --locked -p convex-typegen-runtime

install-example-deps:
    cd examples/basic && bun install
    cd examples/advanced && pnpm install

# Build and run the `basic` example (Convex URL / keys typically come from `.env` via dotenvy).
example *ARGS:
    cargo run -p basic {{ ARGS }}

# Richer Convex surface + Rust smoke test (`examples/advanced/readme.md`).
example-advanced *ARGS:
    cargo run -p advanced {{ ARGS }}

# Generate and open the documentation for this package.
doc:
    cargo doc -p convex-typegen --all-features --no-deps --open

publish:
    cargo publish

publish-test:
    cargo publish --dry-run --allow-dirty

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
