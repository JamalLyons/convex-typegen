#[path = "../../target/model-tests/models.rs"]
mod models;

use convex_typegen_runtime::TypedClient;
use models::api::admin::users;

async fn wrong_kind(client: &mut TypedClient<'_>, args: users::AddArgs)
{
    let _ = client.query(users::add, args).await;
}

fn main() {}
