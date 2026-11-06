# Generated models

Codegen turns your [Convex schema](https://docs.convex.dev/database/schemas) and function
[validators](https://docs.convex.dev/database/types) into Rust structs and enums. Each model derives
[`ConvexValue`](macros.md) and round-trips through
`convex_typegen_runtime::value_serde::{to_value, from_value}` so Int64, Float64, bytes, and nested
objects keep their wire kinds.

## Args, documents, and string enums

When `convex/tasks.ts` exports an `add` mutation with `{ text: v.string(), … }`, you get a typed
args struct next to the function reference:

```rust,ignore
client.mutation(
    tasks::add,
    tasks::AddArgs {
        text: "Ship it".into(),
        priority: None,
    },
);
```

A query that returns table rows yields document structs with `_id`, `_creation_time`, and your
fields. IDs use `model::Id<tables::Tasks>`; full rows use `model::Doc<tables::Tasks>`.

A union of string literals becomes a Rust enum with one variant per string. Validator
`v.union(v.literal("low"), v.literal("high"))` becomes `AddArgsPriority` with `Low` and `High`
variants that encode as `"low"` and `"high"` on the wire.

## Function references

Public queries, mutations, and actions become constants under `api`. The `list` export in
`convex/tasks.ts` is `tasks::list`; its `path()` returns `"tasks:list"`. Each reference implements
`FunctionReference` with associated `Args`, `Returns`, `Kind`, and application `Error` types. See
[typed calls](../typed-client.md) and
[application errors](https://docs.convex.dev/functions/error-handling/application-errors).

Module segments and exports use snake_case. Internal functions, HTTP actions, crons, and modules
outside your configured Convex root are omitted from `api`.

## Shorter Rust names, same Convex paths

Rust types drop a repeated prefix when it matches the enclosing module name. The action
`integrations:integrationsMirror` still uses that Convex path, but the generated args struct is
`api::integrations::MirrorArgs` and the reference is `api::integrations::mirror`. Struct fields
stay snake_case in Rust; Serde keeps the original wire names (`camelCase`, quoted keys, and
`_creationTime` on documents).

Nested validator shapes become nested Rust types named after their field or container role.

## IDs, documents, and unions

`model::Id<tables::Tasks>` is a string on the wire with a distinct Rust type per table. The
backend still validates the ID string against the table you used at compile time.

`model::Doc<tables::Tasks>` is the generated row type. Union tables emit a Rust enum with one
variant per branch; each branch carries its own `_id` and `_creation_time` fields.

## Optional and null fields

`v.optional(T)` maps to `Option<T>`. `None` omits the key on encode. When the key is present, the
value must decode as `T`.

For a nullable union of string literals (including `v.nullable` around literals), use a plain enum
for the strings and `Option<Enum>` on a required field:

- `Some(AddArgsPriority::High)` sends `"high"`.
- `None` on a **required** nullable field sends JSON `null`.

When the validator is both optional and nullable, the outer `Option` tracks whether the field was
sent:

- `None` — key omitted.
- `Some(None)` — key present with JSON `null`.
- `Some(Some(AddArgsPriority::High))` — key present with a string literal.

For other nullable validators, `v.nullable(T)` becomes a union of `T` and `()` for Convex null.
Optional fields use `None` for absence and `Some(...)` when the key is present, wrapping the inner
null branch when the value is explicitly null.

Other union enums carry each branch’s model. When every branch shares a required field with a
distinct literal tag, decoding picks the branch from that field. Otherwise branches are tried in
declaration order. Some branches share wire shapes (for example IDs from different tables); the
backend validates IDs.

Literal return types become unit structs or fixed numeric types. Records keep string-shaped keys.
Bytes use the runtime `Bytes` wrapper.

## Set, clear, or omit a priority

The [first-project tutorial](../getting-started.md) declares
`priority: v.optional(v.nullable(v.union(v.literal("low"), v.literal("high"))))`
on `tasks:add`. Use these values for the generated `AddArgs::priority` field:

| Intent | Rust value | Convex object |
| --- | --- | --- |
| Leave the field unspecified | `None` | No `priority` key |
| Send an explicit empty priority | `Some(None)` | `{ priority: null }` |
| Assign high priority | `Some(Some(tasks::AddArgsPriority::High))` | `{ priority: "high" }` |

The tutorial sends `None` for the rejected empty-text mutation and
`Some(Some(tasks::AddArgsPriority::High))` for the successful write. All three states round trip
through the runtime codec. For a patch endpoint, your handler can treat omission and null
differently; the generator preserves presence on the wire.

## Raw values when validators are missing

Some endpoints do not declare args or returns in the spec. You can still call them:

| Situation | Generated type | What to do |
| --- | --- | --- |
| No args validator | `function::RawArgs` (`BTreeMap<String, RawValue>`) | Build a map or use `RawArgs::new()` for `{}` |
| Empty args object validator | Empty endpoint-specific struct | Pass `MyArgs {}` |
| No return validator | `model::RawValue` | Decode manually or narrow with `from_value` |
| `v.any()` | `model::RawValue` | Same as above; wire kinds are preserved |

If TypeScript builds validators dynamically, static generation may not see the full shape. Capture
a [function spec](https://docs.convex.dev/cli/overview) from a deployed backend and commit it—see
[generation](../generation.md#use-a-committed-function-spec). The spec carries validators the
offline parser cannot resolve.

## Reference: literal and variant names

You rarely need these rules day to day. They explain odd variant names in diffs and snapshots.

String-literal enums use PascalCase variant names derived from ASCII letters and digits.
Punctuation splits words. Leading digits and the name `Self` get a `Type` prefix. Literals with no
ASCII letters or digits become `_Wire` plus lowercase UTF-8 hex bytes (for example `_Wire21` for
`"!"`).

When two variants would share a name, each gets a `Wire` suffix plus the original string’s
lowercase UTF-8 hex bytes. Extra trailing `X` characters avoid colliding with an existing base
name. Reordering literals in TypeScript does not rename variants.

If two different Convex exports would collide in the same Rust module, the generator reports both
origins so you can rename the TypeScript export or adjust modules.
