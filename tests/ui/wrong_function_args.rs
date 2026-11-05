#[path = "../../target/model-tests/models.rs"]
mod models;

use convex_typegen_runtime::TypedClient;
use models::api::{admin, users};

async fn wrong_args(client: &mut TypedClient<'_>)
{
    let _ = client.query(admin::users::list, users::NumberArgs { value: 1.0 }).await;
}

fn main() {}
