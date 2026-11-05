use std::path::PathBuf;

use convex_typegen::config::Configuration;
use convex_typegen::{check, generate};

fn main()
{
    let mut config = Configuration::default();
    let mut check_only = false;
    let mut output_supplied = false;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--check" => check_only = true,
            "--schema" => config = config.with_schema_path(value(&mut args, "--schema")),
            "--convex-dir" => config.convex_dir = value(&mut args, "--convex-dir"),
            "--output" => {
                config.out_file = value(&mut args, "--output");
                output_supplied = true;
            }
            "--function-spec" => config.function_spec_path = Some(value(&mut args, "--function-spec")),
            "--function" => config.function_paths.push(value(&mut args, "--function")),
            "--help" | "-h" => {
                println!(
                    "convex-typegen [--check] [--schema PATH] [--convex-dir PATH] [--output PATH] [--function-spec PATH] \
                     [--function PATH]"
                );
                return;
            }
            other => fail(&format!("unknown argument {other:?}; use --help")),
        }
    }
    if !output_supplied && std::env::var_os("OUT_DIR").is_none() {
        fail("--output is required outside a Cargo build script");
    }
    if check_only {
        match check(config) {
            Ok(true) => {}
            Ok(false) => std::process::exit(1),
            Err(error) => fail(&error.to_string()),
        }
    } else if let Err(error) = generate(config) {
        fail(&error.to_string());
    }
}
fn value(args: &mut impl Iterator<Item = String>, flag: &str) -> PathBuf
{
    args.next()
        .map(PathBuf::from)
        .unwrap_or_else(|| fail(&format!("{flag} requires a path")))
}
fn fail(message: &str) -> !
{
    eprintln!("convex-typegen: {message}");
    std::process::exit(2)
}
