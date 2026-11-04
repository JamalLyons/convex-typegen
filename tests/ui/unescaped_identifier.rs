#[path = "../../target/model-tests/models.rs"]
mod models;

fn use_wire_name(args: models::ModelsRoundTripArgs)
{
    let _ = args.camelCase;
}

fn main() {}
