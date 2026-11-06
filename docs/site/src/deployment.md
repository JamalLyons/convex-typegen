# Deploy and connect

## Deploy the TypeScript backend separately

Use the [local self-hosted setup](getting-started.md#4-deploy-and-run-locally) for the
tutorial. The Convex CLI uses `CONVEX_SELF_HOSTED_URL` and
`CONVEX_SELF_HOSTED_ADMIN_KEY` to deploy the functions and optionally
[capture the function spec](generation.md#use-a-committed-function-spec).
Keep the key in your deployment environment and out of source control.

Cargo builds your Rust program from local TypeScript and optional spec JSON. Deploying
a backend does not regenerate an existing Rust binary, and compiling Rust does not
deploy the TypeScript. After changing validators, deploy and rebuild so both sides
use the same contract.

## Connect at runtime

Pass the deployment's client URL to `convex::ConvexClient::new`. The tutorial reads
`CONVEX_URL` itself. Use the client API URL (port 3210 in the tutorial); the separate
site port is for HTTP actions.

Public endpoints may still require authentication. Set the user token with the
official SDK's `set_auth` before calls or subscription setup. The adapter's
`client_mut()` gives you access to the SDK while it is borrowed. The local tutorial
endpoints do not require auth. Keep deployment admin credentials out of distributed
client applications.

See the [official Rust SDK](https://docs.rs/convex/latest/convex/) for client options.
