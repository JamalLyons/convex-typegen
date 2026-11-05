# basic — convex-typegen example

Small Rust CLI that talks to a Convex backend using types generated from this folder’s `convex/` sources.

## Prerequisites

- Rust toolchain (`cargo`)
- Bun.js (for the Convex CLI and `convex dev`)

## 1. Convex backend

From **this directory** (`examples/basic/`):

```bash
bun install
```

Start a dev deployment (creates/updates `.env.local` with `CONVEX_URL` in many setups):

```bash
bunx convex dev --tail-logs
```

If you do not use Convex’s auto env file, create **`.env.local`** next to `Cargo.toml` with:

```bash
CONVEX_URL=https://<your-deployment>.convex.cloud
```

Use the deployment URL from the Convex dashboard or the `convex dev` output.

## 2. Rust app

Still in `examples/basic/`:

```bash
cargo build   # runs build.rs → generates OUT_DIR/convex_api.rs
cargo run     # number-guessing demo; reads .env.local via CARGO_MANIFEST_DIR
```

From the **repo root** you can run:

```bash
cargo run --manifest-path examples/basic/Cargo.toml
```

`build.rs` uses the OUT_DIR helper (`convex/schema.ts`, `convex/` discovery). `convex_typegen_runtime::include!()` loads the generated bindings from Cargo's `OUT_DIR`, so builds leave the source tree untouched.

## What to change

- **Schema / functions:** edit `convex/schema.ts` and the `*.ts` files under `convex/` (except `_generated/`). Rebuild the Rust crate to refresh types.
- **Game logic:** see `convex/games.ts` and the queries/mutations the CLI calls via generated `convex_api`.
