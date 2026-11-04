# Generated models

The generator emits Serde models from schema and function validators. Use them with
`convex_typegen_runtime::value_serde::{to_value, from_value}` to preserve Convex wire kinds.
Existing consumers should regenerate their bindings and update Rust names and field types.

## Function references and model names

Public queries, mutations, and actions generate constants under `api`. For example,
`convex/admin/users.ts` exporting `list` emits `api::admin::users::list`, whose
`path()` returns `"admin/users:list"`. Each reference implements
`convex_typegen_runtime::function::FunctionReference` with associated `Args`,
`Returns`, and `Kind` types. The kind markers are `Query`, `Mutation`, and `Action`.

Module segments and exports use snake_case, escape Rust keywords where possible,
and normalize punctuation and leading digits. The generator reports both origins
when distinct names collide in the same Rust namespace. Internal functions, HTTP
actions, crons, and explicitly configured functions outside the Convex root are
omitted from the generated API.

Regenerate your bindings and move imports from flat types such as
`AdminUsersListArgs` and `AdminUsersListReturn` to
`api::admin::users::{ListArgs, ListReturn}`. Replace
`AdminUsersListArgs::FUNCTION_PATH` with `api::admin::users::list.path()` when using
the official SDK directly. Nested models extend the enclosing model's name with
their field or container role. Struct fields use snake_case and Serde preserves
the original wire names, including camelCase and quoted keys.

`model::Id<tables::Tasks>` is a string on the wire with a distinct Rust type for each
table. The backend checks the underlying ID string and table against the deployment.
`model::Doc<tables::Tasks>` is the associated generated document model. Each union
document branch has its own `_id` and `_creation_time` fields. The latter uses the
`_creationTime` wire name and the float64 wire kind.

## Presence, unions, and literals

- `v.optional(T)` uses `Option<T>`. `None` omits the key. A present key must decode as
  `T`, including inside arrays of objects or records of objects.
- `v.nullable(T)` emits a union of `T` and the unit value `()` for Convex null.
  With `v.optional(v.nullable(T))`, `None` is absent and the union's null branch
  inside `Some` is present null.
- Union enums carry each branch's model. When every branch has a common required
  field with a distinct literal, the decoder selects the branch by that field.
  Other unions decode as the first matching branch in declaration order. Some branches
  share the same wire representation, such as IDs from different tables. The runtime
  codec has no deployment table mapping, so the backend validates the ID's table.
- Literal models are unit structs that serialize to one exact wire value and reject
  other values during decoding. A number literal uses float64 and a bigint literal
  uses int64. Records retain their string-shaped key validators and typed values.
  Byte buffers use the runtime's `Bytes` wrapper.

## Raw values

`convex_typegen_runtime::function::RawArgs` is a `BTreeMap<String, RawValue>` and
is the associated `Args` type when the endpoint has no declared args validator.
Use `RawArgs::new()` to send an empty argument object. A declared empty args
validator still produces an empty endpoint-specific args struct.

The associated `Returns` type for an undeclared return validator is
`convex_typegen_runtime::model::RawValue`. This wrapper also represents `v.any()`
values and retains bytes, numeric wire kinds, and nested values through the
runtime codec. A declared `v.null()` return uses `()`. Unsupported TypeScript
expressions can still produce adapter diagnostics. A committed function-spec can
supply resolved validators for functions with dynamic composition.
