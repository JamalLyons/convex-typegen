# Troubleshooting

## The build cannot find the schema

Check the path reported by the generator. In a workspace, relative paths start at
the package containing `build.rs`. Set the [Convex root or schema override](configuration.md#rust-configuration).

## A function is missing or its types are raw

Make sure the function is public and its module lives under the configured Convex
root. Declare both `args` and `returns`; `args: {}` explicitly declares no arguments.
Check the [supported source patterns](docs/guide/support.md#typescript-the-offline-parser-understands).
If you configured a spec, inspect that file for the endpoint and its validators and
[refresh it](generation.md#use-a-committed-function-spec) if it is stale.

## include! cannot find the output

Check that the build script ran in this same package and wrote the expected filename
under its `OUT_DIR`. Use the [filename form of the macro](generation.md#include-bindings-from-cargo-output)
for alternate Cargo output filenames, or a normal module declaration for committed bindings.

## Compilation succeeds but calls fail

Verify that the app's URL points to the deployment containing these functions.
Check the endpoint's authentication requirements. A `CallError::Decode` can mean the
deployed return validator and generated bindings are out of sync.
See [application errors](errors.md#choose-a-response) to identify the failure.

## A subscription times out

Keep the SDK client alive, poll the stream, and check that your mutation changes data
read by the query. A subscription emits query results, so a write that leaves the
result unchanged need not produce a new item. Use a unique write value in a smoke test.
