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

## Pipeline profiling findings (#64)

### Reference run and reproduction

The generator and fixture source were at `c1e5273d9008eeffc16de726bf0887b1b4b793b1`.
All captures used seed `630063`, with `large` confirmed at 1,000 TypeScript files and `huge`
at 10,240 files. The reference machine was an Apple Mac16,1 with an Apple M4 CPU (arm64),
17,179,869,184 bytes of RAM, macOS 27.0.1 (26A434), and an APFS volume. The toolchain was
`rustc 1.99.1 (58cf8f9c3 2026-10-13)`, Samply 0.13.1, and DHAT 0.3.3.

Run the commands from this repository root on macOS. The runner uses the release profile with
`CARGO_PROFILE_RELEASE_DEBUG=1`, `CARGO_PROFILE_RELEASE_STRIP=false`, and
`RUSTFLAGS="-C force-frame-pointers=yes"`. It generates the project before starting the
profiler, then calls the production `convex_typegen::generate` once with the synthetic
`convex/schema.ts`, source directory and fresh output path. This includes discovery,
parse, IR, codegen, and the atomic file write. Cargo, fixture generation, and profile
serialization are outside the timed call. The `profile-heap` feature is private to the
unpublished benchmark crate and installs DHAT only in the profiling binary. The wall mode
uses the standard allocator and neither profiler. These runs used freshly generated
files and overlapped with other local profiling or test jobs.

```sh
scripts/profile-pipeline.sh cpu large 630063
caffeinate -i scripts/profile-pipeline.sh heap large 630063
scripts/profile-pipeline.sh wall large 630063

caffeinate -i scripts/profile-pipeline.sh cpu huge 630063
caffeinate -i scripts/profile-pipeline.sh heap huge 630063

PROFILE_CPU_SECONDS=30 scripts/profile-pipeline.sh cpu huge 630063
PROFILE_CPU_SECONDS=120 scripts/profile-pipeline.sh cpu huge 630063
PROFILE_HEAP_SECONDS=180 scripts/profile-pipeline.sh heap huge 630063
PROFILE_HEAP_SECONDS=360 caffeinate -i scripts/profile-pipeline.sh heap huge 630063
```

Each command prints its working directory and the expanded build, generator, and profiler
commands. On this macOS setup, Samply launch worked without elevated permissions. Attaching
to an already-running process failed with `task_for_pid` error 5. Samply's `--duration`
capped sampling while the profiler continued waiting for the child. After the sampling
window, send `kill -TERM <profile-pipeline-pid>` to the child of the matching Samply process
to save the prefix. DHAT's optional watchdog saves the accumulated heap profile and exits
with status 124 when the requested time elapses before generation finishes.

The uncapped massive CPU and DHAT attempts were stopped before a Rust file was written.
At termination, `ps -o etime` reported 11:42 for the CPU process and 10:44 for the heap
process. The long CPU attempt also lost executable symbolization when Cargo replaced
the shared binary during the capture. The runner now copies the CPU executable into
each run directory before sampling. The bounded massive captures below cover discovery,
parse and early IR. They provide no complete massive-tier wall, peak, codegen, or
write baseline.

Raw captures, symbols, and fixtures go under ignored `target/profiling/`. The checked-in
[compact summaries](profiles/issue-64/) can be regenerated with Python 3 by passing the run
directory to the summarizer:

```sh
python3 scripts/summarize-profile.py cpu /path/to/cpu-run
python3 scripts/summarize-profile.py heap /path/to/heap-run
```

To inspect the original stacks, open `cpu.json.gz` with `samply load` and keep the
`cpu.json.syms.json` sidecar beside it. Open `dhat-heap.json` with the DHAT viewer.
Samply recorded the main thread at 99 Hz. The summaries group each stack once by the
matching stage or operation. They show inclusive sample counts and Samply's recorded
thread CPU deltas, attributed to the stack captured at the end of each interval.
DHAT reports cumulative allocation bytes and blocks, and live bytes at the global
heap peak within that capture. DHAT block counts follow DHAT's accounting and are
separate from the benchmark harness's allocation-call counts. An unspecified caller
means the stack identifies the operation without enough frames to assign its caller.

### Time

- **IR provenance indexing dominated CPU in both tiers.** In the completed 1,000-file capture,
  stacks containing `index_node_files` accounted for 81,259,549 of 89,092,108 recorded thread
  CPU microseconds (14,712 of 16,115 samples). The repeat attributed 58,796,006 of 65,135,089
  thread CPU microseconds to the same path (5,882 of 6,486 samples). The first 30 seconds of
  the 10,240-file capture included discovery, parsing and early IR. It attributed 10,919,766
  of 15,167,265 thread CPU microseconds to the index (2,124 of 2,630 samples). The
  120-second repeat attributed 106,885,703 of 118,990,333 thread CPU microseconds to
  the index (10,650 of 11,868 samples). Each function module constructs a `ValidatorSource`
  that traverses all loaded ASTs to build a node-to-file map. Other lowering was led by
  `ValidatorSource` teardown, with that frame at the leaf of 740 samples in the first
  complete capture. Rebuilding and dropping this map belongs in the same tuning pass.
  [Parser source](../src/convex/parser.rs), [follow-up #90](https://github.com/JamalLyons/convex-typegen/issues/90).
- **Oxc parsing, filesystem work, Rust emission, and the atomic write were small in the
  complete 1,000-file captures.** In the first capture, Oxc parsing, semantic checks and arena
  stacks accounted for 10 of 16,115 samples. Source filesystem stacks accounted for 29,
  with nine further samples in the remaining parse and discovery work. Codegen and the
  atomic write each appeared in one sample. In the repeat, codegen appeared in two samples
  and the write was below sampling resolution. The output file contained 1,596,603 bytes
  in both CPU runs, the DHAT run, and the wall run, and the generated bytes matched. The massive CPU
  prefix ended in IR, so the output-stage observations apply to the completed smaller tier.

The unprofiled 1,000-file pass took **64.506549583 s**. The Samply passes took
172.105379666 s and 65.523468000 s of wall time. Those are single observations under
different concurrent load, with compilation during the first CPU capture. Compare
stack shares here and use repeated unprofiled wall runs when evaluating a change.

### Allocations

The complete 1,000-file DHAT run took 945.703812 s of wall time under allocation
instrumentation. It recorded 41,530,964,817 cumulative bytes in 143,616,082 blocks
and 164,754,650 live bytes at the global heap peak.

- **Provenance indexing led cumulative allocation bytes.** In the complete 1,000-file run,
  the index accounted for 41,106,223,504 cumulative bytes and 140,393,518 blocks.
  In the 360-second massive DHAT capture, `index_node_files` accounted for 7,912,410,936 of
  11,889,265,633 cumulative bytes and 27,031,545 of 56,582,985 blocks. Its
  `BTreeMap<usize, &str>` node allocations contributed 428,529,864 of 1,633,280,194 live
  bytes at the observed peak. The shorter heap capture stopped during source loading and
  had no provenance-index allocations. [Follow-up #90](https://github.com/JamalLyons/convex-typegen/issues/90).
- **ESTree materialization dominated live heap during massive-tier source loading.**
  The bounded DHAT capture that ended during source loading attributed 2,470,806,596 of
  3,197,160,389 cumulative bytes and 729,893,450 of 732,801,288 live bytes at the observed
  heap peak to ESTree conversion. The evidence includes Oxc serialization buffers and
  allocations under `serde_json::Value` deserialization. Discovery and source loading both
  call the lexer. Stacks specifically attributed to discovery accounted for 451,685,663
  of those cumulative bytes and zero at the observed peak. Many recursive deserializer
  stacks identify the operation without retaining the caller. In the longer capture,
  after source loading completed and IR began, ESTree conversion still contributed
  1,201,287,172 of 1,633,280,194 live bytes at the observed heap peak. ESTree conversion
  also led peak live heap in the complete 1,000-file run, with 116,574,769 of 164,754,650
  bytes. [Follow-up #91](https://github.com/JamalLyons/convex-typegen/issues/91).
- Oxc parsing, semantics and arenas accounted for 635,638,672 cumulative bytes in that
  prefix. The largest points were `Arena::new_chunk` calls during parsing and semantic
  checks. This group accounted for 33,932 live bytes at the observed peak. That separates
  short-lived arena churn from the retained JSON trees that determine peak memory in
  source loading.
- In the complete 1,000-file run, Rust codegen allocated 24,947,346 cumulative bytes
  and the atomic write allocated 81 bytes. Both groups had zero live bytes at peak.

### Proposed targets for #67

- On the reference machine and seed, bring the unprofiled **10,240-file** run below
  **64.506549583 s**, the observed end-to-end wall time for **1,000 files**. Use the smaller
  tier's actual completion as the throughput target after removing repeated provenance
  indexing. Establish the complete massive-tier wall baseline before reporting a speedup.
- Bring live ESTree conversion bytes at the massive-tier heap peak below **1,201,287,172
  bytes**, the live ESTree bytes observed after all source loading and during early IR.
  Compare against this proposal with a completed pipeline and record the exact DHAT capture scope.
- Repeat comparisons with fresh seeds and report each seed and elapsed time alongside the
  replay of `630063`. Record memory and wall time in separate runs.
