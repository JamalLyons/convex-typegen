# Synthetic Convex projects

Use the repository's private `xtask` to generate fresh TypeScript inputs for performance and
correctness work. The generator writes a project manifest, a Convex schema, a shared validator
module, and endpoint modules at several directory depths. It only writes local files.

```sh
just gen-project small
```

The command prints `CONVEX_TYPEGEN_SEED=<u64>` before writing and reports the output directory
and generation time. By default the project goes to `target/generated-projects/<tier>-<seed>/`.
The time covers generation and file writes, excluding the Cargo build and any parsing or codegen.

## Size tiers

- `small` generates 12 TypeScript files for quick smoke runs.
- `medium` generates 100 TypeScript files.
- `large` generates 1,000 TypeScript files.
- `huge` generates 10,240 TypeScript files.

Counts include `convex/schema.ts` and `convex/shared.ts`. The remaining files each contain one
to three endpoints. Every project includes public and internal queries, mutations, and actions.
The seed varies paths, endpoint counts and argument shapes, and schema fields and table shapes.
Every schema includes the validator catalog in
[`xtask/src/project.rs`](../xtask/src/project.rs), an indexed table union, and imported validator
spreads. The source uses the standard Convex `_generated/server` import, which the offline Rust
parser recognizes and the Convex CLI supplies when preparing a project for a backend.

## Replay a run

Record the seed, tier, and repository revision. At the same revision the relative file paths and
contents are byte-identical for the same seed and tier. If the first directory still exists, use
`--out` to replay into a new one. The generator refuses to overwrite an existing project.

```sh
just gen-project small 42
CONVEX_TYPEGEN_SEED=42 cargo run --locked --release -p xtask -- small --out target/replay-env
cargo run --locked --release -p xtask -- small --seed 42 --out target/replay-flag
diff -r target/replay-env target/replay-flag
```

Without an explicit seed the tool reads fresh OS randomness. An explicit `--seed` (including
the positional seed passed through by Just) takes precedence over `CONVEX_TYPEGEN_SEED`.

Run `cargo test --locked -p xtask` to check reproducibility and exercise the offline
parser and codegen on generated projects.

## Pipeline benchmarks

The unpublished `convex-typegen-bench` workspace crate benchmarks four production stages using
projects written by the `xtask` generator above:

- `parse_discovery` — discovers function sources, expands relative imports, and parses the schema
  and function sources into ESTree JSON.
- `ir_lowering` — converts the parsed schema and function ASTs into `GenerationIr`.
- `codegen` — emits Rust from a prepared `GenerationIr`.
- `end_to_end` — runs `convex_typegen::generate` from the generated TypeScript project.

The generator creates each tier before Criterion starts its timed iterations. The `small` tier has
12 TypeScript files (the approximately 10-file tier); the other tiers have 100, 1,000, and 10,240
files.

```sh
just bench
CONVEX_TYPEGEN_SEED=42 just bench
CONVEX_TYPEGEN_SKIP_HUGE=1 just bench
```

Each run prints `CONVEX_TYPEGEN_SEED=<u64>`, including when a fresh OS-random seed is chosen. Set
`CONVEX_TYPEGEN_SEED` to replay identical inputs and compare commits. Criterion stores prior results
under `target/criterion`; running the same command again reports its statistical comparison with
the previous run. `CONVEX_TYPEGEN_SKIP_HUGE=1` omits the 10,240-file tier for faster local work.

Criterion reports wall time per stage and tier. A one-shot measurement for every stage also prints
`STAGE_METRIC` with elapsed nanoseconds, extra peak live heap bytes above the process baseline,
allocation-call count (successful alloc, alloc_zeroed, and realloc calls), and cumulative bytes
requested from the system allocator. The counting allocator is defined only in the benchmark crate;
these memory and allocation measurements are separate from Criterion's repeated timing samples.

### Baseline

Record only completed Criterion runs. Include the host model/CPU, OS version, `rustc --version`,
benchmark source commit, seed, and both same-seed run summaries. If a tier cannot be completed,
mark its measurements as not run and state why rather than estimating values.

| Tier | Stage | Wall time (run 1 / run 2) | Peak extra heap | Allocations | Seed |
| --- | --- | ---: | ---: | ---: | --- |
| Pending measured runs | | | | | |
