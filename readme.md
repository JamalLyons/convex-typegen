# convex-typegen

[![CI](https://github.com/JamalLyons/convex-typegen/actions/workflows/rust.yml/badge.svg)](https://github.com/JamalLyons/convex-typegen/actions/workflows/rust.yml)
[![crates.io](https://img.shields.io/crates/v/convex-typegen.svg)](https://crates.io/crates/convex-typegen)
[![docs.rs](https://docs.rs/convex-typegen/badge.svg)](https://docs.rs/convex-typegen)
[![Runtime MSRV: 1.85](https://img.shields.io/badge/runtime_Rust-1.85-blue)](https://jamallyons.github.io/convex-typegen/docs/guide/concepts.html#versions-and-rust-versions)
[![Generator MSRV: 1.95](https://img.shields.io/badge/generator_Rust-1.95-blue)](https://jamallyons.github.io/convex-typegen/docs/guide/concepts.html#versions-and-rust-versions)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/JamalLyons/convex-typegen/blob/master/LICENSE)

Bring Convex's end-to-end type safety to Rust. **convex-typegen** turns your TypeScript schema and function metadata into typed function references, arguments, return values, table IDs, and documents. Generate bindings offline in `build.rs`, then query, mutate, run actions, and stream live updates through the official Convex Rust client—with the compiler checking your calls.

[Documentation](https://jamallyons.github.io/convex-typegen/) · [Migration from v0.4](https://jamallyons.github.io/convex-typegen/docs/guide/migration-v0.4.html) · [Support matrix](https://jamallyons.github.io/convex-typegen/docs/guide/support.html)

## Quick start

Use matching generator and runtime releases. The runtime supports Rust **1.85**; building the generator requires Rust **1.95**.

```sh
cargo add convex-typegen-runtime convex futures
cargo add tokio --features macros,rt-multi-thread
cargo add --build convex-typegen
```

Generate `$OUT_DIR/convex_api.rs` in `build.rs`:

```rust,ignore
fn main()
{
    convex_typegen::build::generate_out_dir().expect("generate Convex bindings");
}
```

For a `tasks:list` query with `args: {}` and `returns: v.array(v.string())`, use this `src/main.rs`:

```rust,ignore
#[allow(dead_code)]
mod convex_api
{
    convex_typegen_runtime::include!();
}

use convex::ConvexClient;
use convex_api::api::tasks;
use convex_typegen_runtime::TypedClient;
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>>
{
    let mut sdk = ConvexClient::new(&std::env::var("CONVEX_URL")?).await?;
    let mut client = TypedClient::new(&mut sdk);
    let tasks: Vec<String> = client.query(tasks::list, tasks::ListArgs {}).await?;
    println!("{tasks:?}");

    let mut updates = client.subscribe(tasks::list, tasks::ListArgs {}).await?;
    while let Some(tasks) = updates.next().await {
        println!("{:?}", tasks?);
    }
    Ok(())
}
```

These exact Rust snippets compile in [examples/readme](examples/readme), alongside the matching TypeScript schema and query. Set `CONVEX_URL` to your backend URL to run it. See the [tutorial](https://jamallyons.github.io/convex-typegen/getting-started.html) for setup and [local E2E testing](https://jamallyons.github.io/convex-typegen/docs/e2e.html) for `just e2e`.

## Contributing

Start with [CONTRIBUTING.md](CONTRIBUTING.md). Report vulnerabilities using [SECURITY.md](SECURITY.md).

MIT licensed. See [LICENSE](LICENSE).
