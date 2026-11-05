#[allow(dead_code)]
mod convex_api
{
    convex_typegen_runtime::include!();
}

use convex::ConvexClient;
use convex_api::api::tasks;
use convex_typegen_runtime::TypedClient;
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>>
{
    let mut sdk = ConvexClient::new(&std::env::var("CONVEX_URL")?).await?;
    let mut client = TypedClient::new(&mut sdk);
    let tasks: Vec<String> = client.query(tasks::list, tasks::ListArgs {}).await?;
    println!("{tasks:?}");

    let mut updates = client.subscribe(tasks::list, tasks::ListArgs {}).await?;
    while let Some(tasks) = updates.next().await {
        println!("{:?}", tasks?);
    }
    Ok(())
}
