# What's guaranteed and what isn't

convex-typegen exists so Rust callers get the same compile-time contract TypeScript gets from Convex's generated `api` and `Doc` types. The types reflect whatever schema and validators you feed the generator at **build time**. They do not replace Convex's runtime rules.

## Guaranteed at compile time

When you use generated references and models with [`TypedClient`](../typed-client.md), the Rust compiler checks:

- **Function paths** — pass `tasks::list` to `query`, not `mutation`, and never a mistyped string path on the typed API.
- **Argument shapes** — each endpoint's args struct matches its Convex validators (required fields, optionals, nested objects, literal enums).
- **Return types** — declared `returns` validators become Rust result types (for example `Vec<String>` for `tasks::list`, `Id<tables::Tasks>` for `tasks::add`).
- **Tables and documents** — `tables::*` models, `Id<T>`, and document fields line up with `convex/schema.ts`.
- **Optionals vs null** — `v.optional` and nullable unions encode as distinct Rust types where the generator models them.
- **Application errors** — `with_error::<YourPayload>()` ties a declared error shape to decoding via [`CallError`](../errors.md).

Schema or function changes that alter generated types fail the Rust build until you regenerate bindings and update call sites—similar to TypeScript after `npx convex dev`.

## Escape hatches by design

Not every call has to go through generated types:

- **`client_mut()`** on [`TypedClient`](../typed-client.md#using-the-rest-of-the-convex-sdk) returns `&mut convex::ConvexClient` for auth, `watch_all`, and untyped `query` / `mutation` / `action` using `path()` plus raw Convex values.
- **Re-exported `convex` crate** — `convex_typegen_runtime::convex` is the SDK version the runtime targets; you can use it directly alongside generated code.

Choosing an escape hatch opts out of compile-time checking for that call.

## Deploy drift

Generated types describe the contract **when you last built**. If you deploy a changed backend without regenerating and rebuilding the Rust app, the compiler cannot catch the mismatch; Convex may reject calls at runtime (wrong args, missing fields, changed return shapes)—the same class of problem as running an old TypeScript client against a new deployment.

**Practice:** regenerate bindings and rebuild Rust whenever validators or the schema change, and treat binding diffs in review like TypeScript `_generated` updates.

## Runtime-only checks

Rust types cannot prove:

- **Whether an `Id` still exists** — deleting or never inserting a row is validated when the query or mutation runs.
- **Auth and permissions** — tokens, roles, and deployment rules are enforced on the server.
- **Convex validators beyond your declared contract** — internal functions, custom server checks, and data written outside your schema still execute at runtime.

For a runnable contrast between typed calls and SDK escape hatches, see [typed calls and subscriptions](../typed-client.md).
