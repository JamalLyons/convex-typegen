# advanced — convex-typegen stress-test example

This example is a **richer Convex backend** than `examples/basic/`: multiple tables, `v.id` links, nested `v.object` args (emitted as named Rust structs such as `ProjectsSettings` and `TasksSearchFilter`), optional parameters, `action`, and several Convex modules so `convex-typegen` must discover many files and emit many `*Args` structs.

## What Convex covers here

| Area | Where |
|------|--------|
| Schema: scalars, `v.id`, `v.record`, `v.array`, nested `v.object`, optionals, literals, `v.any`, `v.bytes`, `v.int64` | `convex/schema.ts` |
| Queries / mutations with typed `args` | `convex/users.ts`, `teams.ts`, `projects.ts`, `tasks.ts` |
| Actions (no DB in handler; side-effect-free mirror) | `convex/integrations.ts` |
| Mutations / queries with **no** `args` (empty Rust args structs) | `convex/workspace.ts` |
| Multi-module API (`users:…`, `tasks:…`, …) | Several files under `convex/` |
| Database **indexes** + `withIndex` in queries | `convex/schema.ts` + `users`, `teams`, `projects`, `tasks` |

## Prerequisites

- Rust (`cargo`)
- pnpm (or another package manager) for the Convex CLI

## 1. Convex backend

From **`examples/advanced/`**:

```bash
pnpm install
pnpm exec convex dev --tail-logs
```

Create **`.env.local`** next to this example’s `Cargo.toml` with your deployment URL if needed:

```bash
CONVEX_URL=https://<your-deployment>.convex.cloud
```

## 2. Rust app

```bash
cargo build   # runs build.rs → regenerates src/convex_types.rs
cargo run     # seeds demo data (if empty), runs queries/mutations/action using generated args
```

From the repo root:

```bash
cargo run --manifest-path examples/advanced/Cargo.toml
```

The binary calls `workspaceSeedIfEmpty`, prints `workspaceSummary`, then exercises several generated `*Args` types via `ConvexClient::prepare_args` and the Convex client.
