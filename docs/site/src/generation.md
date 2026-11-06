# Choose generation inputs and output

## Use a custom layout or monorepo

Keep the Rust package's build script and point it at the backend root. For example,
if the package is `apps/desktop` and the Convex project is `apps/backend`:

```rust,no_run
use convex_typegen::{build, config::Configuration};

fn main() {
    build::generate(Configuration::default().with_convex_dir("../backend/convex"))
        .expect("generate Convex bindings");
}
```

This reads `../backend/convex/schema.ts`, derives module names relative to
`../backend/convex`, and leaves the output in Cargo's `OUT_DIR`. An explicit
`with_schema_path` takes precedence if your schema lives elsewhere. Use
`with_function_paths` if you only want a known subset of source files.

## Use a committed function spec

Choose a spec when the source parser cannot resolve a function's validators, or when you want a
reviewable snapshot of the deployed interface. After deploying to your local self-hosted
backend with both `CONVEX_SELF_HOSTED_*` variables set, run this from the backend project root:

```sh
npx convex function-spec > function-spec.json.tmp &&
  mv function-spec.json.tmp function-spec.json
```

Review and commit the JSON. In the preceding build script, add
`.with_function_spec_path("../backend/function-spec.json")` after `with_convex_dir`.
The spec replaces TypeScript function discovery entirely, including any explicit function
paths. Table schemas still come from TypeScript. Refresh the spec when your deployed function
inventory or validators change. Run deployment and capture as separate steps; the build script
does not need credentials, the Convex CLI, or a running backend.

## Commit bindings and check for drift

Use this when downstream consumers should compile without the generator, or when you want
to review binding changes in Git. Install the CLI for your matching release
(`cargo install convex-typegen --version 1 --locked`). During development of this repository,
use `cargo install --path /path/to/convex-typegen --locked` instead.

From the consumer package root, generate once, then run the check in CI:

```sh
convex-typegen --output src/convex_api.rs
convex-typegen --check --output src/convex_api.rs
```

Pass the same input flags to both invocations. Use `mod convex_api;` to load the checked-in
file from `src/`. Remove the generation build script and its build dependency if this is the
only generated output. Keep the matching runtime dependency. The drift check reads local
inputs and compares bytes without writing; see [CLI exit codes](configuration.md#cli).

## Include bindings from Cargo output

For the default output, put `convex_typegen_runtime::include!();` inside the module
that should own the generated items, as in the [first project](getting-started.md#3-generate-and-use-the-bindings).
If you need a different filename under `OUT_DIR`, use
`.with_out_file(PathBuf::from(std::env::var_os("OUT_DIR").expect("Cargo OUT_DIR")).join("bindings.rs"))`
in the build script (import `std::path::PathBuf`). Then use
`convex_typegen_runtime::include!("bindings.rs");` inside your Rust module.
The macro takes a string literal relative to `OUT_DIR`; it just includes Rust source.
