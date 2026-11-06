# Getting started

This tutorial uses the checked-in v1 review fixture and the executable consumer in
[`e2e/`](../../e2e/). Run it against the [open-source Convex backend](https://docs.convex.dev/self-hosting)
on your machine in Docker—not a Convex cloud deployment.

## Install the generator and runtime

Use matching releases: the generator runs during the Rust build, and the runtime is linked into
your application.

```sh
cargo add convex-typegen-runtime@0.4
cargo add --build convex-typegen@0.4
```

The generator needs Rust **1.97**. If you check in pre-generated bindings and skip generation in
`build.rs`, the runtime alone supports Rust **1.85**. See
[concepts](concepts.md#versions-and-rust-versions).

## Generate bindings into `OUT_DIR`

With the default Convex layout (`convex/schema.ts` and function modules under `convex/`), add a
build script that reads local files only (no network):

```rust,ignore
fn main() {
    if let Err(error) = convex_typegen::build::generate_out_dir() {
        panic!("convex-typegen failed: {error}");
    }
}
```

Include the generated module where you want the bindings to live:

```rust,ignore
mod generated {
    convex_typegen_runtime::include!();
}
use generated::api;
```

`generate_out_dir()` writes `$OUT_DIR/convex_api.rs` and tells Cargo when to rebuild. Table shapes
come from your [Convex schema](https://docs.convex.dev/database/schemas). Function argument and
return types come from [validators](https://docs.convex.dev/database/types) and
[argument validation](https://docs.convex.dev/functions/validation) on each exported
[query](https://docs.convex.dev/functions/query-functions),
[mutation](https://docs.convex.dev/functions/mutation-functions), or
[action](https://docs.convex.dev/functions/actions).

For a custom layout, point the generator at your tree:

```rust,no_run
use convex_typegen::{build, config::Configuration};

fn main() {
    build::generate(Configuration::default().with_convex_dir("backend/convex"))
        .expect("generate Convex bindings");
}
```

This reads `backend/convex/schema.ts`. Override the schema path with
`.with_schema_path("path/to/schema.ts")` when needed.

## Capture and refresh a committed function spec

A function spec is a JSON snapshot of your deployment’s public functions and their resolved
argument and return validators. Commit one when you want stable generation without re-parsing
every TypeScript edge case. The schema still comes from `convex/schema.ts`.

Refresh the repository fixture against a local backend on `http://127.0.0.1:13210` with the
fixture deployed and its admin key exported:

```sh
cd tests/fixtures/v1-review
npm ci --no-audit --no-fund
cd ../../..
CONVEX_SELF_HOSTED_URL=http://127.0.0.1:13210 \
CONVEX_SELF_HOSTED_ADMIN_KEY="$CONVEX_SELF_HOSTED_ADMIN_KEY" \
scripts/refresh-function-spec.sh
```

The script accepts loopback URLs only, canonicalizes the JSON, and replaces
`tests/fixtures/v1-review/function-spec.json` on success. It does not start Docker or deploy; use
`just e2e` for the full flow. See [the E2E guide](../e2e.md). Capture specs with the
[Convex CLI](https://docs.convex.dev/cli/overview) against your own backend the same way.

Wire a committed spec into your build script:

```rust,no_run
use convex_typegen::{build, config::Configuration};

fn main() {
    build::generate(
        Configuration::default()
            .with_convex_dir("backend/convex")
            .with_function_spec_path("backend/function-spec.json"),
    ).expect("generate Convex bindings");
}
```

Cargo tracks the schema, its imports, the spec, and related sources.

## Build a typed client

Create the official SDK client, then wrap it with `TypedClient`. The adapter borrows the SDK
mutably for calls and subscriptions. Authenticate with the SDK when you need to—for example
[`set_auth`](https://docs.rs/convex/latest/convex/) on the underlying client via
`client.client_mut()`.

The snippets below match the E2E consumer. They use `ignore` because `api` and `tables` exist only
after that project’s build script runs; `just e2e` compiles and runs the full program.

### List tasks with a query

Start by listing documents from the fixture’s `admin/users:docs` query. The generated reference
 picks the operation kind, argument struct, and return type for you.

```rust,ignore
use convex::ConvexClient;
use convex_typegen_runtime::TypedClient;

let mut sdk = ConvexClient::new(&url).await?;
let mut client = TypedClient::new(&mut sdk);

let titles: Vec<_> = client
    .query(
        api::admin::users::list,
        api::admin::users::ListArgs {},
    )
    .await?;
```

### Insert a row with a mutation

The `add` mutation takes a string and returns a table-scoped ID. Use that ID in later queries.

```rust,ignore
use convex_typegen_runtime::model::Id;

let id: Id<tables::Tasks> = client
    .mutation(
        api::admin::users::add,
        api::admin::users::AddArgs {
            text: "typed e2e".into(),
        },
    )
    .await?;
```

Optional fields encode cleanly when you omit them. The fixture’s `users:optional` query expects
nested optional strings to be absent—not sent as null:

```rust,ignore
let args = api::users::OptionalArgs {
    root: None,
    nested: api::users::OptionalArgsNested { note: None },
};
let present = client.query(api::users::optional, args).await?;
assert!(!present);
```

See [generated models](../generated-models.md#optional-and-null-fields) for when to use `None`
versus `Some(None)`.

### Stream live updates

[Subscriptions](https://docs.convex.dev/functions/query-functions) replay the query whenever its
result changes. After you mutate data the query reads, the stream yields another decoded value.

```rust,ignore
use futures::StreamExt;

let mut updates = client
    .subscribe(
        api::admin::users::docs,
        api::admin::users::DocsArgs {},
    )
    .await?;
let first = updates.next().await.expect("initial result")?;

let next_id = client
    .mutation(
        api::admin::users::add,
        api::admin::users::AddArgs {
            text: "subscription update".into(),
        },
    )
    .await?;
let second = updates.next().await.expect("updated result")?;
assert!(second.iter().any(|doc| doc._id == next_id));
```

Drop the stream to release the SDK subscription. Keep the SDK client alive while you poll it.

### Run an action and decode an application error

Actions follow the same call shape. Convex specs do not describe thrown error payloads—you declare
a Rust type on the reference when you care about the shape. See
[application errors](https://docs.convex.dev/functions/error-handling/application-errors).

```rust,ignore
use convex_typegen_runtime::{CallError, serde::Deserialize};

#[derive(Debug, Deserialize)]
#[serde(crate = "convex_typegen_runtime::serde")]
struct Denied {
    code: String,
    retry: bool,
}

let pong = client
    .action(api::admin::users::ping, api::admin::users::PingArgs {})
    .await?;
assert_eq!(pong, api::admin::users::PingReturn);

let fail = api::admin::users::fail.with_error::<Denied>();
match client.action(fail, api::admin::users::FailArgs {}).await {
    Err(CallError::Application { data: Ok(Denied { code, retry }), .. }) => {
        assert_eq!(code, "denied");
        assert!(!retry);
    }
    other => panic!("expected typed ConvexError payload, got {other:?}"),
}
```

If the payload does not match `Denied`, you still get `CallError::Application` with
`data: Err(InvalidApplicationData)` and the original Convex value. See
[typed client](../typed-client.md#application-error-payloads).

### Complete `main.rs`

<details>
<summary>Full program combining the steps above (also checked in as <code>e2e/src/main.rs</code> with extra wire-kind tests)</summary>

```rust,ignore
use convex::ConvexClient;
use convex_typegen_runtime::model::Id;
use convex_typegen_runtime::serde::Deserialize;
use convex_typegen_runtime::{CallError, TypedClient};
use futures::StreamExt;
use std::time::Duration;

mod generated {
    convex_typegen_runtime::include!();
}
use generated::*;

const CALL_TIMEOUT: Duration = Duration::from_secs(20);

#[derive(Debug, Deserialize)]
#[serde(crate = "convex_typegen_runtime::serde")]
struct Denied {
    code: String,
    retry: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let url = std::env::var("CONVEX_SELF_HOSTED_URL")?;
    let mut sdk = tokio::time::timeout(CALL_TIMEOUT, ConvexClient::new(&url)).await??;
    let mut client = TypedClient::new(&mut sdk);

    let nested = tokio::time::timeout(
        CALL_TIMEOUT,
        client.query(api::admin::users::list, api::admin::users::ListArgs {}),
    )
    .await??;
    assert_eq!(nested, ["nested"]);

    let id: Id<tables::Tasks> = tokio::time::timeout(
        CALL_TIMEOUT,
        client.mutation(
            api::admin::users::add,
            api::admin::users::AddArgs {
                text: "typed e2e".into(),
            },
        ),
    )
    .await??;

    let pong = tokio::time::timeout(
        CALL_TIMEOUT,
        client.action(api::admin::users::ping, api::admin::users::PingArgs {}),
    )
    .await??;
    assert_eq!(pong, api::admin::users::PingReturn);

    let fail = api::admin::users::fail.with_error::<Denied>();
    match tokio::time::timeout(CALL_TIMEOUT, client.action(fail, api::admin::users::FailArgs {})).await?
    {
        Err(CallError::Application {
            data: Ok(Denied { code, retry }),
            ..
        }) => {
            assert_eq!(code, "denied");
            assert!(!retry);
        }
        other => panic!("expected typed ConvexError payload, got {other:?}"),
    }

    let optional = api::users::OptionalArgs {
        root: None,
        nested: api::users::OptionalArgsNested { note: None },
    };
    let present =
        tokio::time::timeout(CALL_TIMEOUT, client.query(api::users::optional, optional)).await??;
    assert!(!present);

    let mut updates = tokio::time::timeout(
        CALL_TIMEOUT,
        client.subscribe(api::admin::users::docs, api::admin::users::DocsArgs {}),
    )
    .await??;
    let first = tokio::time::timeout(CALL_TIMEOUT, updates.next())
        .await?
        .expect("initial subscription value")?;
    assert!(first.iter().any(|doc| doc._id == id));

    let next_id = tokio::time::timeout(
        CALL_TIMEOUT,
        client.mutation(
            api::admin::users::add,
            api::admin::users::AddArgs {
                text: "subscription update".into(),
            },
        ),
    )
    .await??;
    let second = tokio::time::timeout(CALL_TIMEOUT, updates.next())
        .await?
        .expect("subsequent subscription value")?;
    assert!(second.iter().any(|doc| doc._id == next_id));

    Ok(())
}
```

</details>

## Verify from a clean checkout

Install `just`, Docker, Node.js/npm, `jq`, and stable Rust, then run from the repository root:

```sh
just e2e
```

The command starts the pinned self-hosted backend on loopback, deploys the fixture, captures a fresh
function spec, builds the Rust consumer offline, and runs query, mutation, action, typed error, and
subscription checks. It removes the container afterward. See [E2E details](../e2e.md).
