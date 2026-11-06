#[allow(dead_code)]
mod convex_api
{
    convex_typegen_runtime::include!();
}

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use convex::ConvexClient;
use convex_api::api::tasks;
use convex_api::tables;
use convex_typegen_runtime::model::Id;
use convex_typegen_runtime::{CallError, ConvexValue, TypedClient};
use futures::StreamExt;
use tokio::time::timeout;

#[derive(Debug, ConvexValue)]
struct AddFailure
{
    code: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>>
{
    let mut sdk = ConvexClient::new(&std::env::var("CONVEX_URL")?).await?;
    let mut client = TypedClient::new(&mut sdk);

    let before: Vec<String> = client.query(tasks::list, tasks::ListArgs {}).await?;
    println!("Before: {before:?}");

    let reply: String = client.action(tasks::ping, tasks::PingArgs {}).await?;
    println!("Action: {reply}");
    assert_eq!(reply, "pong");

    let add = tasks::add.with_error::<AddFailure>();
    let invalid = tasks::AddArgs {
        text: String::new(),
        priority: None,
    };
    match client.mutation(add, invalid).await {
        Err(CallError::Application { data: Ok(failure), .. }) => {
            assert_eq!(failure.code, "empty_text");
            println!("Rejected: {}", failure.code);
        }
        Err(other) => return Err(other.into()),
        Ok(_) => return Err("the backend accepted empty text".into()),
    }

    // ANCHOR: subscribe
    let mut updates = client.subscribe(tasks::list, tasks::ListArgs {}).await?;
    let text = format!("Write Rust ({})", SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos());
    let id: Id<tables::Tasks> = client
        .mutation(
            add,
            tasks::AddArgs {
                text: text.clone(),
                priority: Some(Some(tasks::AddArgsPriority::High)),
            },
        )
        .await?;
    println!("Created: {}", id.as_str());

    // The stream can first yield the initial result. Wait for this write to appear.
    timeout(Duration::from_secs(10), async {
        while let Some(update) = updates.next().await {
            if update?.contains(&text) {
                println!("Observed: {text}");
                return Ok::<(), Box<dyn std::error::Error>>(());
            }
        }
        Err("subscription ended before observing the write".into())
    })
    .await??;
    // ANCHOR_END: subscribe

    Ok(())
}
