# v1 roadmap

Where convex-typegen is going. Crate version today: **1.0.0**. This is the lasting picture for contributors and coding agents. Open decisions stay open until they are settled in issues.

## Why the library exists

**Mission:** convex-typegen gives Rust the end-to-end type safety TypeScript gets from Convex.
Schema and function validators become Rust types at build time; wrong tables, fields, arguments,
or return types fail compilation, and schema changes break the Rust build until call sites match.

[Convex](https://www.convex.dev/) is a reactive database: queries, mutations, and actions are TypeScript running in the database, and subscribed clients receive new results when the data they read changes ([understanding Convex](https://docs.convex.dev/understanding/)). The product’s core pitch is end-to-end type safety. `npx convex dev` writes `convex/_generated/` so a TypeScript app calls `useQuery(api.tasks.get, args)` with inferred argument and return types, `Id<"tasks">`, `Doc<"tasks">`, and live subscriptions ([generated API](https://docs.convex.dev/generated-api/)).

Rust is a client language. Backend functions stay TypeScript. The official [`convex` crate](https://docs.rs/convex/latest/convex/struct.ConvexClient.html) is stringly typed: `client.query("tasks:list", BTreeMap::new())` takes a function-name string and a `BTreeMap<String, Value>`, and returns an untyped [`FunctionResult`](https://docs.rs/convex/latest/convex/enum.FunctionResult.html) ([Rust quickstart](https://docs.convex.dev/quickstart/rust.md)). `Value` has no `Id` type and no generated document layer.

convex-typegen runs in `build.rs` and emits argument structs, table structs, and typed
public function references—the Rust analogue of Convex's generated `api` and `Doc` types.
The [typed client](https://jamallyons.github.io/convex-typegen/typed-client.html) provides
calls and subscriptions over those references. See [what's guaranteed](https://jamallyons.github.io/convex-typegen/docs/type-safety.html) for compile-time limits.

## v1 headline

**End-to-end type safety for Convex in Rust — a generated, fully typed function client, the Rust equivalent of `api.*`.** This is decided.

The adapter borrows the official client mutably for SDK operations. Auth and the SDK builder remain available.

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
- **Spec-driven IR.** One neutral IR, fed by a committed [`npx convex function-spec`](https://docs.convex.dev/cli/reference/function-spec) JSON (canonical `module:name`, `functionType`, visibility, resolved `args` and `returns`) with the Oxc parser as the offline fallback. `build.rs` stays offline; `scripts/refresh-function-spec.sh` refreshes the spec only against the local self-hosted fixture backend. Function-spec does not include table schemas, so `schema.ts` parsing remains. Validators the deployment did not declare remain unknown in the IR.
- **Local-backend end-to-end testing.** Run `just e2e` locally to deploy the fixture to the open-source backend and exercise typed calls and subscriptions. CI compiles the E2E consumer with the committed function spec.
- **Build ergonomics and CLI.** Lighter build-dependencies, an `OUT_DIR` option, `--check` for CI drift, span-annotated diagnostics. See open decisions below.
- **Docs.** A supported-features matrix and an end-to-end tutorial that uses the generated client.

## Known gaps today

Checked against this tree (1.0.0). Items marked unverified have not been re-run on a live backend.

- [x] The offline parser normalizes Convex 1.49.0 validator factories and accepts object and union table documents, including indexed definitions and earlier same-file validator consts.
- [x] Function discovery accepts recognized Convex server builder imports and distinguishes public and internal visibility. Relative imports and static spreads are resolved through a bounded offline source graph with file/span diagnostics.
- [x] The function IR records canonical root-relative module identity and declared args/returns metadata. Generated model names and function paths use that identity.
- [x] Generated args and returns, typed table IDs and documents, literal and union models, and explicit field presence are available. See [generated models](https://jamallyons.github.io/convex-typegen/docs/generated-models.html).
- [x] Typed function references, client methods, query subscriptions, and application error decoding are available.
- [x] Direct Serde conversion against `convex::Value` is available under `value_serde::{to_value, from_value}` and used by `TypedClient`. It preserves int64/float64 wire kinds, non-finite floats, signed zero, bytes (`ConvexBytes`), and recursive values, with path-aware errors for unsupported values and overflow. The lossy JSON bridge was removed.
- [x] `convex-typegen` is build-only and has no Convex client or TLS dependency. `convex-typegen-runtime` owns the codecs, Serde re-exports, and extension over the official client.
- [ ] Tests are unit tests plus `tests/golden_generate.rs` and `tests/build_script_smoke.rs`. CI compiles the E2E consumer. Backend tests run locally with `just e2e`.
- [x] Default build output is `$OUT_DIR/convex_api.rs`, included with `convex_typegen_runtime::include!()`; generation is atomic and the CLI supports non-writing drift checks.

## Open decisions

- **MSRV.** The generator requires Rust **1.97** and the runtime requires Rust **1.85**. CI enforces both. Oxc 0.157 requires Rust 1.97. Raising MSRV is a major bump ([semver policy](semver-policy.md)).
- **Output.** `$OUT_DIR` + `convex_typegen_runtime::include!()` is the default; committed output is opt-in through `Configuration::out_file` or the CLI `--output` flag.
- **Naming scheme.** Module-nested refs that match Convex paths (`admin/users:list`) will rename today’s `{Module}{Export}Args` types. That is a breaking codegen change under [semver policy](semver-policy.md). On 0.x it may ship in a minor; from 1.0 it needs a major.

Also unresolved: whether table schemas can be read from an official API (function-spec does not include them), and which ideas to reuse from the [nickcomua fork](https://github.com/nickcomua/convex-typegen) and [rustex](https://github.com/LucaLeukert/rustex).

## After v1

- Reactive helpers on the typed subscription: a cached latest value, and a set of live queries over `watch_all`.
- Optional framework adapters (Leptos, Dioxus, Bevy, Tauri), after checking what those ecosystems already do.
- Watch mode next to `npx convex dev`. `--check` for CI drift belongs with v1 build ergonomics; continuous watch can wait.
