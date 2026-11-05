# Typed client API

The runtime wraps the official Convex Rust client with generated function references. These
references constrain calls to the matching argument and return models, expose query subscriptions
as streams, and can declare a Rust type for application error payloads.

## Function references

Generated references encode the canonical Convex path and its operation, arguments, return, and
application error types. See the [runtime API reference](https://docs.rs/convex-typegen-runtime).

## Calls

Use `TypedClient` to call a generated query, mutation, or action reference. The SDK client remains
available through the adapter.

## Subscriptions

Queries can be subscribed to as streams of independently decoded results. Dropping the stream
releases the SDK subscription.

## Typed errors

Declare an application error payload on a reference with `with_error::<T>()`; malformed payloads
retain their original Convex value and decode error.

See the [runtime details](docs/typed-client.md) for the method example and typed application error
handling.
