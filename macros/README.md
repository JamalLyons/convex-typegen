# convex-typegen-macros

Implements the `ConvexValue` derive for generated bindings and handwritten
Convex models. The derive provides Serde implementations for use with the
runtime's Convex value codec.

Apps depend on `convex-typegen-runtime`, which re-exports the derive as
`convex_typegen_runtime::ConvexValue`.

See the [convex-typegen website](https://ctg.jamallyons.com/)
for setup and examples.
