# Typed Convex clients for Rust

<div class="intro">
<p class="eyebrow">CONVEX-TYPEGEN · V1 DOCUMENTATION</p>
<p class="lead">convex-typegen gives Rust the end-to-end type safety TypeScript gets from Convex. Your schema and function signatures become Rust types at build time, so a wrong table, field, argument, or return type is a compile error—not a runtime bug. Schema changes break the Rust build until your code matches.</p>
<p><a class="primary-link" href="getting-started.html">Build your first project <span aria-hidden="true">→</span></a>
&nbsp;·&nbsp;
<a href="docs/type-safety.html">What's guaranteed and what isn't</a></p>
</div>

## Without and with generated types

**Official SDK only** — string paths, `BTreeMap` arguments, manual `Value` decoding:

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

**With convex-typegen** — typed paths, args, and returns (same tutorial as [first project](getting-started.md)):

```rust,ignore
use convex::ConvexClient;
use convex_api::api::tasks;
use convex_typegen_runtime::TypedClient;

let mut sdk = ConvexClient::new(&url).await?;
let mut client = TypedClient::new(&mut sdk);
let texts: Vec<String> = client.query(tasks::list, tasks::ListArgs {}).await?;
```

The generator reads local TypeScript or a committed function spec at build time. The runtime uses the generated references to encode your arguments and decode Convex results, including IDs, documents, and application errors.

<div class="doc-grid">
<a href="typed-client.html"><strong>Call and subscribe</strong><span>Use queries, mutations, actions, and live updates.</span></a>
<a href="generation.html"><strong>Set up generation</strong><span>Choose your inputs and where Cargo writes the bindings.</span></a>
<a href="docs/generated-models.html"><strong>Read the generated types</strong><span>Understand naming, wire values, and nullable fields.</span></a>
<a href="docs/type-safety.html"><strong>Compile-time guarantees</strong><span>What the types enforce—and what still runs on the server.</span></a>
</div>

**Rust support:** 1.97 for the generator; 1.85 for the runtime with pregenerated bindings.
See [how generation works](docs/guide/concepts.md) for the crate split.
