# Local self-hosted Convex E2E

The v1 typed-client acceptance gate deploys the committed review fixture to the
open-source Convex backend running in Docker on the same machine. It never uses
a Convex cloud deployment. Backend image version 24.04 is pinned by digest,
both exposed ports bind to loopback, and the runner creates a fresh admin key
inside the container for each run.

From the workspace root, run:

```sh
just e2e
```

Prerequisites are Docker, Node.js/npm, `jq`, current stable Rust, network access to the
locked package registries, and `just`. The script fetches locked Rust
dependencies first, then runs the consumer build offline. The local
backend uses `127.0.0.1:13210` (API) and `127.0.0.1:13211` (HTTP actions); keep
those ports free. The command installs the fixture's locked npm dependencies,
deploys it with the self-hosted CLI, captures a fresh function spec, then runs
the Rust consumer with Cargo offline. Its build script feeds the captured spec
to the generator and includes the generated bindings from `OUT_DIR`.

The executable checks nested typed queries, a typed mutation and returned
`Id`, `Doc` results, a successful action, and the decoded `ConvexError` payload.
It also checks root and nested function paths, omitted internal/HTTP/cron
functions, table-union serialization, omitted optional fields versus required
null fields, record values, exact Int64/Float64/Bytes echo, and the initial and
subsequent typed subscription results. Every backend operation and subscription
wait has a 20-second timeout; backend startup is bounded to 60 retries.

The runner always writes sanitized Docker output to
`target/e2e-artifacts/backend.log` (or `E2E_ARTIFACT_DIR/backend.log` when set)
and removes the backend container. The `Local self-hosted Convex E2E` workflow
runs on pull requests and pushes to `master`; it uploads that log even when the
job fails.
