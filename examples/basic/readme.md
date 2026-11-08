# basic: a typed notes CLI

The smallest complete tour of convex-typegen. `convex/` declares one table, an index, and
queries, mutations, and an action that all have `args` **and** `returns` validators.
[`src/main.rs`](src/main.rs) passes generated args structs in and gets generated types back,
with no manual value parsing. It is a companion to the [docs site](https://ctg.jamallyons.com/)
tutorial.

| Concept | Where |
| --- | --- |
| Table and index | `notes` and `by_done` in `convex/schema.ts` |
| String-literal union becomes a Rust enum | `priority` becomes `AddArgsPriority` |
| Optional vs nullable | `tag` can be missing (`Option<String>`); `priority` is `null` when unset |
| Typed `Id<tables::Notes>` | `done` and `rm` |
| Live subscription | `watch` |

## Prerequisites

Rust 1.97+, Node.js 22+ with npm, and Docker. Everything runs on your machine; no Convex cloud
account is needed.

## 1. Start a local Convex backend

From the repository root, start the open-source backend in Docker (the image is the one
`just e2e` pins). Ports bind to loopback only.

```bash
docker run --detach --name convex-notes \
  --publish 127.0.0.1:3210:3210 --publish 127.0.0.1:3211:3211 \
  --env CONVEX_CLOUD_ORIGIN=http://127.0.0.1:3210 \
  --env CONVEX_SITE_ORIGIN=http://127.0.0.1:3211 \
  --env DISABLE_BEACON=true \
  ghcr.io/convex/convex-backend@sha256:e9c084e8bd6c1f249012c672193f0988c748824f55a24cdad45f6185a8462f9a
```

Wait until `curl http://127.0.0.1:3210/version` answers, then create an admin key and deploy:

```bash
cd examples/basic
export CONVEX_SELF_HOSTED_URL=http://127.0.0.1:3210
export CONVEX_SELF_HOSTED_ADMIN_KEY="$(docker exec convex-notes ./generate_admin_key.sh | tail -n 1)"
npm ci
npx convex dev --once
```

## 2. Run the CLI

`build.rs` generates the bindings offline into `OUT_DIR`; only running the CLI talks to the backend.

```bash
export CONVEX_URL=http://127.0.0.1:3210
cargo run -- add "Write the docs" high docs
cargo run -- add "Water the plants"
cargo run -- list
cargo run -- done <id>      # id printed by `add` or `list`
cargo run -- rm <id>
cargo run -- summary        # runs the Convex action
cargo run -- watch          # prints the list again whenever it changes
```

`add TEXT [low|normal|high] [TAG]`. Leaving out the priority stores `null`; leaving out the tag
omits the field. Run `watch` in one terminal and `add` in another to see it update live.
You can put `CONVEX_URL=...` in `.env.local` instead of exporting it.

## Clean up

```bash
docker rm --force convex-notes
```

See the docs for [typed calls and subscriptions](https://ctg.jamallyons.com/typed-client.html)
and [what's guaranteed](https://ctg.jamallyons.com/docs/type-safety.html).
