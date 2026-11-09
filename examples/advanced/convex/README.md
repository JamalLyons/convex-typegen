# Advanced example — Convex backend

This directory is a **Convex** app used to stress-test `convex-typegen`:

- Several **tables** with `v.id` references, records, nested objects, optional fields, `v.any`, `v.bytes`, and literal unions.
- Multiple **modules** (`users`, `teams`, `projects`, `tasks`, `integrations`, `workspace`) so file discovery and `module:function` paths are exercised.
- **`query`**, **`mutation`**, and **`action`** exports with a wide range of **`args`** shapes (including nested `v.object` and optional roots).
- **`defineTable({ ... }).index(...)`** chains in `schema.ts` and matching **`withIndex`** usage in handlers.

Run `pnpm install` then `pnpm exec convex dev` from `examples/advanced/` to push this schema to your dev deployment and refresh `_generated/` when Convex’s codegen format changes.
