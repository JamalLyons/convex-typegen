# Typed calls and subscriptions

The runtime wraps the official [Convex Rust client](https://docs.rs/convex/latest/convex/) with
generated function references. These references constrain
[queries](https://docs.convex.dev/functions/query-functions),
[mutations](https://docs.convex.dev/functions/mutation-functions), and
[actions](https://docs.convex.dev/functions/actions) to the matching argument and return models,
expose query subscriptions as streams, and can declare a Rust type for
[application error](https://docs.convex.dev/functions/error-handling/application-errors) payloads.

Use `TypedClient` when the generated Rust types should validate arguments and
decode results for you. Create the official SDK client first and give the adapter
a mutable borrow. Use [the first project](getting-started.md#3-generate-and-use-the-bindings)
for a runnable consumer with the functions below.

## Calls

| Operation | Use it when | Tutorial example |
| --- | --- | --- |
| `query` | You need one result from a read. | `client.query(tasks::list, tasks::ListArgs {}).await?` |
| `mutation` | You need a transactional write and its result. | `client.mutation(tasks::add, args).await?` |
| `action` | You need to call a backend action, for example one that talks to another service. | `client.action(tasks::ping, tasks::PingArgs {}).await?` |

Pass the generated function reference as the first argument and its matching args as the
second. The Rust compiler rejects a query reference passed to `mutation`, or args from
a different endpoint. A declared empty args object still has its own type: pass
`tasks::ListArgs {}` to `tasks::list`. A declared return validator determines the result
type. See [generated models](docs/generated-models.md#raw-values-when-validators-are-missing) for undeclared validators.

Like the SDK, `query` opens a subscription, returns its first result, and then drops
the subscription.

## Subscriptions

Use `subscribe` when a view should stay current as data changes. Only queries can
be subscribed to. The tutorial opens the stream, writes a new row, and waits for the
write to appear in a result. `futures::StreamExt` provides `next()`.

```rust,ignore
{{#include ../../../examples/readme/src/main.rs:subscribe}}
```

The stream delivers an initial result and then reflects changes to the data read by
the query. Each item decodes independently. Keep the SDK client alive while polling
the stream; the stream owns the subscription and releases it on drop. A closed
subscription ends the stream. In an integration check, bound the wait and look for the
expected change rather than assuming it is the next item; the tutorial does both.

See [application errors](errors.md) to distinguish application failures from
encoding, decoding, and transport failures.

## Using the rest of the Convex SDK

`TypedClient` borrows a `&mut convex::ConvexClient`. Use its typed methods for
generated calls and `client_mut()` whenever you need the SDK directly. Set up
the connection and make any SDK clones before borrowing it.

Apps that call the SDK directly need a `convex` dependency whose version is
compatible with the runtime's, so both crates use the same SDK types. See the
[semver policy](docs/semver-policy.md#library-api) and the
[Convex Rust SDK docs](https://docs.rs/convex/0.10.4/convex/).

| SDK capability | Where to use it | Example |
| --- | --- | --- |
| Read once with `query` | Typed method | `client.query(tasks::list, tasks::ListArgs {}).await?` |
| Write with `mutation` | Typed method | `client.mutation(tasks::add, args).await?` |
| Run an `action` | Typed method | `client.action(tasks::ping, tasks::PingArgs {}).await?` |
| Follow one query with `subscribe` | Typed method | `client.subscribe(tasks::list, tasks::ListArgs {}).await?` |
| Set or clear a user token | `client_mut()` | `client.client_mut().set_auth(Some(token)).await;` Pass `None` to sign out. |
| Fetch fresh tokens with `set_auth_callback` | `client_mut()` | `client.client_mut().set_auth_callback(Some(fetch_token)).await;` See the callback type below. |
| Authenticate as a deployment admin | `client_mut()` | `client.client_mut().set_admin_auth(admin_key, None).await;` |
| Watch all active queries with `watch_all` | `client_mut()` | `let mut all = client.client_mut().watch_all();` This gives you the SDK's `QuerySetSubscription`. Use `all.next().await` with `futures::StreamExt`. |
| Make untyped calls | `client_mut()` | `client.client_mut().query(tasks::list.path(), raw_args).await?` See how to encode `raw_args` below. |
| Connect with default options | Before wrapping | `let mut sdk = convex::ConvexClient::new(url).await?;` |
| Configure client ID or connection-state notifications | Before wrapping | `convex::ConvexClientBuilder::new(url).with_client_id("tasks").with_on_state_change(state_tx)` |
| Connect with a builder | Before wrapping | `let mut sdk = convex::ConvexClient::new_from_builder(builder).await?;` You can also call `builder.build().await?`. |
| Share a connection across SDK handles | Before wrapping | `let mut other_sdk = sdk.clone();` Give each adapter its own mutable borrow. |

The SDK calls an auth callback with `force_refresh = true` on reconnect. Pass
that flag to your token provider so it can fetch a fresh token. For example,
if `auth_provider` is an `Arc` to your provider with an async
`token(force_refresh)` method returning a JWT:

```rust,ignore
let fetch_token: convex::AuthTokenFetcher = Box::new(move |force_refresh| {
    let provider = auth_provider.clone();
    Box::pin(async move {
        let jwt = provider.token(force_refresh).await?;
        Ok(convex::AuthenticationToken::User(jwt))
    })
});
client.client_mut().set_auth_callback(Some(fetch_token)).await;
```

Use the generated function's `path()` and encode its args as a Convex object
for an untyped SDK call. The SDK returns a `FunctionResult`, so you can decode
a successful value separately or inspect the SDK's error variants.

```rust,ignore
use convex::{FunctionResult, Value};
use convex_typegen_runtime::value_serde::{from_value, to_value};

let Value::Object(raw_args) = to_value(&tasks::ListArgs {})? else {
    return Err("expected a Convex argument object".into());
};
let result = client.client_mut().query(tasks::list.path(), raw_args).await?;
if let FunctionResult::Value(value) = result {
    let tasks: Vec<String> = from_value(value)?;
    println!("{tasks:?}");
}
```

A builder accepts a Tokio channel sender for connection-state updates.
Keep the receiver wherever your app handles connection status.

```rust,ignore
use convex::{ConvexClient, ConvexClientBuilder};
use convex_typegen_runtime::TypedClient;

let (state_tx, mut state_rx) = tokio::sync::mpsc::channel(8);
let builder = ConvexClientBuilder::new(url)
    .with_client_id("tasks")
    .with_on_state_change(state_tx);
let mut sdk = ConvexClient::new_from_builder(builder).await?;
let mut other_sdk = sdk.clone();
let mut client = TypedClient::new(&mut sdk);
let mut other_client = TypedClient::new(&mut other_sdk);
```
