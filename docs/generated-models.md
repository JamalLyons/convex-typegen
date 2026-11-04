# Generated models

The generator emits Serde models from schema and function validators. Use them with
`convex_typegen_runtime::value_serde::{to_value, from_value}` to preserve Convex wire kinds.
Existing consumers should regenerate their bindings and update Rust names and field types.

## Names and documents

The function model name combines the PascalCase forms of every canonical module path
segment and the export, followed by `Args` or `Return`. For example, `admin/users:list`
emits `AdminUsersListArgs` and `AdminUsersListReturn`. An explicitly configured file
outside the Convex root uses the `Unscoped` prefix and has no `FUNCTION_PATH` constant.

Nested types extend the enclosing model's name with their field or container role. Rust
field identifiers use snake_case, escape keywords and normalize punctuation. Serde
uses the original wire names, including camelCase and quoted keys. If distinct inputs
normalize to the same Rust identifier in a scope, generation reports both origins.

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

`convex_typegen_runtime::model::RawValue` wraps the official SDK's `convex::Value`
for `v.any()` and shapes absent from the IR. For example, construct an unknown args
model as `UsersListArgs(RawValue(convex::Value::Object(Default::default())))` to send
an empty object. An unknown return is a `RawValue` directly. The wrapper retains
bytes, numeric wire kinds, and nested values when passed through the runtime codec.
Unsupported TypeScript expressions can still produce adapter diagnostics. A committed
function-spec can supply resolved validators for functions with dynamic composition.
