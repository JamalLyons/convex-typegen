# Typed Convex clients for Rust

<div class="intro">
<p class="eyebrow">CONVEX-TYPEGEN · V1 DOCUMENTATION</p>
<p class="lead">Generate Rust types from your Convex schema and functions. Make typed calls with the official SDK and stream live query results into your app.</p>
<p><a class="primary-link" href="getting-started.html">Build your first project <span aria-hidden="true">→</span></a></p>
</div>

```rust,ignore
let tasks: Vec<String> = client
    .query(tasks::list, tasks::ListArgs {})
    .await?;
```

The generator reads local TypeScript or a committed function spec at build time. The runtime uses the generated references to encode your arguments and decode Convex results, including IDs, documents, and application errors.

<div class="doc-grid">
<a href="typed-client.html"><strong>Call and subscribe</strong><span>Use queries, mutations, actions, and live updates.</span></a>
<a href="generation.html"><strong>Set up generation</strong><span>Choose your inputs and where Cargo writes the bindings.</span></a>
<a href="docs/generated-models.html"><strong>Read the generated types</strong><span>Understand naming, wire values, and nullable fields.</span></a>
<a href="configuration.html"><strong>Look up a setting</strong><span>Find build options, CLI flags, defaults, and precedence.</span></a>
</div>

**Rust support:** 1.97 for the generator; 1.85 for the runtime with pregenerated bindings.
See [how generation works](docs/guide/concepts.md) for the crate split.
