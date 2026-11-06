# Getting started

Install matching `convex-typegen` and `convex-typegen-runtime` releases, then add a Cargo build
script that generates bindings into `OUT_DIR`. The generator requires Rust **1.97**;
the runtime and derive crate support Rust **1.85**. The generator reads your
[schema](https://docs.convex.dev/database/schemas) and local TypeScript or JSON inputs and stays
offline during builds.

Continue with the [full tutorial](docs/guide/getting-started.md), including the build script,
committed function-spec flow, typed calls, and local self-hosted verification.
