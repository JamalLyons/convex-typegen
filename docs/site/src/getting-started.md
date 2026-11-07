# Build your first project

Build a task list that writes a row and observes it through a live query. By the end you will
have called all three Convex function kinds and handled a typed application error.

This tutorial is the shortest path to **end-to-end type safety**: after generation, `tasks::list`
expects `ListArgs`, returns `Vec<String>`, and `tasks::add` returns `Id<tables::Tasks>`—the
compiler rejects the wrong reference, args, or result type before you deploy. See
[what's guaranteed and what isn't](docs/type-safety.md) for compile-time limits and runtime checks.

The generator reads your [schema](https://docs.convex.dev/database/schemas) and local TypeScript or
JSON and stays offline during builds. It requires Rust **1.97**; the runtime supports **1.85**.

## 1. Create the project

Install Rust 1.97 or newer, Node.js 22 or newer with npm, and Docker. Node and Docker are for
the backend setup and deployment. Cargo can generate and compile the bindings without either.

```sh
cargo new --edition 2024 typed-tasks
cd typed-tasks
mkdir convex
```

Replace the empty `[dependencies]` section of `Cargo.toml` with the two sections below.
Use matching release versions of the two typegen crates.

```toml
[dependencies]
convex = "0.10.4"
convex-typegen-runtime = "1"
futures = "0.3"
tokio = { version = "1.52.3", features = ["macros", "rt-multi-thread", "time"] }

[build-dependencies]
convex-typegen = "1"
```

Copy this `package.json` to the project root. The Convex CLI ships with the Convex npm package.

```json
{{#include ../../../examples/readme/package.json}}
```

## 2. Declare the Convex schema and functions

Create `convex/schema.ts`. The optional priority can be omitted, set to null, or set to one of two
strings. Rust preserves all three cases.

```typescript
{{#include ../../../examples/readme/convex/schema.ts}}
```

Create `convex/tasks.ts`. Each endpoint declares both validators, even when the argument object
is empty. The Convex CLI generates `_generated/server` during deployment; Rust generation can
recognize the import without that generated directory.

```typescript
{{#include ../../../examples/readme/convex/tasks.ts}}
```

To enable TypeScript validation on deployment, also create `convex/tsconfig.json`:

```json
{{#include ../../../examples/readme/convex/tsconfig.json}}
```

## 3. Generate and use the bindings

Create `build.rs` next to `Cargo.toml`:

```rust
{{#include ../../../examples/readme/build.rs}}
```

The build script reads the local validators and writes `convex_api.rs` into Cargo's `OUT_DIR`.
Add `src/main.rs` in stages, then use the complete file below.

### Connect and list

Create the [Convex Rust client](https://docs.rs/convex/latest/convex/), wrap it with
`TypedClient`, and run the `tasks:list`
[query](https://docs.convex.dev/functions/query-functions):

```rust,ignore
let mut sdk = ConvexClient::new(&std::env::var("CONVEX_URL")?).await?;
let mut client = TypedClient::new(&mut sdk);
let before: Vec<String> = client.query(tasks::list, tasks::ListArgs {}).await?;
```

### Reject bad input and call an action

Use `with_error` for a typed [application error](https://docs.convex.dev/functions/error-handling/application-errors)
payload. Send `priority: None` to omit the optional field on the wire. Run a
[mutation](https://docs.convex.dev/functions/mutation-functions) and an
[action](https://docs.convex.dev/functions/actions) the same way—pass the generated reference and
matching args struct.

### Subscribe after a write

Open a subscription on `tasks:list`, insert a row with high priority
(`Some(Some(tasks::AddArgsPriority::High))`), and wait until the stream reports the new text. See
[typed calls](typed-client.md#subscriptions).

### Complete `main.rs`

```rust
{{#include ../../../examples/readme/src/main.rs}}
```

Run `cargo check` now. Cargo may fetch Rust dependencies the first time; generation itself
reads only local files. The function references tell the compiler which kind of call to make
and which arguments and result to expect. For example, `tasks::add` takes `AddArgs` and returns
an `Id<tables::Tasks>`. See [generated models](docs/generated-models.md#set-clear-or-omit-a-priority)
for the three `priority` shapes.

## 4. Deploy and run locally

Start the [self-hosted Convex backend](https://docs.convex.dev/self-hosting) with Docker. This
example uses a disposable container on loopback:

```sh
docker run --detach --name typed-tasks-convex \
  --publish 127.0.0.1:3210:3210 --publish 127.0.0.1:3211:3211 \
  --env CONVEX_CLOUD_ORIGIN=http://127.0.0.1:3210 \
  --env CONVEX_SITE_ORIGIN=http://127.0.0.1:3211 \
  --env DISABLE_BEACON=true \
  ghcr.io/convex/convex-backend@sha256:e9c084e8bd6c1f249012c672193f0988c748824f55a24cdad45f6185a8462f9a
```

Once `curl -f http://127.0.0.1:3210/version` succeeds, generate a deployment admin key and deploy
the functions from the project root:

```sh
export CONVEX_SELF_HOSTED_URL=http://127.0.0.1:3210
export CONVEX_SELF_HOSTED_ADMIN_KEY="$(docker exec typed-tasks-convex ./generate_admin_key.sh | tail -n 1)"
npm install
npx convex dev --once --typecheck enable
unset CONVEX_SELF_HOSTED_ADMIN_KEY
CONVEX_URL=http://127.0.0.1:3210 cargo run --bin typed-tasks
```

On a fresh database, expect output like this (the ID and timestamp will differ):

```text
Before: []
Action: pong
Rejected: empty_text
Created: <task id>
Observed: Write Rust (<timestamp>)
```

The mutation writes the task, and the subscription delivers a new list containing it. Running
again starts with the previous rows. When finished, remove the disposable container with
`docker rm -f typed-tasks-convex`. See [deploy and connect](deployment.md) for environment and
authentication details, or [troubleshooting](troubleshooting.md) if the output differs.
