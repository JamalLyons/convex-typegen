# How generation works

Convex functions run in the backend. A Rust consumer builds bindings for their validators
locally, then connects to a deployment at runtime.

1. **Describe the wire contract.** Write the table schema and declare each public function's
   `args` and `returns`. The generator reads the schema and either the function sources or a
   [committed function spec](../../generation.md#use-a-committed-function-spec).
2. **Generate at build time.** The `convex-typegen` build dependency writes Rust code.
   The default Cargo helper writes to `OUT_DIR` and registers inputs so Cargo knows when to rerun it.
3. **Include and use the types.** The `convex-typegen-runtime` application dependency provides
   `include!()`, the wire codec, and `TypedClient`. The client adapts the official Convex SDK.
   Add `convex` as a direct dependency when your code imports the SDK.

## Versions and Rust versions

Use the same release of the generator and runtime. Building the generator requires Rust **1.97**.
If you [pregenerate the bindings](../../generation.md#commit-bindings-and-check-for-drift) and omit
the build dependency, the runtime supports Rust **1.85**. The Rust API and generated names follow
the [semantic versioning policy](../semver-policy.md).

## Path-scoped names

A public `list` in `convex/admin/users.ts` becomes `api::admin::users::list`, with its arguments
and return type in the same module. The reference carries the wire path `admin/users:list`, so
there is no function-name string to keep in sync in your Rust calls. See
[generated model naming](../generated-models.md#function-references).

## Choose an input and an output independently

TypeScript function sources work well when validators can be [resolved statically](support.md#typescript-the-offline-parser-understands).
Choose a function spec when you need the deployment's resolved validators or want to commit a
snapshot of that interface. The schema always comes from TypeScript.

`OUT_DIR` works well when Cargo should regenerate bindings on each relevant change. Choose
checked-in output when you want code review to show binding changes or when your consumers build
without the generator. Neither output choice requires a deployment during generation.
