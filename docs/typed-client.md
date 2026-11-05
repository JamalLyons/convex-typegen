# Typed Convex client

Wrap a client from `convex::ConvexClient::new` or `convex::ConvexClientBuilder`.
The adapter borrows it mutably because the SDK takes `&mut self` for calls and
subscription setup. Use `client.client_mut()` to access SDK methods such as
`set_auth` or `set_admin_auth` while the adapter is in scope.

```rust,ignore
use convex_typegen_runtime::TypedClient;
use futures::StreamExt;

let mut client = TypedClient::new(&mut sdk);
let messages = client.query(api::messages::list, api::messages::ListArgs {}).await?;
let mut updates = client.subscribe(api::messages::list, api::messages::ListArgs {}).await?;
let sent = client.mutation(api::messages::send, send_args).await?;

while let Some(update) = updates.next().await {
    println!("{update:?}");
}
```

The reference determines the operation kind, argument type, and return type.
`action` works the same way with action references. Only queries can be
subscribed to. The stream owns the SDK subscription and releases it on drop.
Keep the SDK client alive while polling the stream. Each update is decoded
independently, so an error item can be followed by a successful update.

## Application error payloads

Convex function specs have no thrown-error schema. Declare the payload once on a
generated reference using `with_error::<T>()`, where `T` implements
`serde::Deserialize` for owned data. Calls and subscriptions then decode that
payload automatically. This can be a reusable `const` or a local binding.

```rust,ignore
use convex_typegen_runtime::{CallError, serde::Deserialize};

#[derive(Debug, Deserialize)]
#[serde(crate = "convex_typegen_runtime::serde")]

struct SendFailure {
    code: String,
}

let send = api::messages::send.with_error::<SendFailure>();
match client.mutation(send, send_args).await {
    Err(CallError::Application { data: Ok(failure), .. }) => {
        eprintln!("Application error code: {}", failure.code);
    }
    Err(CallError::Application { message, data: Err(invalid) }) => {
        eprintln!("{message}: {}", invalid.source);
        // invalid.raw is the original convex::Value.
    }
    other => { /* handle success or another CallError variant */ }
}
```

For an endpoint without a known schema, the generated reference carries
`convex::Value` as its application error type. Use `send.raw_errors()` to
explicitly fall back to `convex::Value` on a reference with a declared type.

`CallError` separates argument encoding (`Encode`), successful return-value
decoding (`Decode`), SDK request failures (`Transport`), plain server error
messages (`Server`), and thrown `ConvexError` values (`Application`). A malformed
application payload stays in `Application`. Its `data` is
`Err(InvalidApplicationData)` with the original value and the path-aware decoder
error. A closed SDK subscription ends the stream.
