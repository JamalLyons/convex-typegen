use std::env;
use std::error::Error;
use std::path::PathBuf;
use std::time::Instant;

mod project;

const USAGE: &str = "Usage: xtask <small|medium|large|huge> [--seed <u64>] [--out <new-directory>]";

fn main() -> Result<(), Box<dyn Error>>
{
    let mut args = env::args_os().skip(1);
    let tier = args.next().ok_or(USAGE)?;
    if tier == "--help" || tier == "-h" {
        println!("{USAGE}");
        return Ok(());
    }
    let (tier, ts_files) = project::TIERS.into_iter().find(|(name, _)| tier == *name).ok_or(USAGE)?;

    let mut seed = None;
    let mut output = None;
    while let Some(flag) = args.next() {
        match flag.to_str() {
            Some("--seed") if seed.is_none() => {
                seed = Some(
                    args.next()
                        .and_then(|arg| arg.into_string().ok())
                        .ok_or("--seed requires a u64")?,
                );
            }
            Some("--out") if output.is_none() => {
                output = Some(PathBuf::from(args.next().ok_or("--out requires a directory")?));
            }
            _ => return Err(USAGE.into()),
        }
    }
    let seed = match (seed, env::var("CONVEX_TYPEGEN_SEED")) {
        (Some(seed), _) | (None, Ok(seed)) => seed.parse::<u64>()?,
        (None, Err(env::VarError::NotPresent)) => {
            let mut bytes = [0; 8];
            getrandom::fill(&mut bytes).map_err(|err| format!("could not obtain a random seed: {err}"))?;
            u64::from_le_bytes(bytes)
        }
        (None, Err(err)) => return Err(err.into()),
    };

    println!("CONVEX_TYPEGEN_SEED={seed}");
    let output = output.unwrap_or_else(|| PathBuf::from(format!("target/generated-projects/{tier}-{seed}")));
    let start = Instant::now();
    project::write_project(&output, ts_files, seed)
        .map_err(|err| format!("could not write project at {}: {err}", output.display()))?;
    println!(
        "Wrote {ts_files} TypeScript files to {} in {:.3} ms",
        output.display(),
        start.elapsed().as_secs_f64() * 1000.0
    );
    Ok(())
}
