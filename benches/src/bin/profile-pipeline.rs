//! One production generation pass for CPU sampling or heap profiling.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use convex_typegen::config::Configuration;

#[cfg(feature = "profile-heap")]
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

fn main() -> Result<(), Box<dyn std::error::Error>>
{
    let mut args = std::env::args_os().skip(1);
    let (Some(project), Some(output), None) = (args.next(), args.next(), args.next()) else {
        return Err("usage: profile-pipeline PROJECT_DIRECTORY OUTPUT_RS".into());
    };
    let convex_dir = PathBuf::from(project).join("convex");
    let config = Configuration {
        schema_path: convex_dir.join("schema.ts"),
        out_file: output.into(),
        convex_dir,
        function_paths: Vec::new(),
        function_spec_path: None,
    };
    #[cfg(feature = "profile-heap")]
    let elapsed = profile_heap(config)?;
    #[cfg(not(feature = "profile-heap"))]
    let elapsed = generate(config)?;

    println!("PIPELINE_WALL_NS={}", elapsed.as_nanos());
    Ok(())
}

fn generate(config: Configuration) -> Result<Duration, convex_typegen::prelude::ConvexTypeGeneratorError>
{
    let started = Instant::now();
    convex_typegen::generate(config)?;
    Ok(started.elapsed())
}

#[cfg(feature = "profile-heap")]
fn profile_heap(config: Configuration) -> Result<Duration, Box<dyn std::error::Error>>
{
    use std::sync::mpsc;

    let limit = std::env::var("PROFILE_HEAP_SECONDS")
        .ok()
        .map(|seconds| seconds.parse())
        .transpose()?;
    let Some(seconds) = limit else {
        let _profiler = dhat::Profiler::new_heap();
        return Ok(generate(config)?);
    };

    let (finished, wait) = mpsc::channel();
    let profiler = dhat::Profiler::new_heap();
    let watchdog = std::thread::spawn(move || {
        let started = Instant::now();
        if matches!(
            wait.recv_timeout(Duration::from_secs(seconds)),
            Err(mpsc::RecvTimeoutError::Timeout)
        ) {
            eprintln!("PARTIAL_HEAP_WALL_NS={}", started.elapsed().as_nanos());
            drop(profiler);
            std::process::exit(124);
        }
        drop(profiler);
    });

    let result = generate(config);
    let _ = finished.send(());
    watchdog.join().expect("heap profiler watchdog");
    Ok(result?)
}
