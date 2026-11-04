#[path = "../../target/model-tests/models.rs"]
mod models;

use convex_typegen_runtime::model::Id;
use models::tables::{Tasks, Users};

fn main()
{
    let user = Id::<Users>::from(String::from("user"));
    let _: Id<Tasks> = user;
}
