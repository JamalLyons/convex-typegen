# Macros

## Include generated bindings

Use the runtime's `include!()` macro to load generated Rust in a module after
Cargo runs the build script. See [include bindings from Cargo output](../generation.md#include-bindings-from-cargo-output)
for the default and custom filename forms.

## Derive a Convex model

Use `ConvexValue` when you control a wire shape outside the generated validators,
for example an application error payload, an adapter around `v.any()`, or a patch
model. For an arbitrary result represented by `RawValue`, pass its inner SDK value
(`raw.0`) to `from_value::<YourModel>(raw.0)` once you know its shape.

The example below encodes a patch into an SDK `Value`, checks the exact wire keys and
values, and decodes it back. Use the [tutorial manifest](../getting-started.md#1-create-the-project),
save the file as `src/bin/wire.rs`, and run `cargo run --bin wire`. No backend is needed;
success means all assertions pass. The runtime supplies the derive and the codec, so
this example needs no direct Serde dependency.

```rust
{{#include ../../../../examples/readme/src/bin/wire.rs:wire}}
```

## Supported attributes and shapes

- Named-field structs encode as objects and reject unknown fields on decode.
  Field names default to the Rust spelling, with the `r#` prefix removed.
- Use `#[convex(optional)]` on `Option<T>` to omit `None` and decode a present
  key as `T`. An unmarked `Option<T>` is required and nullable. An optional
  field can also accept null when its inner type accepts null, such as the inner `Option<Priority>` above.
- Use `i64` for Int64, `f64` for Float64, `Bytes` for bytes, and
  `BTreeMap<K, V>` with string-shaped keys for records.
- Use a unit struct with `#[convex(literal = "created")]` for an exact wire
  literal. Booleans, signed 64-bit integers, and floats also work. Write `1`
  or `1_i64` for Int64 and `1.0` for Float64. The generator can use
  `#[convex(float_bits = ...)]` to preserve an exact Float64 bit pattern.
- For a string literal enum, annotate each unit variant with a distinct
  `#[convex(literal = "...")]` string. The enum encodes as that exact string.
- Tuple variants encode the payload of a single-field tuple variant directly. Decoding
  tries variants in declaration order. For a discriminated union, set
  `#[convex(discriminator = "kind")]` on the enum and `#[convex(tag = CreatedTag)]`
  on each variant, using its distinct literal type for the required `kind` field.
  The decoder selects the branch by that wire field and validates the whole payload.

The derive supports concrete types. Tuple structs, Rust unions, and variants
with unsupported field shapes produce compile-time diagnostics. For generated
function calls, use the typed references under `bindings::api`.

## Why generation stays in the build script

The derive sees only the Rust item it annotates. The generator and CLI read
TypeScript or a committed function spec offline. Keeping those inputs in
`build.rs` lets Cargo track changes and gives rust-analyzer a generated Rust
file to navigate. The derive crate only depends on Rust syntax and token
libraries, so using the runtime does not pull in Oxc.
