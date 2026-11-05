fn main()
{
    if let Err(e) = convex_typegen::build::generate_out_dir() {
        panic!("convex-typegen failed: {e}");
    }
}
