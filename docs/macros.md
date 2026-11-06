# Macros

Use `ConvexValue` for Rust types you write yourself—custom error payloads, patch objects, or
`v.any()` results you decode by hand. Generated models already derive it; you only need the macro
when you add your own structs or enums.

Convert between your types and SDK values with the runtime codec:

```rust
use convex::Value;
use convex_typegen_runtime::{ConvexValue, value_serde::{from_value, to_value}};

#[derive(ConvexValue)]
struct Patch {
    title: String,
}

let wire: Value = to_value(&Patch { title: "Draft".into() })?;
let patch: Patch = from_value(wire)?;
```

The derive implements Serde’s `Serialize` and `Deserialize` with Convex wire rules. Pair it with
`to_value` / `from_value` rather than `serde_json` when talking to `convex::Value`.

## Include generated bindings

Keep `convex-typegen` in `[build-dependencies]` and `convex-typegen-runtime` in
`[dependencies]`. Call `convex_typegen::build::generate_out_dir()` from `build.rs`.
Then load the generated items in the module where you want them to live.

```rust
mod bindings {
    convex_typegen_runtime::include!();
}
```

The default filename is `convex_api.rs`. If your build script writes another
file under Cargo's `OUT_DIR`, use a relative filename such as
`convex_typegen_runtime::include!("bindings.rs")`.

## Derive a Convex model

```rust
use convex_typegen_runtime::{ConvexValue, value_serde::Bytes};

#[derive(ConvexValue)]
struct Attachment {
    #[convex(rename = "displayName")]
    display_name: String,
    #[convex(optional)]
    payload: Option<Bytes>,
    byte_count: i64,
}
```

- Named-field structs encode as objects and reject unknown fields on decode.
  Field names default to the Rust spelling, with the `r#` prefix removed.
- Use `#[convex(optional)]` on `Option<T>` to omit `None` and decode a present
  key as `T`. An unmarked `Option<T>` is required and nullable. An optional
  field can also accept null when its inner type accepts null, such as a union
  with a `()` branch.
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

The derive sees only the Rust item it annotates. The generator reads TypeScript or a committed
function spec offline. Keeping those inputs in `build.rs` lets Cargo rebuild when they change and
gives rust-analyzer a generated Rust file to navigate. Neither macro runs generation or touches the
network.

For parser evaluation notes aimed at maintainers, see [Oxc AST evaluation](oxc-ast-evaluation.md).
