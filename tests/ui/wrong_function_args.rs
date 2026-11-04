#[path = "../../target/model-tests/models.rs"]
mod models;

use convex_typegen_runtime::function::FunctionReference;
use models::api::{admin, users};

fn accept<F: FunctionReference>(_: F, _: F::Args) {}

fn main()
{
    accept(admin::users::list, users::NumberArgs { value: 1.0 });
}
