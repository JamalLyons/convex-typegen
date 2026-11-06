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

If you need to call the official SDK directly, the same reference exposes the wire
name through `tasks::list.path()`. Use `client.client_mut()` to reach SDK methods such as
[`set_auth`](https://docs.rs/convex/latest/convex/) while the adapter holds the mutable borrow.

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
