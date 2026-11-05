use std::alloc::{GlobalAlloc, Layout, System};
use std::hint::black_box;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use convex_typegen::bench_support;
use convex_typegen::config::Configuration;
use criterion::{BatchSize, Criterion, Throughput, criterion_group, criterion_main};
use tempfile::TempDir;
use xtask::project::{self, TIERS};

struct CountingAllocator;

static ALLOCATIONS: AtomicU64 = AtomicU64::new(0);
static ALLOCATED_BYTES: AtomicU64 = AtomicU64::new(0);
static LIVE_BYTES: AtomicU64 = AtomicU64::new(0);
static PEAK_LIVE_BYTES: AtomicU64 = AtomicU64::new(0);

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

// The allocator delegates each operation to System, recording only successful heap changes.
unsafe impl GlobalAlloc for CountingAllocator
{
    unsafe fn alloc(&self, layout: Layout) -> *mut u8
    {
        // SAFETY: The caller provides a valid layout as required by GlobalAlloc.
        let pointer = unsafe { System.alloc(layout) };
        if !pointer.is_null() {
            record_allocation(layout.size());
        }
        pointer
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8
    {
        // SAFETY: The caller provides a valid layout as required by GlobalAlloc.
        let pointer = unsafe { System.alloc_zeroed(layout) };
        if !pointer.is_null() {
            record_allocation(layout.size());
        }
        pointer
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout)
    {
        LIVE_BYTES.fetch_sub(layout.size() as u64, Ordering::Relaxed);
        // SAFETY: The caller provides the pointer and layout paired with its allocation.
        unsafe { System.dealloc(pointer, layout) };
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, new_size: usize) -> *mut u8
    {
        // SAFETY: The caller provides the pointer and layout paired with its allocation.
        let replacement = unsafe { System.realloc(pointer, layout, new_size) };
        if !replacement.is_null() {
            ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
            ALLOCATED_BYTES.fetch_add(new_size as u64, Ordering::Relaxed);
            if new_size >= layout.size() {
                update_live_bytes((new_size - layout.size()) as u64, true);
            } else {
                LIVE_BYTES.fetch_sub((layout.size() - new_size) as u64, Ordering::Relaxed);
            }
        }
        replacement
    }
}

fn record_allocation(bytes: usize)
{
    ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
    ALLOCATED_BYTES.fetch_add(bytes as u64, Ordering::Relaxed);
    update_live_bytes(bytes as u64, true);
}

fn update_live_bytes(bytes: u64, allocation: bool)
{
    let live = if allocation {
        LIVE_BYTES.fetch_add(bytes, Ordering::Relaxed).saturating_add(bytes)
    } else {
        LIVE_BYTES.fetch_sub(bytes, Ordering::Relaxed).saturating_sub(bytes)
    };
    let mut peak = PEAK_LIVE_BYTES.load(Ordering::Relaxed);
    while live > peak {
        match PEAK_LIVE_BYTES.compare_exchange_weak(peak, live, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => break,
            Err(observed) => peak = observed,
        }
    }
}

struct Fixture
{
    tier: &'static str,
    files: usize,
    _directory: TempDir,
    config: Configuration,
    generated_root: std::path::PathBuf,
}

fn fixed_or_random_seed() -> u64
{
    match std::env::var("CONVEX_TYPEGEN_SEED") {
        Ok(seed) => seed
            .parse::<u64>()
            .unwrap_or_else(|error| panic!("CONVEX_TYPEGEN_SEED must be a u64: {error}")),
        Err(std::env::VarError::NotPresent) => {
            let mut bytes = [0; 8];
            getrandom::fill(&mut bytes).expect("operating-system randomness is available");
            u64::from_le_bytes(bytes)
        }
        Err(error) => panic!("could not read CONVEX_TYPEGEN_SEED: {error}"),
    }
}

fn measure<T>(seed: u64, tier: &str, stage: &str, operation: impl FnOnce() -> T) -> T
{
    let live_before = LIVE_BYTES.load(Ordering::Relaxed);
    let allocations_before = ALLOCATIONS.load(Ordering::Relaxed);
    let allocated_bytes_before = ALLOCATED_BYTES.load(Ordering::Relaxed);
    PEAK_LIVE_BYTES.store(live_before, Ordering::Relaxed);
    let start = Instant::now();
    let output = operation();
    let elapsed_ns = start.elapsed().as_nanos();
    let peak_delta = PEAK_LIVE_BYTES.load(Ordering::Relaxed).saturating_sub(live_before);
    let allocation_count = ALLOCATIONS.load(Ordering::Relaxed).saturating_sub(allocations_before);
    let allocated_bytes = ALLOCATED_BYTES.load(Ordering::Relaxed).saturating_sub(allocated_bytes_before);
    println!(
        "STAGE_METRIC seed={seed} tier={tier} stage={stage} wall_ns={elapsed_ns} peak_extra_bytes={peak_delta} \
         allocation_count={allocation_count} allocated_bytes={allocated_bytes}"
    );
    output
}

fn pipeline(c: &mut Criterion)
{
    let seed = fixed_or_random_seed();
    let skip_huge = std::env::var_os("CONVEX_TYPEGEN_SKIP_HUGE").is_some_and(|value| value != "0");
    println!("CONVEX_TYPEGEN_SEED={seed}");
    if skip_huge {
        println!("CONVEX_TYPEGEN_SKIP_HUGE=1");
    }

    // Project creation and source writes happen before Criterion starts any timed iteration.
    let mut fixtures = Vec::new();
    for (tier, files) in TIERS {
        if tier == "huge" && skip_huge {
            continue;
        }
        let directory = TempDir::new().expect("create benchmark temp directory");
        let generated_root = directory.path().join(format!("{tier}-{seed}"));
        project::write_project(&generated_root, files, seed).expect("generate seeded Convex project");
        let config = Configuration {
            schema_path: generated_root.join("convex/schema.ts"),
            out_file: generated_root.join("convex_api.rs"),
            convex_dir: generated_root.join("convex"),
            function_paths: Vec::new(),
            function_spec_path: None,
        };
        println!(
            "GENERATED_PROJECT seed={seed} tier={tier} files={files} root={}",
            generated_root.display()
        );
        fixtures.push(Fixture {
            tier,
            files,
            _directory: directory,
            config,
            generated_root,
        });
    }

    for fixture in &fixtures {
        let parsed_template = bench_support::parse(&fixture.config).expect("parse generated project");
        let lowered_template = bench_support::lower(parsed_template.clone()).expect("lower generated project");
        let codegen_path = fixture.generated_root.join("bench-codegen.rs");
        let stage_label = format!("{}-{}", fixture.tier, fixture.files);

        // One-shot stage measurements include heap high-water, allocations and wall time.
        measure(seed, fixture.tier, "parse_discovery", || {
            black_box(bench_support::parse(&fixture.config).expect("parse generated project"))
        });
        let parsed_for_measure = parsed_template.clone();
        measure(seed, fixture.tier, "ir_lowering", || {
            black_box(bench_support::lower(parsed_for_measure).expect("lower generated project"))
        });
        measure(seed, fixture.tier, "codegen", || {
            bench_support::codegen(&codegen_path, &lowered_template).expect("generate Rust bindings")
        });
        let config_for_measure = fixture.config.clone();
        measure(seed, fixture.tier, "end_to_end", || {
            convex_typegen::generate(config_for_measure).expect("generate bindings end-to-end")
        });

        let mut group = c.benchmark_group(format!("tier/{stage_label}"));
        group.throughput(Throughput::Elements(fixture.files as u64));
        group.sample_size(10);
        group.warm_up_time(Duration::from_millis(500));
        group.measurement_time(Duration::from_secs(1));

        group.bench_function("parse_discovery", |b| {
            b.iter(|| black_box(bench_support::parse(&fixture.config).expect("parse generated project")));
        });
        group.bench_function("ir_lowering", |b| {
            b.iter_batched(
                || parsed_template.clone(),
                |parsed| black_box(bench_support::lower(parsed).expect("lower generated project")),
                BatchSize::PerIteration,
            );
        });
        group.bench_function("codegen", |b| {
            b.iter(|| {
                bench_support::codegen(&codegen_path, &lowered_template).expect("generate Rust bindings");
                black_box(())
            });
        });
        group.bench_function("end_to_end", |b| {
            b.iter_batched(
                || fixture.config.clone(),
                |config| {
                    convex_typegen::generate(config).expect("generate bindings end-to-end");
                    black_box(())
                },
                BatchSize::PerIteration,
            );
        });
        group.finish();
    }
}

criterion_group!(benches, pipeline);
criterion_main!(benches);
