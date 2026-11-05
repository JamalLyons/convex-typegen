//! Offline Cargo build script using generated output in OUT_DIR.

fn main()
{
    if let Err(e) = convex_typegen::build::generate_out_dir() {
        panic!("convex-typegen failed: {e}");
    }
}
