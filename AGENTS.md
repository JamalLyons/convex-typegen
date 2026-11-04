# Agent notes

## Task isolation

- For task work that changes files, use a dedicated Git worktree and branch by default. Do not make changes in a checkout another agent may be using; this keeps concurrent tasks from interrupting or overwriting each other.
- Base the worktree on the appropriate target branch, and keep the task's changes isolated there. Read-only work can use the existing checkout.
- Use a shared checkout only when the user explicitly asks for it or the task requires editing that checkout.

Read these before changing the crate:

- [docs/v1-roadmap.md](docs/v1-roadmap.md) — v1 vision, known gaps, open decisions
- [docs/architecture.md](docs/architecture.md) — pipeline and where to edit
- [docs/semver-policy.md](docs/semver-policy.md) — library API vs generated output

## Build, test, lint, format

Run these from the workspace root.

```bash
just check          # workspace clippy (warnings denied) + tests
just lint           # workspace clippy on all targets and features
just test           # workspace tests with all features
just build          # workspace release build with all features
just runtime-msrv   # check and test only the runtime on Rust 1.85
just fmt            # nightly rustfmt for all workspace members
just fmt-check      # same, check only
```

CI equivalents:

```bash
cargo +nightly fmt --all --check
cargo clippy --workspace --locked --all-targets --all-features -- -D warnings
cargo test --workspace --locked --all-features
cargo build --workspace --locked --all-features
cargo build --locked -p convex-typegen --no-default-features
cargo test --locked -p convex-typegen --no-default-features
cargo +1.85.0 check --locked -p convex-typegen-runtime
cargo +1.85.0 test --locked -p convex-typegen-runtime
```

Workspace CI uses stable Rust. The generator declares Rust **1.95**, which the `msrv` job checks.
The `runtime-msrv` job selects only `convex-typegen-runtime` and uses Rust **1.85** with the shared
lockfile. Nightly rustfmt is required for the unstable options in `rustfmt.toml`.

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
