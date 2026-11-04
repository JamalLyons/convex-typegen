#[path = "../../target/model-tests/models.rs"]
mod models;

use convex_typegen_runtime::function::{FunctionReference, Query};
use models::api::admin::users;

fn accept_query(_: impl FunctionReference<Kind = Query>) {}

fn main()
{
    accept_query(users::add);
}
