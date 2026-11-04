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

## Generated-project correctness properties

`just check` runs `xtask/tests/property_generated_projects.rs` on the `small` tier. Each run
chooses a fresh seed unless `CONVEX_TYPEGEN_SEED` is set, executes the full parse → IR → codegen
pipeline for four consecutive seeds, then compiles and runs one generated consumer. The consumer
round-trips 16 seeded values through the emitted `AccountsDoc` and the direct Convex `Value` codec.
The values cover Int64 and Float64 (including integral floats), bytes, nested records, optional
field omission/presence, nullable values, literals, and unions. A failing assertion or generated
consumer diagnostic includes its seed and tier.

Set a tier and seed to replay a run or broaden the source projects while keeping compile cost
bounded to one representative consumer:

```sh
CONVEX_TYPEGEN_SEED=42 CONVEX_TYPEGEN_TIER=small cargo test --locked -p xtask --test property_generated_projects
CONVEX_TYPEGEN_SEED=42 CONVEX_TYPEGEN_TIER=medium cargo test --locked -p xtask --test property_generated_projects
CONVEX_TYPEGEN_SEED=42 CONVEX_TYPEGEN_TIER=large cargo test --locked -p xtask --test property_generated_projects
CONVEX_TYPEGEN_SEED=42 CONVEX_TYPEGEN_TIER=huge cargo test --locked -p xtask --test property_generated_projects
```

The generator is seed-driven rather than shrink-based, so failures replay exactly instead of
being minimized automatically. When a failure is reduced to a smaller input, retain that input
under `tests/fixtures/property-regressions/` and keep its seed and tier in the fixture name or
metadata.

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

Both runs used seed `630063` on an Apple M4 (arm64), macOS 27.0.1, and
`rustc 1.99.1 (58cf8f9c3 2026-10-13)`. The measured source is
`d21a00a66e5e9feb2c4f289d2a6bbc6ef12e3c88`, before the typed model changes from #40 were integrated.

Each cell lists run 1 / run 2. Criterion times are the center values printed by Criterion.
The other columns come from one completed invocation per stage per run.

| TS files | Stage | Criterion wall time | Single pass (ns) | Peak extra heap (bytes) | Allocation calls |
| ---: | --- | --- | ---: | ---: | ---: |
| 12 | parse_discovery | 2.5074 ms / 2.4631 ms | 2460208 / 2477917 | 1910988 / 1910988 | 36727 / 36727 |
| 12 | ir_lowering | 6.3179 ms / 6.2311 ms | 6045917 / 5563334 | 456914 / 473053 | 18423 / 18573 |
| 12 | codegen | 117.15 µs / 96.286 µs | 335542 / 267375 | 23354 / 23354 | 1701 / 1701 |
| 12 | end_to_end | 9.0955 ms / 9.1011 ms | 9208583 / 8880750 | 2333605 / 2333693 | 56880 / 56931 |
| 100 | parse_discovery | 21.239 ms / 21.081 ms | 21170625 / 21096959 | 12607919 / 12607919 | 295696 / 295696 |
| 100 | ir_lowering | 453.14 ms / 449.00 ms | 463678625 / 451673334 | 4942995 / 4869795 | 1400083 / 1395416 |
| 100 | codegen | 498.36 µs / 486.43 µs | 817250 / 776125 | 169161 / 169161 | 13946 / 13946 |
| 100 | end_to_end | 514.22 ms / 506.08 ms | 501011750 / 479489625 | 17345202 / 17348464 | 1715346 / 1721957 |
| 1000 | parse_discovery | 217.15 ms / 209.00 ms | 213052917 / 203405500 | 117503503 / 117503503 | 2867320 / 2867320 |
| 1000 | ir_lowering | 52.438 s / Incomplete | 52267707334 / 51169518667 | 49509330 / 49626634 | 141281946 / 142784169 |
| 1000 | codegen | 4.5049 ms / Incomplete | 4887000 / 4725417 | 1517160 / 1517160 | 138131 / 138131 |
| 1000 | end_to_end | 55.709 s / Incomplete | 55633325166 / 51457965500 | 165610453 / 165540475 | 144626407 / 144404646 |
| 10,240 | All | Not measured | — | — | — |

Run 1 completed Criterion sampling through 1,000 files before it was stopped. Run 2 used
`CONVEX_TYPEGEN_SKIP_HUGE=1` and was interrupted during the 1,000-file IR sampling. Its single
passes for all four stages had already completed. The 10,240-file tier was omitted because
the 1,000-file IR and end-to-end samples each took tens of seconds per pass on this host.

The same-seed comparison did not establish that timings were within noise across the suite.
Criterion flagged differences on the unchanged binary, including small-tier codegen. Three
of the nine completed comparisons reported no change or a change within the noise threshold.
