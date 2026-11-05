# v1 review

Historical review and probe results from the baseline below. Recorded Rust 1.95 commands are
not current setup instructions; see [current Rust requirements](guide/concepts.md#versions-and-rust-versions).

Baseline: [`cb5efa4`](https://github.com/JamalLyons/convex-typegen/tree/cb5efa46377f3ea9be35e7b4e4d1a0e6224a035a) on `master`, crate 0.4.0. The [roadmap](v1-roadmap.md) sets the goal: generate a fully typed Rust counterpart to Convex's `api.*`, over the official client. [v1 tracking issue](https://github.com/JamalLyons/convex-typegen/issues/53) · [milestone](https://github.com/JamalLyons/convex-typegen/milestone/1).

## Reproduced gaps

The offline probes use the public generator against isolated fixture variants so an early schema or import error does not mask later gaps. The live probes deploy the full fixture below.

1. **Validator names.** One schema field at a time with `v.float64()`, `v.bigint()`, `v.nullable(v.string())`, `v.function()` or `v.commitTs()` returned `InvalidType`. In SDK 1.49.0, number/float64 and int64/bigint are aliases (bigint is deprecated), and nullable expands to a union with null. Commit timestamps and function handles have dedicated semantics. The local fixture exercises the first three normalized shapes through the deployed spec and wire; the last two were parser probes only. [Allowlist](../src/convex/parser.rs#L35-L38) · [Convex validators](https://docs.convex.dev/api/modules/values).
2. **Table unions.** The fixture's `events: defineTable(v.union(v.object(...), v.object(...)))` deployed. The generator returned `InvalidSchema { context: "table_events", details: "Missing column definitions" }`. [Table parser](../src/convex/parser.rs#L135-L158) · [Convex schemas](https://docs.convex.dev/database/schemas).
3. **Module paths.** After isolating the nested file's inline functions, `admin/users.ts` emitted `"users:list"`. Adding root `users.ts` failed with duplicate `UsersListArgs`. The deployed spec distinguished the paths. [AST map](../src/convex/mod.rs) · [Basename extraction](../src/convex/parser.rs#L584-L592) · [Emission](../src/convex/codegen.rs#L70-L82).
4. **Call recognition.** `export const fake = helper({});` emitted `SyntheticFakeArgs`; `helper.method({})` failed with `Missing function type`. `query({args: {...shared}})` failed on `SpreadElement`. The exact false-positive condition includes a first call argument: the current loop emits nothing for an identifier call with zero arguments. [Discovery and args](../src/convex/parser.rs#L606-L703).
5. **Reserved files.** `rcfp` included root `http.ts`, `crons.ts`, `convex.config.ts` and the helper file. When passed only the three reserved fixture files, codegen emitted zero endpoints and no warning. Their shape happens to avoid the exported-call matcher. The spec includes the HTTP route separately and no cron entry. [File discovery](../src/fs.rs#L32-L87) · [HTTP actions](https://docs.convex.dev/functions/http-actions) · [Cron jobs](https://docs.convex.dev/scheduling/cron-jobs).
6. **Visibility.** The generator emitted args and path constants for all three internal builders in the fixture. The spec classified each as internal. Preserve this in IR and exclude these refs from the public Rust API. [Function model](../src/convex/types.rs#L66-L107) · [Parser](../src/convex/parser.rs#L624-L644) · [Internal functions](https://docs.convex.dev/functions/internal-functions).
7. **Returns.** `list` with a declared array return and `add` with a declared ID return generated only args and path constants. There is no typed query result or typed subscription on 0.4.0. [Argument extraction](../src/convex/parser.rs#L664-L722) · [Runtime extension](../src/convex/client.rs#L72-L88) · [Argument and return validation](https://docs.convex.dev/functions/validation).
8. **Shared validators.** `data: payload` with an import from `validators.ts` failed with `Invalid column type`, while the deployed spec inlined the full validator including fields added by spread. The direct Oxc AST spike is separate from production. [Generation pipeline](../src/lib.rs#L55-L78) · [Validator extraction](../src/convex/parser.rs#L221-L242) · [AST spike evaluation](oxc-ast-evaluation.md).
9. **Generated models.** An isolated schema emitted a table and union with `Debug, Clone` only, an ID as `String`, a verbatim camelCase field and no system fields. The record emitted `HashMap<String, i64>`; an args union emitted `ConvexJsonValue`. A standalone string literal generated the invalid field type `pub sample: "fixed",`. The local `docs` query returned both system fields and the mutation returned the matching document ID. [Named object serde](../src/convex/codegen.rs#L194-L214) · [Table and validator mapping](../src/convex/codegen.rs#L219-L407) · [Document IDs](https://docs.convex.dev/database/document-ids).
10. **Nested omission fails live.** Generated `UsersOptionalArgs { root: None, nested: UsersOptionalNested { note: None } }` passed through `prepare_args` as `{"nested": Object({"note": Null})}`. The official Rust client received `ArgumentValidationError` at `.nested.note`. CLI controls agreed: `{"nested":{}}` returned `false`, a present string returned `true`, and root or nested null failed. In contrast the echo function accepted explicit null for the required nullable field. [Nested field emitter](../src/convex/codegen.rs#L194-L214) · [Root omission](../src/convex/codegen.rs#L453-L476) · [Convex validation](https://docs.convex.dev/functions/validation).
11. **JSON is lossy, with a numeric nuance.** Direct conversion gave JSON `1` → `Int64(1)`, JSON `1.0` → `Float64(1.0)`, `[0,255]` → array of Int64, wire bytes → JSON array, and wire Infinity → JSON null. The generated `f64` arg `UsersNumberArgs { value: 1.0 }` preserved Float64 and succeeded live. The roadmap's integer-JSON case applies to integer-shaped input, such as raw JSON or a dynamic union. The live backend rejected encoded Int64(1) for `v.number()`. Echo round-tripped `i64::MAX`, Infinity, bytes, record values and a literal via Convex's encoded JSON. [JSON bridge](../src/convex/client.rs#L15-L68) · [Convex data types](https://docs.convex.dev/database/types).
12. **Default build dependency pulls in the client.** The root manifest defaults to `client`, and both examples declare the generator in regular and build dependencies. The feature tree includes the SDK and vendored TLS in the default graph; the isolated no-default-features graph excludes them. Run `cargo +1.95 tree --locked --target x86_64-unknown-linux-gnu -e features -i openssl-src` and `cargo +1.95 tree --locked --no-default-features -e normal`. [Features](../Cargo.toml#L26-L39) · [Basic manifest](../examples/basic/Cargo.toml) · [Advanced manifest](../examples/advanced/Cargo.toml).
13. **No E2E gate.** The workflow runs format, lint, unit/integration tests, example builds and dependency checks. The tests use generated strings and temporary source directories. The examples get `CONVEX_URL` from the environment. The throwaway SDK probe observed the initial query result and a subsequent result after mutation, and obtained a structured `FunctionResult::ConvexError` with object data from the action. [CI](../.github/workflows/rust.yml#L22-L110) · [Golden](../tests/golden_generate.rs) · [Smoke](../tests/build_script_smoke.rs) · [Example entry point](../examples/basic/src/main.rs#L18).
14. **Source output on build.** `Configuration::default().out_file` is `src/convex_api.rs`; both example build scripts call `generate` with that configuration and codegen calls `File::create` before checking the generated-name collisions. A failed generation can truncate a previously good artifact. [Defaults](../src/config.rs#L29-L39) · [Example build script](../examples/basic/build.rs#L6-L20) · [Write path](../src/convex/codegen.rs#L34-L82).

The open PRs at review time (#7, #18–20, #23, #25, #27–30) were dependency updates, including Oxc and CI checkout bumps. The findings above use the locked baseline. [Open pull requests](https://github.com/JamalLyons/convex-typegen/pulls).

## What function-spec can replace

The local 1.49.0 CLI capture has top-level `url` and `functions`. For example:

```json
{
  "identifier": "admin/users.js:list",
  "functionType": "Query",
  "visibility": {"kind": "public"},
  "args": {"type": "object", "value": {}},
  "returns": {"type": "array", "value": {"type": "string"}}
}
```

The wire path becomes `admin/users:list` after removing the terminal `.js` from the module portion. Each object field has `fieldType` and `optional`. The observed spec encodes int64 as `bigint`, float64 as `number`, nullable as a union, and records with `keys` and `values`. The omitted args on `users.js:list` became `{"type":"any"}` and omitted returns became JSON `null`, distinct from declared `{"type":"null"}`. HTTP entries have `functionType: "HttpAction"`, `method` and `path` rather than a callable identifier. No thrown error schema or tables were present. [CLI function-spec](https://docs.convex.dev/cli/reference/function-spec) · [Application errors](https://docs.convex.dev/functions/error-handling/application-errors).

Set `Configuration::function_spec_path` to use a committed spec as the complete authoritative function inventory. It replaces TypeScript function discovery, paths, visibility, and validator resolution; TypeScript remains the source for tables and the fallback when no spec is configured. Omitted args and returns remain unknown in the IR, while an empty object and declared `v.null()` remain distinct. HTTP actions are excluded from callable functions. Unsupported or malformed entries fail with the spec path and function/validator context; generation never falls back silently. Cargo consumes only the committed file.

The adapter preserves canonical nested module identity in the IR. The emitter uses it for
nested API references and scoped models, so both `users.js:list` and `admin/users.js:list` are covered
by the full capture snapshot. See [generated models](generated-models.md).

## Proposed v1 architecture

- **Sources and IR.** Parse the schema with Oxc. Parse function sources with a bounded, binding-aware resolver for supported builders, aliases, consts, relative imports and spreads, or load the committed spec for functions. Lower both inputs into a typed IR with separate function identity, kind and visibility, declared versus unknown validators, table document shape, field presence, and source provenance. Normalize aliases while retaining wire kinds and exact literal payloads. Reject unsupported expressions with source spans; a project can switch to a deployed spec for functions that use dynamic composition. Keep the ESTree adapter during the IR transition and change it independently where Oxc's typed nodes help.
- **Codegen.** Generate from IR without TS knowledge. Allocate names deterministically from scopes and exact wire names, escape Rust keywords, preserve wire keys with serde renames and diagnose normalization collisions. Use discriminant-based decoding for provably tagged unions, validated literal types and a documented matching policy for ambiguous unions. Emit a raw Value escape hatch for unknown shapes. Represent required nullable fields explicitly and optional nullable fields with three distinguishable states. Map matching full return document shapes to table document types only when the declared shape proves equivalence.
- **Runtime.** Encode/decode directly against official `convex::Value`, including exact f64, i64 and bytes. Generated fields supply omission metadata; the codec preserves explicit null. Add `Id<T>` with a typed marker and a string wire form, and `Doc<T>` with typed ID and creation time plus the generated fields. Ensure a union table includes the system fields for every variant. Preserve errors with their input/output path.
- **Typed calls.** Emit public `FunctionRef<Kind, Args, Output, Error = Value>` constants in `api`. A borrowing `TypedClient<'_>` over `&mut ConvexClient` provides `query`, `mutation`, `action` and query-only `subscribe`. A wrapper avoids conflict with the official client's inherent methods and preserves existing auth and builder APIs. The subscription owns its SDK handle, decodes each item and drops the underlying handle when released. Separate transport, server message, argument encode, result decode and application error cases. Allow an explicit error payload type per call/reference and preserve raw data if its decode fails.

Illustrative generated declarations, with runtime and module scaffolding elided:

```rust
pub mod tables {
    pub enum Tasks {}
    // Doc<Tasks> carries these generated fields and the two system fields.
    #[derive(Debug, Serialize, Deserialize)]
    pub struct TasksFields { pub text: String }
    impl Table for Tasks { type Fields = TasksFields; }
}

pub mod api {
    pub mod admin {
        pub mod users {
            #[derive(Debug, Default, Serialize, Deserialize)]
            pub struct DocsArgs {}
            #[derive(Debug, Serialize, Deserialize)]
            pub struct AddArgs { pub text: String }

            pub const docs: FunctionRef<Query, DocsArgs, Vec<Doc<Tasks>>> =
                FunctionRef::new("admin/users:docs");
            pub const add: FunctionRef<Mutation, AddArgs, Id<Tasks>> =
                FunctionRef::new("admin/users:add");
            pub const fail: FunctionRef<Action, (), ()> =
                FunctionRef::new("admin/users:fail");
        }
    }
}
```

Illustrative caller:

```rust
let mut typed = TypedClient::new(&mut client);
let id: Id<Tasks> = typed.mutation(api::admin::users::add, AddArgs {
    text: "local".into(),
}).await?;
let docs: Vec<Doc<Tasks>> = typed.query(api::admin::users::docs, DocsArgs {}).await?;
let live = typed.subscribe(api::admin::users::docs, DocsArgs {}).await?;
let failure = typed.action(api::admin::users::fail.with_error::<Denied>(), ()).await;
```

Unit args in this sketch encode as an empty argument map. The SDK signatures take `&mut self`; the wrapper's subscription should release its borrow after setup. [Official Rust SDK](https://docs.rs/convex/0.10.4/convex/struct.ConvexClient.html) · [Generated TypeScript API](https://docs.convex.dev/generated-api/).

## Decisions for Jamal

- **Crate boundary.** Approve a build-only generator plus a small runtime crate with explicit version pairing and a migration path for old reexports. A single crate with feature gating gives an escape hatch today, but keeps the common default build-dependency trap and ties runtime resolution to Oxc. Set crate name and transition policy in #39.
- **MSRV (historical proposal, superseded).** Jamal chose generator MSRV 1.97 for v1 because Oxc 0.157 requires it; the runtime remains on Rust 1.85. The original proposal was: keep generator MSRV 1.95 for v1. Pin or hold parser upgrades that would raise it until the release decision; CI verifies 1.95. A lower runtime floor may be useful after the split, but promise one only once a dedicated job proves it. Lowering the generator floor via Oxc downgrade would expand parser maintenance; raising it for a dependency update would spend a compatibility change without advancing the headline. Decide in #39.
- **Output location.** Make `OUT_DIR` plus `include!` the documented default build integration. Keep explicit output paths for CLI-generated, reviewable files and snapshots. Writing into `src` on every build makes checks and workspace cleanliness depend on incidental build activity. Decide legacy default behavior in #44.
- **Naming.** Adopt path-scoped refs and colocated types with deterministic Rust identifier spelling. Preserve exact wire spelling independently. Flat basename types cannot represent nested collisions; maintaining permanent aliases would carry that ambiguity into v1. Decide whether to offer temporary unambiguous aliases and how to handle normalization collisions in #40 and #41.
- **Transport.** Use the official SDK directly behind a thin adapter. Keep test seams private until another transport demonstrates a real need. A public transport trait would freeze cancellation, auth, stream and error semantics early. Set the error schema opt-in and wrapper shape in #42.
- **Schema introspection.** Keep `schema.ts` as the v1 table input. Convex documents an MCP `tables` tool returning declared and inferred schemas. SDK 1.49.0 implements it through a privileged system query plus a shapes endpoint; a local admin-authenticated SDK subscription to `_system/frontend/getSchemas` returned both the `events` union and the `tasks` object. The supported MCP tool is worth investigating as an explicit capture tool separate from the build. Coupling codegen to those deployment internals or inferring schemas from sampled documents would add a new compatibility contract. [Convex MCP tables tool](https://docs.convex.dev/ai/convex-mcp-server#table-tools).
- **Prior art.** Review the nickcomua fork's cross-file resolution and tagged-union cases as test inputs, and its generated API trait as a design comparison. Reuse Rustex's namespaced client ergonomics as a comparison for the call surface. Avoid wholesale adoption: resolve imports only within the offline supported subset, let both adapters feed the same IR, and keep the official SDK as the owner of transport and subscription behavior. [nickcomua fork](https://github.com/nickcomua/convex-typegen) · [rustex](https://github.com/LucaLeukert/rustex).

Land Value and neutral IR independently, then source/spec coverage, runtime packaging and generated models, then refs and client. Enable the [local-backend acceptance job](https://github.com/JamalLyons/convex-typegen/issues/43) before declaring v1 ready. The gate should fail on a known-bad generated path or nested null regression and prove query, mutation, successful action, error and subscription over the SDK. Drift checking and docs can proceed once their interfaces settle.

## Local reproduction

### Rust baseline

On macOS arm64 with Rust 1.95, the locked release build and Clippy with warnings denied passed. The all-features suite passed 91 tests and the no-default-features suite passed 84 tests, each with one ignored doctest. Both examples passed build and Clippy from a temporary baseline checkout. Nightly format checks passed at the root and for both examples. Commands:

```sh
cargo +1.95 build --locked --all-features --release
cargo +1.95 clippy --locked --all-targets --all-features -- -D warnings
cargo +1.95 test --locked --all-features
cargo +1.95 build --locked --no-default-features
cargo +1.95 test --locked --no-default-features
cargo +nightly fmt --all --check
for example in basic advanced; do
  manifest="examples/$example/Cargo.toml"
  cargo +nightly fmt --manifest-path "$manifest" --check
  cargo +1.95 clippy --locked --manifest-path "$manifest" --all-targets --all-features -- -D warnings
  cargo +1.95 build --locked --manifest-path "$manifest" --all-features
done
```

### Docker and CLI

Materialize the fixture from the appendix first. From the repository root, run the following in Bash. The Docker tag initially pulled was `latest`; this is the digest observed for that pull. The backend listens only on loopback. Keep the generated admin key in the shell environment.

```bash
docker run -d --name convex-v1-review \
  -p 127.0.0.1:13210:3210 -p 127.0.0.1:13211:3211 \
  -e CONVEX_CLOUD_ORIGIN=http://127.0.0.1:13210 \
  -e CONVEX_SITE_ORIGIN=http://127.0.0.1:13211 \
  -e DISABLE_BEACON=true \
  ghcr.io/convex/convex-backend@sha256:e9c084e8bd6c1f249012c672193f0988c748824f55a24cdad45f6185a8462f9a
export CONVEX_SELF_HOSTED_URL=http://127.0.0.1:13210
curl -fsS --retry 20 --retry-connrefused "$CONVEX_SELF_HOSTED_URL/version"
export CONVEX_SELF_HOSTED_ADMIN_KEY="$(docker exec convex-v1-review ./generate_admin_key.sh | tail -1)"
test -n "$CONVEX_SELF_HOSTED_ADMIN_KEY" || exit 1
cd tests/fixtures/v1-review
npm ci --no-audit --no-fund
npx --no-install convex dev --once --typecheck enable
jq --version
cd ../..
CONVEX_SELF_HOSTED_URL=http://127.0.0.1:13210 scripts/refresh-function-spec.sh
jq -r '.functions[] | [.functionType,.identifier,.visibility.kind] | @tsv' tests/fixtures/v1-review/function-spec.json
cd tests/fixtures/v1-review
```

The refresh script requires `CONVEX_SELF_HOSTED_URL=http://127.0.0.1:13210` and the admin key exported above. It runs the pinned fixture CLI, keeps only function structure, sorts entries and recursively sorts object keys, and replaces `tests/fixtures/v1-review/function-spec.json` only after a successful command and canonicalization. The commands below include deliberate failures. The Docker backend accepts extended JSON literals through `convex run`. [Self-hosting](https://docs.convex.dev/self-hosting) · [Local backend testing](https://docs.convex.dev/testing/convex-backend).

```bash
npx --no-install convex run users:optional '{"nested":{}}'
npx --no-install convex run users:optional '{"nested":{"note":null}}'
npx --no-install convex run users:optional '{"nested":{"note":"ok"}}'
npx --no-install convex run users:optional '{"root":null,"nested":{}}'
npx --no-install convex run users:number '{"value":1}'
npx --no-install convex run users:number '{"value":{"$integer":"AQAAAAAAAAA="}}'
npx --no-install convex run admin/users:echo '{"data":{"count":{"$integer":"/////////38="},"number":{"$float":"AAAAAAAA8H8="},"bytes":{"$bytes":"AP8="},"nested":{"nullable":null},"record":{"x":{"$integer":"AQAAAAAAAAA="}},"choice":"red"}}'
npx --no-install convex run admin/users:add '{"text":"local"}'
npx --no-install convex run admin/users:docs '{}'
npx --no-install convex run admin/users:ping '{}'
npx --no-install convex run admin/users:fail '{}'
curl -fsS http://127.0.0.1:13211/ping
```

After the SDK probe, clean up with `docker rm -f convex-v1-review` and `unset CONVEX_SELF_HOSTED_URL CONVEX_SELF_HOSTED_ADMIN_KEY`.

<details>
<summary>Fixture setup script</summary>

From the repository root, run this script to create the fixture under `tests/fixtures/`. Its source is included here to keep the review PR docs-only. Issue #43 promotes the fixture into the automated suite.

```bash
mkdir -p tests/fixtures/v1-review/convex/admin
cat > tests/fixtures/v1-review/.gitignore <<'FIXTURE'
node_modules/
.env.local
convex/_generated/
FIXTURE
cat > tests/fixtures/v1-review/package.json <<'FIXTURE'
{
  "name": "convex-typegen-v1-review",
  "private": true,
  "dependencies": {"convex": "1.49.0"},
  "devDependencies": {"typescript": "5.9.3"}
}
FIXTURE
cat > tests/fixtures/v1-review/convex/tsconfig.json <<'FIXTURE'
{
  "compilerOptions": {
    "allowJs": true,
    "strict": true,
    "moduleResolution": "Bundler",
    "jsx": "react-jsx",
    "skipLibCheck": true,
    "allowSyntheticDefaultImports": true,
    "target": "ESNext",
    "lib": ["ES2022", "dom"],
    "forceConsistentCasingInFileNames": true,
    "module": "ESNext",
    "isolatedModules": true,
    "noEmit": true
  },
  "include": ["./**/*"],
  "exclude": ["./_generated"]
}
FIXTURE
cat > tests/fixtures/v1-review/convex/schema.ts <<'FIXTURE'
import { defineSchema, defineTable } from "convex/server";
import { v } from "convex/values";

export default defineSchema({
  tasks: defineTable({ text: v.string() }),
  events: defineTable(v.union(
    v.object({ kind: v.literal("created"), id: v.id("tasks") }),
    v.object({ kind: v.literal("deleted"), reason: v.string() }),
  )),
});
FIXTURE
cat > tests/fixtures/v1-review/convex/validators.ts <<'FIXTURE'
import { v } from "convex/values";

export const common = { count: v.int64(), number: v.float64(), bytes: v.bytes() };
export const payload = v.object({
  ...common,
  nested: v.object({ note: v.optional(v.string()), nullable: v.nullable(v.string()) }),
  record: v.record(v.string(), v.int64()),
  choice: v.union(v.literal("red"), v.literal("blue")),
});
FIXTURE
cat > tests/fixtures/v1-review/convex/users.ts <<'FIXTURE'
import { query } from "./_generated/server";
import { v } from "convex/values";

export const list = query({ handler: () => ["root"] });
export const optional = query({
  args: { root: v.optional(v.string()), nested: v.object({ note: v.optional(v.string()) }) },
  returns: v.boolean(),
  handler: (_, { nested }) => Object.hasOwn(nested, "note"),
});
export const number = query({
  args: { value: v.number() }, returns: v.number(), handler: (_, { value }) => value,
});
FIXTURE
cat > tests/fixtures/v1-review/convex/admin/users.ts <<'FIXTURE'
import { ConvexError, v } from "convex/values";
import { internalQuery, internalMutation, internalAction, query, mutation, action } from "../_generated/server";
import { payload } from "../validators";

export const list = query({ args: {}, returns: v.array(v.string()), handler: () => ["nested"] });
export const echo = query({ args: { data: payload }, returns: payload, handler: (_, { data }) => data });
export const add = mutation({
  args: { text: v.string() },
  returns: v.id("tasks"),
  handler: (ctx, { text }) => ctx.db.insert("tasks", { text }),
});
export const docs = query({
  args: {},
  returns: v.array(v.object({ _id: v.id("tasks"), _creationTime: v.number(), text: v.string() })),
  handler: (ctx) => ctx.db.query("tasks").collect(),
});
export const fail = action({
  args: {}, returns: v.null(),
  handler: () => { throw new ConvexError({ code: "denied", retry: false }); },
});
export const hiddenQuery = internalQuery({ args: {}, returns: v.null(), handler: () => null });
export const hiddenMutation = internalMutation({ args: {}, returns: v.null(), handler: () => null });
export const hiddenAction = internalAction({ args: {}, returns: v.null(), handler: () => null });
export const ping = action({ args: {}, returns: v.literal("pong"), handler: () => "pong" as const });
FIXTURE
cat > tests/fixtures/v1-review/convex/http.ts <<'FIXTURE'
import { httpRouter } from "convex/server";
import { httpAction } from "./_generated/server";

const http = httpRouter();
http.route({ path: "/ping", method: "GET", handler: httpAction(async () => new Response("pong")) });
export default http;
FIXTURE
cat > tests/fixtures/v1-review/convex/crons.ts <<'FIXTURE'
import { cronJobs } from "convex/server";
import { internal } from "./_generated/api";

const crons = cronJobs();
crons.daily("fixture cleanup", { hourUTC: 0, minuteUTC: 0 }, internal.admin.users.hiddenMutation);
export default crons;
FIXTURE
cat > tests/fixtures/v1-review/convex/convex.config.ts <<'FIXTURE'
import { defineApp } from "convex/server";

export default defineApp();
FIXTURE
```

</details>

<details>
<summary>Offline generator and live Rust SDK probes</summary>

Run from the repository root with the fixture present. The probe builds a temporary consumer and keeps generated artifacts out of the checkout.

```bash
mkdir -p /tmp/convex-v1-probe/src/bin
cat >/tmp/convex-v1-probe/Cargo.toml <<EOF_MANIFEST
[package]
name = "convex-v1-probe"
version = "0.0.0"
edition = "2024"
[dependencies]
convex-typegen = { path = "$PWD" }
convex = "=0.10.4"
tokio = { version = "1", features = ["macros", "rt-multi-thread", "time"] }
futures = "0.3"
EOF_MANIFEST
cp Cargo.lock /tmp/convex-v1-probe/Cargo.lock
cat >/tmp/convex-v1-probe/src/main.rs <<'PROBE'
use std::{env, fs, path::PathBuf};
use convex_typegen::prelude::*;
fn main() {
    let base = PathBuf::from(env::args().nth(1).unwrap());
    let convex_dir = base.join("convex");
    let schema_path = convex_dir.join("schema.ts");
    let out_file = base.join("out.rs");
    let function_paths = env::args().skip(2).map(|p| convex_dir.join(p)).collect();
    let config = Configuration { convex_dir, schema_path, out_file: out_file.clone(), function_paths };
    println!("inputs: {:?}", rcfp(&config).unwrap());
    match convex_typegen::generate(config) {
        Ok(()) => println!("{}", fs::read_to_string(out_file).unwrap()),
        Err(err) => { eprintln!("{err:?}"); std::process::exit(1); }
    }
    for json in [convex_typegen::serde_json::json!(1), convex_typegen::serde_json::json!(1.0), convex_typegen::serde_json::json!([0,255])] {
        println!("{json} -> {:?}", json.clone().into_convex_value());
    }
    for value in [convex::Value::Bytes(vec![0,255]), convex::Value::Float64(f64::INFINITY)] {
        println!("{value:?} -> {}", value.clone().into_serde_value());
    }
}
PROBE
export CARGO_TARGET_DIR=/tmp/convex-v1-probe-target
cargo +1.95 build --manifest-path /tmp/convex-v1-probe/Cargo.toml
set -eu
probe=/tmp/convex-v1-probe-target/debug/convex-v1-probe
fixture="$PWD/tests/fixtures/v1-review"
scratch=$(mktemp -d)
mkdir -p "$scratch/convex/admin"
cp "$fixture"/convex/{http,crons,convex.config,users,validators}.ts "$scratch/convex/"
cp "$fixture"/convex/admin/users.ts "$scratch/convex/admin/"
run() { echo "=== $*"; "$probe" "$scratch" "$@" >"$scratch/log" 2>&1 || true; grep -E 'inputs:|Invalid|FUNCTION_PATH|pub (struct|enum)|pub (note|case|key|record|camelCase|sample):| -> ' "$scratch/log"; }
cp "$fixture/convex/schema.ts" "$scratch/convex/"
run users.ts
printf 'export default defineSchema({ tasks: defineTable({ text: v.string() }) });\n' > "$scratch/convex/schema.ts"
run
run http.ts crons.ts convex.config.ts
run users.ts
run admin/users.ts
sed '/export const echo = /d' "$fixture/convex/admin/users.ts" > "$scratch/convex/admin/users.ts"
run admin/users.ts
run users.ts admin/users.ts
for v in 'float64()' 'bigint()' 'nullable(v.string())' 'function()' 'commitTs()'; do
  printf 'export default defineSchema({ tasks: defineTable({ sample: v.%s }) });\n' "$v" > "$scratch/convex/schema.ts"
  run users.ts
done
printf 'export default defineSchema({ tasks: defineTable({ key: v.id("tasks"), camelCase: v.string(), sample: v.literal("fixed"), record: v.record(v.string(), v.int64()), case: v.union(v.literal("red"), v.literal("blue")) }) });\n' > "$scratch/convex/schema.ts"
cat >"$scratch/convex/synthetic.ts" <<'TS'
export const fake = helper({});
export const q = query({args: { sample: v.union(v.string(), v.null()) }, returns: v.string()});
TS
run synthetic.ts
printf 'export const x = helper.method({});\n' > "$scratch/convex/synthetic.ts"
run synthetic.ts
printf 'export const x = query({args: {...shared}});\n' > "$scratch/convex/synthetic.ts"
run synthetic.ts


mkdir -p /tmp/convex-v1-wire/convex
printf 'export default defineSchema({ tasks: defineTable({ text: v.string() }) });\n' >/tmp/convex-v1-wire/convex/schema.ts
cp tests/fixtures/v1-review/convex/users.ts /tmp/convex-v1-wire/convex/
"$probe" /tmp/convex-v1-wire users.ts >/dev/null
cp /tmp/convex-v1-wire/out.rs /tmp/convex-v1-probe/src/generated.rs
cat >/tmp/convex-v1-probe/src/bin/wire.rs <<'PROBE'
#[path = "../generated.rs"]
mod generated;
use std::{collections::BTreeMap, time::Duration};
use convex::{ConvexClient, FunctionResult, Value};
use convex_typegen::ConvexClientExt;
use futures::StreamExt;
use generated::*;
#[tokio::main]
async fn main() {
    let mut client = ConvexClient::new("http://127.0.0.1:13210").await.unwrap();
    let args = ConvexClient::prepare_args(UsersOptionalArgs { root: None, nested: UsersOptionalNested { note: None } }).unwrap();
    println!("generated optional args: {args:?}");
    let result = client.query(UsersOptionalArgs::FUNCTION_PATH, args).await.unwrap();
    assert!(matches!(result, FunctionResult::ErrorMessage(ref msg) if msg.contains(".nested.note")), "{result:?}");
    let args = ConvexClient::prepare_args(UsersNumberArgs { value: 1.0 }).unwrap();
    assert!(matches!(args["value"], Value::Float64(1.0)));
    let result = client.query(UsersNumberArgs::FUNCTION_PATH, args).await.unwrap();
    assert!(matches!(result, FunctionResult::Value(Value::Float64(1.0))), "{result:?}");
    let error = client.action("admin/users:fail", BTreeMap::new()).await.unwrap();
    println!("structured error: {error:?}");
    assert!(matches!(error, FunctionResult::ConvexError(ref err) if matches!(err.data, Value::Object(_))));
    let mut sub = client.subscribe("admin/users:docs", BTreeMap::new()).await.unwrap();
    let first = tokio::time::timeout(Duration::from_secs(5), sub.next()).await.unwrap().unwrap();
    let added = client.mutation("admin/users:add", BTreeMap::from([("text".into(), Value::String("subscription probe".into()))])).await.unwrap();
    assert!(matches!(added, FunctionResult::Value(Value::String(_))));
    let second = tokio::time::timeout(Duration::from_secs(5), sub.next()).await.unwrap().unwrap();
    let (FunctionResult::Value(Value::Array(first)), FunctionResult::Value(Value::Array(second))) = (first, second) else { panic!("expected array updates") };
    assert_eq!(first.len()+1, second.len());
    println!("raw SDK subscription: {} -> {}", first.len(), second.len());
}
PROBE
cargo +1.95 run --manifest-path /tmp/convex-v1-probe/Cargo.toml --bin wire
```

</details>
