# convex-typegen

**End-to-end type safety for Convex in Rust.**

convex-typegen is the bridge that gives Rust the end-to-end type safety TypeScript developers get naturally from Convex's generated `api` and `Doc` types. Your schema and function signatures become Rust types at build time, so a wrong table, field, argument, or return type is a compile error, not a runtime bug. If your schema changes, your Rust code stops compiling until it matches.

[![CI](https://github.com/JamalLyons/convex-typegen/actions/workflows/rust.yml/badge.svg)](https://github.com/JamalLyons/convex-typegen/actions/workflows/rust.yml)
[![crates.io](https://img.shields.io/crates/v/convex-typegen.svg)](https://crates.io/crates/convex-typegen)
[![docs.rs](https://docs.rs/convex-typegen/badge.svg)](https://docs.rs/convex-typegen)

## Without and with generated types

Calling Convex with only the official Rust SDK means string paths, untyped argument maps, and manual decoding of every result:

```rust,ignore
use std::collections::BTreeMap;

use convex::{ConvexClient, FunctionResult, Value};

let mut client = ConvexClient::new(&url).await?;
let result = client.query("tasks:list", BTreeMap::new()).await?;
let texts: Vec<String> = match result {
    FunctionResult::Value(Value::Array(items)) => items
        .into_iter()
        .filter_map(|item| match item {
            Value::String(text) => Some(text),
            _ => None,
        })
        .collect(),
    _ => return Err("unexpected query result".into()),
};
```

With convex-typegen, the same call is checked at compile time (from the [first-project tutorial](https://jamallyons.github.io/convex-typegen/getting-started.html)):

```rust,ignore
use convex::ConvexClient;
use convex_api::api::tasks;
use convex_typegen_runtime::TypedClient;

let mut sdk = ConvexClient::new(&url).await?;
let mut client = TypedClient::new(&mut sdk);
let texts: Vec<String> = client.query(tasks::list, tasks::ListArgs {}).await?;
```

[Documentation](https://jamallyons.github.io/convex-typegen/) ·
[What's guaranteed at compile time](https://jamallyons.github.io/convex-typegen/docs/type-safety.html) ·
[First project](https://jamallyons.github.io/convex-typegen/getting-started.html) ·
[Configuration](https://jamallyons.github.io/convex-typegen/configuration.html) ·
[Rust API](https://jamallyons.github.io/convex-typegen/api-reference.html)

## Quick start

Add matching v1 releases of `convex-typegen` as a build dependency and
`convex-typegen-runtime` as an application dependency. Generating bindings requires Rust 1.97;
using pregenerated bindings requires Rust 1.85.

```toml
[dependencies]
convex-typegen-runtime = "1"

[build-dependencies]
convex-typegen = "1"
```

With `convex/schema.ts` next to your Rust project, add this `build.rs`:

```rust,ignore
fn main()
{
    convex_typegen::build::generate_out_dir().expect("generate Convex bindings");
}
```

Load the result inside a Rust module with `convex_typegen_runtime::include!()`.
Follow the [first project](https://jamallyons.github.io/convex-typegen/getting-started.html)
for the complete Cargo manifest, TypeScript validators, and typed Rust calls.

[Contributing](https://jamallyons.github.io/convex-typegen/contributing.html) ·
[Security](https://github.com/JamalLyons/convex-typegen/blob/master/SECURITY.md) ·
[MIT license](https://github.com/JamalLyons/convex-typegen/blob/master/LICENSE)
