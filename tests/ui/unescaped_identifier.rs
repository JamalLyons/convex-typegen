#[path = "../../target/model-tests/models.rs"]
mod models;

fn use_wire_name(args: models::api::models::RoundTripArgs)
{
    let _ = args.camelCase;
}

fn main() {}
