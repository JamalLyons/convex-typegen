# convex-typegen

Generate a fully typed Rust client for your Convex backend. `convex-typegen` reads your
Convex schema and function metadata during the Rust build, then emits typed function references,
argument and return models, table IDs, and documents. Its runtime calls the official Convex Rust
client and delivers live query updates as streams.

## A typed call

```rust,ignore
use convex_typegen_runtime::{TypedClient, model::Doc};
use generated::{api, tables};
use futures::StreamExt;

let mut client = TypedClient::new(&mut sdk);
let tasks: Vec<Doc<tables::Tasks>> = client.query(
    api::tasks::list,
    api::tasks::ListArgs { done: Some(false) },
).await?;
let mut updates = client.subscribe(api::tasks::list, api::tasks::ListArgs::default()).await?;
let latest = updates.next().await.expect("initial result")?;
```

The generated reference supplies the function path, argument type, and return type. See the
[getting started tutorial](getting-started.md) for setup and the local self-hosted end-to-end flow.

## Explore

- [Getting started](getting-started.md)
- [Guides](guides.md): build output, function specs, and local Convex testing
- [Typed client API](typed-client.md): references, calls, subscriptions, and typed errors
- [Supported features](supported-features.md)
- [Migration from v0.4](migration.md)
- [Rust API reference](api-reference.md)

Convex remains TypeScript on the backend. This project provides a typed Rust client; it does not
generate or run backend functions.
