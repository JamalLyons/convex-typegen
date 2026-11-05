# Support matrix

The generator reads `convex/schema.ts` and public function metadata either from statically
resolvable TypeScript or a committed `convex function-spec` JSON file. Errors for unsupported
static shapes include the input path and byte range where available.

## Rust versions

The generator requires Rust **1.97** because it uses Oxc 0.157. The runtime and derive
crate support Rust **1.85**. Running generation in `build.rs` therefore requires Rust 1.97;
consumers of pregenerated bindings can use the runtime on Rust 1.85.

## Convex validators

The offline TypeScript parser accepts these Convex 1.49.0 validators:

| Validator | Notes |
| --- | --- |
| `v.string()`, `v.number()`, `v.float64()`, `v.boolean()`, `v.null()` | `number` and `float64` map to Float64; null maps to `()` |
| `v.int64()`, `v.bigint()`, `v.commitTs()` | Map to signed `i64` / Int64 |
| `v.bytes()` | Maps to runtime `value_serde::Bytes` |
| `v.id("table")` | Maps to table-scoped `model::Id<tables::Table>` |
| `v.literal(value)` | String, boolean, finite number, or signed 64-bit bigint literal; use `v.null()` for null |
| `v.optional(validator)` and `v.nullable(validator)` | Optional object fields are omitted when absent; nullable adds a null union branch |
| `v.array(validator)`, `v.object({...})`, `v.record(key, value)` | Record keys must be strings, IDs, string literals, or unions of these |
| `v.union(a, b, ...)` | Requires at least one member |
| `v.any()` | Preserves an arbitrary Convex value through `model::RawValue`; in args it uses the raw argument fallback |
| `v.function(signature?)` | Function handles; signatures accept `fnType` query/mutation/action and static args/returns |

`number` is Convex's floating-point number validator. The schema parser accepts required object
tables and unions of required object table shapes, including `.index()`, `.searchIndex()`, and
`.vectorIndex()` table builder chains. Unsupported validator calls, dynamic computed keys, and
invalid record key validators fail with diagnostics instead of silently producing guessed types.

## Source resolution limits

- Function discovery examines `.ts` files under `convex/` by default. `schema.ts`, `_generated/`,
  `node_modules/`, `*.d.ts`, `http.ts`, `crons.ts`, and `convex.config.ts` are excluded.
- A non-empty `Configuration::function_paths` replaces directory discovery for function files.
  Files outside `convex_dir` do not have a root-relative module identity.
- Endpoint declarations must be exported `const` bindings that call a recognized named `query`,
  `mutation`, `action`, `internalQuery`, `internalMutation`, or `internalAction` import from
  `convex/server` or a generated `/_generated/server` module.
- Inline validators, earlier top-level same-file `const` initializers, and named imported
  validators from relative modules are resolved. Imported validators can refer to sibling constants
  in their source module. Static object spreads work when they resolve to object literals.
- Resolution is deliberately bounded: validator-reference and object-spread depth are limited to
  64. Dynamic expressions, runtime-built validators, unsupported exports/import forms, and cyclic
  references are not evaluated. TypeScript assertions and `satisfies` wrappers around validators
  are unwrapped.
- A committed function spec replaces TypeScript function discovery and parsing and supplies the
  authoritative complete function inventory. Its resolved validators may cover dynamic TypeScript
  composition. The schema still comes from TypeScript; function-spec does not contain table
  definitions.

## Public surface and raw fallback

Only public query, mutation, and action endpoints become references under `api`. Internal
functions, HTTP actions in `http.ts`, and crons in `crons.ts` are skipped. Root function files appear
under `api::<module>`; nested folders remain nested modules. See [concepts](concepts.md#path-scoped-names).

An endpoint without a declared args validator gets
`convex_typegen_runtime::function::RawArgs` (`BTreeMap<String, RawValue>`). A declared empty args
validator instead gets an endpoint-specific empty args struct. An endpoint with no declared return
validator gets `convex_typegen_runtime::model::RawValue`; `v.any()` also maps to `RawValue`. These
fallbacks preserve arbitrary Convex values, including bytes and Int64/Float64 distinctions.

Unknown return identifiers in source parsing remain undeclared/raw. Unsupported inline validators
produce diagnostics. A committed function spec is the option when the deployment can resolve those
validators for you. See [generated models](../generated-models.md#raw-values).
