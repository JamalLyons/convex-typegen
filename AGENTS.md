# Agent notes

## Mission

convex-typegen gives Rust the end-to-end type safety TypeScript gets from Convex: schema and
function signatures become Rust types at build time, so wrong tables, fields, arguments, and
return types are compile errors. Escapes (`client_mut()`, the re-exported `convex` crate) and
runtime-only checks (auth, live validators, document existence) are documented on the
[type-safety page](https://ctg.jamallyons.com/docs/type-safety.html).

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
just generator-msrv # generator CI checks and pinned UI snapshots on Rust 1.97.0
just runtime-msrv   # test the runtime and a pregenerated consumer on Rust 1.85.0
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
bash scripts/generator-msrv.sh
cargo +1.85.0 check --locked -p convex-typegen-runtime
cargo +1.85.0 test --locked -p convex-typegen-runtime
```

Workspace CI uses stable Rust. The generator declares Rust **1.97**, which the `msrv` job checks.
The `runtime-msrv` job checks the runtime on Rust **1.85** and compiles an external consumer
from bindings pregenerated on Rust **1.97**. Nightly rustfmt is required for the unstable options in `rustfmt.toml`.

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

## Worktrees and disk space

- Run `cargo clean` in your worktree once your PR is open and you are done building.
- Preview cleanup with `just clean-worktrees-dry`; apply with `just clean-worktrees`.
  Pass `--stale-days N` (default 3) or `--force` through either recipe. Force permits
  discarding dirty merged worktrees; main/current and locked worktrees are always skipped.
  Lock a worktree with `git worktree lock <path>` while it must be retained.
- The script fetches/prunes origin and recognizes merged branches by ancestry to
  `origin/master`, a deleted origin upstream, or a merged PR found by optional,
  authenticated `gh`. Dry runs skip fetch/prune and use cached refs, so their
  decisions may differ from a subsequent live run. They report zero freed space.
- Unmerged worktrees are idle when no file or directory in the checkout (including
  `target/`) or its Git metadata has been modified in N days. Only their untracked
  `target/` directory is removed. Symlink targets and tracked target files are skipped.
  Timestamps are an activity heuristic; lock worktrees used by long-running processes.
- Dirty means tracked, untracked, or ignored changes outside root `target/`.
  Cleanup uses only this repository's `git worktree list`, including worktrees in
  arbitrary agent directories; no extra roots configuration is needed. It never
  scans neighboring directories. Disk freed is measured in allocated KiB using
  `du` before and after deletion, not estimated build sizes.
- Optional: install `sccache` and build with `RUSTC_WRAPPER=sccache` to reuse compiler
  outputs across worktrees. It is not required. Do not set a shared
  `CARGO_TARGET_DIR`: parallel agents would contend on Cargo's build lock and
  invalidate each other's outputs.
