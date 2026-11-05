# Contributing

Start with [AGENTS.md](AGENTS.md), the [architecture](docs/architecture.md), and the
[semver policy](docs/semver-policy.md). Use a dedicated worktree and branch for each issue;
open focused PRs against `master` with tests for behavior changes and an `[Unreleased]`
changelog entry for user-visible changes.

## Local checks

Install `just`, stable Rust (generator MSRV **1.97**), nightly rustfmt, and Rust **1.85**
for runtime checks. From the workspace root:

```sh
just check          # workspace clippy + tests
just fmt            # nightly rustfmt
just generator-msrv # generator on Rust 1.97
just runtime-msrv   # runtime on Rust 1.85
mdbook build docs/site
```

Run `just e2e` for codegen, runtime, or client changes. It needs Docker, Node.js/npm, and
`jq`, and uses only a local self-hosted Convex backend; see the [E2E guide](docs/e2e.md).
Build scripts must stay offline. After intentional codegen changes, refresh snapshots with
`cargo insta test --accept --all-features` and review the diff.

## Review and cleanup

Describe the problem, resulting behavior, and validation in your PR. Follow the
[label conventions](.github/labels.md). Do not publish crates or merge PRs as part of
contribution work; version bumps, tags, and publishing are separate maintainer steps
under the [release policy](docs/semver-policy.md#releases).

Once your PR is open and builds are done, run `cargo clean` in its worktree. Preview
old-worktree cleanup with `just clean-worktrees-dry`; see
[the cleanup script](scripts/clean-worktrees.sh) and [AGENTS.md](AGENTS.md#worktrees-and-disk-space)
before applying it.

For usage questions, start with the [documentation](https://jamallyons.github.io/convex-typegen/).
Report vulnerabilities privately using [SECURITY.md](SECURITY.md), and follow the
[code of conduct](CODE_OF_CONDUCT.md).
