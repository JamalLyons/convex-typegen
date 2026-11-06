# Support matrix

This page answers two practical questions: what convex-typegen can generate from your project today,
and what to do when static analysis is not enough.

## Rust versions

| Crate | MSRV | When you need it |
| --- | --- | --- |
| `convex-typegen` (generator) | **1.97** | Any project that runs generation in `build.rs` |
| `convex-typegen-runtime` | **1.85** | Every consumer; enough on its own when bindings are pre-generated |

CI checks each crate at its declared MSRV.

## What you can generate offline

The generator reads `convex/schema.ts` for tables and either:

1. **TypeScript under `convex/`** — exported public queries, mutations, and actions whose validators
   are visible to the offline resolver, or
2. **A committed function-spec JSON file** — the public function list and resolved validators from a
   deployment you captured with the [Convex CLI](https://docs.convex.dev/cli/overview).

Start with TypeScript. When a function builds validators at runtime, spreads from unknown sources, or
uses patterns the resolver skips, generation fails with a file path and span when possible. That is
your signal to generate and commit a function spec instead of guessing types.

### Supported Convex validators

These match Convex 1.49-style [validator types](https://docs.convex.dev/database/types):

| Validator | Generated shape (typical) |
| --- | --- |
| `v.string()`, `v.number()`, `v.float64()`, `v.boolean()`, `v.null()` | `String`, `f64`, `bool`, `()` for null |
| `v.int64()`, `v.bigint()`, `v.commitTs()` | `i64` / Int64 |
| `v.bytes()` | `value_serde::Bytes` |
| `v.id("table")` | `model::Id<tables::Table>` |
| `v.literal(value)` | Exact literal model; use `v.null()` for null |
| `v.optional(T)`, `v.nullable(T)` | See [generated models](../generated-models.md#optional-and-null-fields) |
| `v.array(T)`, `v.object({...})`, `v.record(key, value)` | Vec, struct, `BTreeMap` with string-like keys |
| `v.union(a, b, …)` | Enum or nested union models |
| `v.any()` | `model::RawValue` |
| `v.function(signature?)` | Function handle type |

The schema parser accepts required object tables and unions of required object shapes, including
`.index()`, `.searchIndex()`, and `.vectorIndex()` chains. Unsupported validator calls, dynamic
record keys, or invalid record key types fail loudly during generation.

## Raw values and undeclared validators

When an endpoint omits an args or return validator, codegen still emits a reference—you pass or
receive [`RawArgs` / `RawValue`](../generated-models.md#raw-values-when-validators-are-missing)
instead of a typed struct. Declared empty args `{}` still get a dedicated empty struct.

Committing a function spec is the supported way to get typed args and returns for functions whose
TypeScript cannot be analyzed fully offline.

## TypeScript the offline parser understands

By default the tool scans `.ts` files under `convex/`, skipping `schema.ts`, `_generated/`,
`node_modules/`, declaration files, `http.ts`, `crons.ts`, and `convex.config.ts`. You can replace
discovery with an explicit file list in `Configuration::function_paths`.

Supported patterns include:

- Exported `const` endpoints calling `query`, `mutation`, `action`, or their `internal*` variants
  imported from `convex/server` or `/_generated/server`.
- Inline validators, same-file `const` aliases, and named validators imported from relative modules
  (including sibling constants in the imported file).
- Static object spreads that resolve to object literals.
- TypeScript `as` / `satisfies` wrappers around validators—they are peeled off.

Not supported: validators built at runtime, cyclic imports, most dynamic spreads, and files outside
your configured Convex root unless you list them explicitly and they still lack a stable module path
for naming.

### Reference: resolver limits

To keep generation predictable, validator following and object-spread inlining stop at depth **64**.
Beyond that, or when an expression is not one of the supported forms above, use a committed function
spec captured from a backend that already resolved the validators.

A function spec replaces TypeScript function discovery entirely but does **not** replace
`schema.ts`—table definitions still come from your schema file.

## What appears under `api`

Only **public** queries, mutations, and actions become `api` references. Internal functions, HTTP
routes in `http.ts`, and crons in `crons.ts` are skipped. Folder layout mirrors Convex modules:
`convex/admin/users.ts` → `api::admin::users`. See
[concepts](concepts.md#path-scoped-names).
