# Label conventions

Use existing labels consistently on issues and PRs. This document is a convention,
not a script for creating or deleting repository labels.

Choose a primary kind:

- `bug`: incorrect behavior or regression.
- `enhancement`: new capability or improvement.
- `documentation`: documentation-only work.
- `dependencies`: dependency updates, including bot PRs.
- `question`: usage or clarification (direct users to the docs first).

Add relevant areas:

- `area:parser`: TypeScript and function-spec input handling.
- `area:codegen`: generated Rust shapes and names.
- `area:runtime`: values, codecs, models, and runtime macros.
- `area:client`: typed calls, subscriptions, and application errors.
- `area:dx`: build integration, CLI, and contributor workflow.
- `area:testing`: tests, fixtures, and CI.
- `area:docs`: guides, API documentation, and migration.
- `area:perf`: profiling, benchmarks, and performance.

Add `breaking-change` for a public API or generated-contract break, `v1` for launch
requirements, and `epic` only for tracking issues. Use `good first issue` for bounded
starter work and `help wanted` when outside contributions would help. `rust` and
`javascript` are optional language tags for dependency updates.

Use `duplicate`, `invalid`, or `wontfix` only when triaging a closed issue, with a
reason or link. Avoid adding new spellings for an existing kind or area.
