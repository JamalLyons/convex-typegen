# v1 roadmap

Where convex-typegen is going. Crate version today: **0.4.0**. This is the lasting picture for contributors and coding agents. Open decisions stay open until they are settled in issues.

## Why the library exists

[Convex](https://www.convex.dev/) is a reactive database: queries, mutations, and actions are TypeScript running in the database, and subscribed clients receive new results when the data they read changes ([understanding Convex](https://docs.convex.dev/understanding/)). The product’s core pitch is end-to-end type safety. `npx convex dev` writes `convex/_generated/` so a TypeScript app calls `useQuery(api.tasks.get, args)` with inferred argument and return types, `Id<"tasks">`, `Doc<"tasks">`, and live subscriptions ([generated API](https://docs.convex.dev/generated-api/)).

Rust is a client language. Backend functions stay TypeScript. The official [`convex` crate](https://docs.rs/convex/latest/convex/struct.ConvexClient.html) is stringly typed: `client.query("tasks:get", BTreeMap::new())` takes a function-name string and a `BTreeMap<String, Value>`, and returns an untyped [`FunctionResult`](https://docs.rs/convex/latest/convex/enum.FunctionResult.html) ([Rust quickstart](https://docs.convex.dev/quickstart/rust.md)). `Value` has no `Id` type and no generated document layer.

convex-typegen already runs in `build.rs` and emits argument structs, table structs, and `FUNCTION_PATH` constants (default `src/convex_types.rs`). Callers still do this:

```rust
client.query(
    TasksSearchArgs::FUNCTION_PATH,
    ConvexClient::prepare_args(args)?,
)?; // FunctionResult
```

v1 closes that gap: the Rust side of the wire should be as typed as `api.*` is in TypeScript.

## v1 headline

**A generated, fully typed function client — the Rust equivalent of `api.*`.** This is decided.

It is an extension over the official client. Auth, the builder, and the rest of `convex::ConvexClient` stay available. `ConvexClient` methods take `&mut self` and the client is `Clone`; the wrapper has to respect that. Today’s `ConvexClientExt` in `runtime/src/client.rs` only prepares arguments.

The generated surface:

- Typed function references, nested to mirror Convex module paths (`api::admin::users::list` for `admin/users:list`).
- Generated argument **and** return types.
- `Id<T>` and `Doc<T>` with `_id` and `_creationTime`.
- Subscriptions as `Stream`s of typed results.
- Typed `ConvexError` payloads.

```rust
let tasks: Vec<Doc<Tasks>> = client
    .query(api::tasks::list, ListArgs { done: Some(false) })
    .await?;
let id: Id<Tasks> = client
    .mutation(api::tasks::create, CreateArgs { text: "hi".into() })
    .await?;
let mut live = client
    .subscribe(api::tasks::list, ListArgs::default())
    .await?;
// live: impl Stream<Item = Result<Vec<Doc<Tasks>>, ConvexFnError<...>>>
```

Sketch, meant to be improved. Names and method signatures are not frozen.

## Supporting themes

These are what make the headline trustworthy. A workable order: value layer and coverage, then the IR and spec source, then the typed client, with local-backend tests as a gate throughout.

- **Serde-correct value layer.** `Serializer` / `Deserializer` for `convex::Value`: exact int64 and float64, bytes, nested optionals omitted (Convex treats `v.optional` as absent, [validation](https://docs.convex.dev/functions/validation)), `Id<T>` newtypes, and serde on tables, unions, and literal enums. The runtime already converts directly against `convex::Value`.
- **Validator and function-discovery coverage.** The full `v.*` set ([values](https://docs.convex.dev/api/modules/values)), table-level unions, real module paths, public vs internal ([internal functions](https://docs.convex.dev/functions/internal-functions)), `returns`, and non-function files skipped with a warning.
- **Spec-driven IR.** One neutral IR, fed by a committed [`npx convex function-spec`](https://docs.convex.dev/cli/reference/function-spec) JSON (canonical `module:name`, `functionType`, visibility, resolved `args` and `returns`) with the Oxc parser as the offline fallback. `build.rs` stays offline; a CLI or Just target refreshes the spec. Function-spec does not include table schemas, so `schema.ts` parsing remains until an official introspection path exists. Validators the deployment did not declare come back as `any`.
- **Local-backend end-to-end testing.** CI boots the open-source backend ([self-hosting](https://docs.convex.dev/self-hosting), [testing](https://docs.convex.dev/testing/convex-backend)), deploys a fixture, regenerates, and runs the typed client (query, mutation, action, subscription). That is the v1 acceptance gate.
- **Build ergonomics and CLI.** Lighter build-dependencies, an `OUT_DIR` option, `--check` for CI drift, span-annotated diagnostics. See open decisions below.
- **Docs.** A supported-features matrix and an end-to-end tutorial that uses the generated client.

## Known gaps today

Checked against this tree (0.4.0). Items marked unverified have not been re-run on a live backend.

- [x] The offline parser normalizes Convex 1.49.0 validator factories and accepts object and union table documents, including indexed definitions and earlier same-file validator consts.
- [ ] The emitter uses the file basename even though the IR retains the canonical module path. `FUNCTION_PATH` is `"{basename}:{export}"`, so `convex/admin/users.ts` becomes `users:list`. `convex/users.ts` and `convex/admin/users.ts` both exporting `list` then fail codegen with `Duplicate generated args struct name`.
- [x] Function discovery accepts only recognized Convex server builder imports (including aliases), distinguishes public and internal visibility, ignores unrelated/member calls, and skips HTTP, cron, app-config, and generated modules. Relative imports and static spreads are resolved through a bounded offline source graph with file/span diagnostics. Schema and function adapters share the resolver.
- [x] The function IR records canonical root-relative module identity and declared args/returns metadata. The emitter still uses basename-based legacy names and paths; path-scoped generated naming remains a separate issue.
- [ ] Generated return types, typed client methods, and typed subscriptions are still pending.
- [ ] `{Table}Table` structs and union enums derive only `Debug, Clone`: no serde, no `_id`, no `_creationTime`. `v.id(...)` maps to `String` (`convex_type_to_rust_type`). Fields keep their camelCase names under `#![allow(non_snake_case)]`. Named `v.object` structs and `*Args` do derive serde. A union in function args has no table context and becomes `ConvexJsonValue` (see `TasksSearchFilter.minPriority` in `examples/advanced/src/convex_types.rs`).
- [ ] Optional args and named-object fields now emit `skip_serializing_if`; `Option<Option<T>>` encodes omitted vs explicit null. Table structs still do not derive Serde, and nested omission has not been rechecked on a live backend.
- [x] Direct Serde conversion against `convex::Value` is available under `value_serde::{to_value, from_value}` and used by `ConvexClientExt::prepare_args`. It preserves int64/float64 wire kinds, non-finite floats, signed zero, bytes (`ConvexBytes`), and recursive values, with path-aware errors for unsupported values and overflow. The lossy JSON bridge was removed.
- [x] `convex-typegen` is build-only and has no Convex client or TLS dependency. `convex-typegen-runtime` owns the codecs, Serde re-exports, and extension over the official client.
- [ ] Tests are unit tests plus `tests/golden_generate.rs` and `tests/build_script_smoke.rs`. `.github/workflows/rust.yml` does not start a Convex backend. The examples call whatever `CONVEX_URL` points at.
- [ ] Default `Configuration::out_file` is `src/convex_types.rs`, rewritten on `cargo build`.

## Open decisions

- **MSRV.** The generator requires Rust **1.95** and the runtime requires Rust **1.85**. CI enforces both. Oxc 0.129.0 sets a high floor. Raising MSRV is a major bump ([semver policy](semver-policy.md)).
- **`OUT_DIR` vs a committed file.** Keep writing `src/convex_types.rs`, or emit into `OUT_DIR` and `include!` it.
- **Naming scheme.** Module-nested refs that match Convex paths (`admin/users:list`) will rename today’s `{Module}{Export}Args` types. That is a breaking codegen change under [semver policy](semver-policy.md). On 0.x it may ship in a minor; from 1.0 it needs a major.

Also unresolved: whether table schemas can be read from an official API (function-spec does not include them), and which ideas to reuse from the [nickcomua fork](https://github.com/nickcomua/convex-typegen) and [rustex](https://github.com/LucaLeukert/rustex).

## After v1

- Reactive helpers on the typed subscription: a cached latest value, and a set of live queries over `watch_all`.
- Optional framework adapters (Leptos, Dioxus, Bevy, Tauri), after checking what those ecosystems already do.
- Watch mode next to `npx convex dev`. `--check` for CI drift belongs with v1 build ergonomics; continuous watch can wait.
