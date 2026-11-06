# convex-typegen

[![CI](https://github.com/JamalLyons/convex-typegen/actions/workflows/rust.yml/badge.svg)](https://github.com/JamalLyons/convex-typegen/actions/workflows/rust.yml)
[![crates.io](https://img.shields.io/crates/v/convex-typegen.svg)](https://crates.io/crates/convex-typegen)
[![docs.rs](https://docs.rs/convex-typegen/badge.svg)](https://docs.rs/convex-typegen)

Generate typed Rust bindings for your Convex schema and functions. Call queries, mutations,
and actions through the official Convex SDK, and receive live query updates as typed streams.

[Documentation](https://jamallyons.github.io/convex-typegen/) ·
[First project](https://jamallyons.github.io/convex-typegen/getting-started.html) ·
[Configuration](https://jamallyons.github.io/convex-typegen/configuration.html) ·
[Rust API](https://jamallyons.github.io/convex-typegen/api-reference.html)

## Quick start

Add matching v1 releases of `convex-typegen` as a build dependency and
`convex-typegen-runtime` as an application dependency. Generating bindings requires Rust 1.97;
using pregenerated bindings requires Rust 1.85.

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
