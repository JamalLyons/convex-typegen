# Getting started

This tutorial follows the checked-in v1 review fixture and the executable consumer in
[`e2e/`](../../e2e/). It uses only the open-source Convex backend running locally in Docker.
Do not point these steps at a Convex cloud deployment.

## Install the generator and runtime

Use matching releases: the generator runs during the Rust build, and the runtime is linked into
your application.

```sh
cargo add convex-typegen-runtime@0.4
cargo add --build convex-typegen@0.4
```

The generator needs Rust 1.97. A pre-generated consumer that does not build the generator can use
the runtime on Rust 1.85; see [concepts](concepts.md#versions-and-rust-versions).

## Generate bindings into `OUT_DIR`

With the default Convex layout (`convex/schema.ts` and function modules under `convex/`), use this
build script:

```rust,ignore
// This is build.rs. It is ignored by rustdoc because it runs in Cargo's build-script context;
// the same helper is built by examples/basic and the E2E consumer.
fn main() {
    if let Err(error) = convex_typegen::build::generate_out_dir() {
        panic!("convex-typegen failed: {error}");
    }
}
```

Then include the generated file in the Rust module where you want the bindings:

```rust,ignore
// This module is compiled after the build script has generated OUT_DIR/convex_api.rs.
mod generated {
    convex_typegen_runtime::include!();
}
use generated::api;
```

`generate_out_dir()` writes `$OUT_DIR/convex_api.rs`, emits Cargo rebuild directives for its
inputs, and does not invoke the Convex CLI or access the network. If your paths or output filename
differ, use `Configuration` and `generate` directly; see [macros](../macros.md).

## Capture and refresh a committed function spec

A function spec records the deployed public function inventory and resolved argument and return
validators. Commit it when you want generation to use that stable inventory. The schema still comes
from `convex/schema.ts`.

For a local self-hosted backend already running on `http://127.0.0.1:13210`, with the fixture
deployed and its admin key in the environment, refresh the repository fixture with:

```sh
cd tests/fixtures/v1-review
npm ci --no-audit --no-fund
cd ../../..
CONVEX_SELF_HOSTED_URL=http://127.0.0.1:13210 \
CONVEX_SELF_HOSTED_ADMIN_KEY="$CONVEX_SELF_HOSTED_ADMIN_KEY" \
scripts/refresh-function-spec.sh
```

The script rejects a non-loopback URL, canonicalizes and sorts the function data, and replaces
`tests/fixtures/v1-review/function-spec.json` only after successful capture. It does not start a
backend or deploy the fixture. The supported, one-command Docker deployment and verification flow
is `just e2e` below. See [the E2E guide](../e2e.md) for its pinned local backend, ports, and
prerequisites. Never substitute a cloud URL or key.

To use a committed spec in your own build script, set
`Configuration::function_spec_path: Some("convex/function-spec.json".into())`; with a custom
configuration, call `convex_typegen::generate(config)` and emit `cargo:rerun-if-changed` for paths
from `convex_typegen::prelude::rcfp(&config)`. The default `generate_out_dir()` helper discovers
TypeScript functions and does not enable a spec by itself. The E2E consumer shows the complete
custom-config pattern in [`e2e/build.rs`](../../e2e/build.rs).

## Call queries, mutations, and actions

The following excerpts match the E2E consumer. They are marked `ignore` because the generated
`api` and `tables` modules exist only after that consumer's build script runs; `just e2e` compiles
and runs the complete source.

```rust,ignore
// Compiled as part of e2e/src/main.rs after its build script generates `api`.
let nested = client
    .query(api::admin::users::list, api::admin::users::ListArgs {})
    .await?;

let id: Id<tables::Tasks> = client
    .mutation(
        api::admin::users::add,
        api::admin::users::AddArgs { text: "typed e2e".into() },
    )
    .await?;

let pong = client
    .action(api::admin::users::ping, api::admin::users::PingArgs {})
    .await?;
assert_eq!(pong, api::admin::users::PingReturn);
```

The reference chooses the operation, argument type, and return type. `TypedClient` borrows an
official `convex::ConvexClient` mutably; create and authenticate the SDK client first.

## Decode a typed `ConvexError`

Convex function specs do not include a thrown-error schema. Declare the payload type on a reference:

```rust,ignore
// Compiled as part of e2e/src/main.rs; this payload matches the fixture's ConvexError.
#[derive(Debug, Deserialize)]
#[serde(crate = "convex_typegen_runtime::serde")]
struct Denied {
    code: String,
    retry: bool,
}

let fail = api::admin::users::fail.with_error::<Denied>();
match client.action(fail, api::admin::users::FailArgs {}).await {
    Err(CallError::Application { data: Ok(Denied { code, retry }), .. }) => {
        assert_eq!(code, "denied");
        assert!(!retry);
    }
    other => panic!("expected typed ConvexError payload, got {other:?}"),
}
```

If the payload does not match `Denied`, the result remains `CallError::Application` with
`data: Err(InvalidApplicationData)`, including the original Convex value and a path-aware decode
error. See [typed client](../typed-client.md#application-error-payloads).

## Subscribe to a query

Subscriptions are streams of typed results. A successful mutation to data read by the query
produces a later update:

```rust,ignore
// Compiled as part of e2e/src/main.rs; `futures::StreamExt` provides `next()`.
let mut updates = client
    .subscribe(api::admin::users::docs, api::admin::users::DocsArgs {})
    .await?;
let first = updates.next().await.expect("initial result")?;

let next_id = client
    .mutation(
        api::admin::users::add,
        api::admin::users::AddArgs { text: "subscription update".into() },
    )
    .await?;
let second = updates.next().await.expect("updated result")?;
assert!(second.iter().any(|doc| doc._id == next_id));
```

The SDK subscription is released when the stream is dropped. Keep the SDK client alive while
consuming updates. `just e2e` bounds backend operations and subscription reads with timeouts.

## Verify from a clean checkout

Install `just`, Docker, Node.js/npm, `jq`, and stable Rust, then run from the repository root:

```sh
just e2e
```

The command starts the pinned self-hosted backend on loopback, deploys the fixture, captures a fresh
temporary function spec, then builds the Rust consumer with Cargo offline and executes query,
mutation, action, typed error, and subscription checks. It removes the container afterward. This
does not use Convex cloud. See [E2E details](../e2e.md).
