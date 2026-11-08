//! A tiny notes CLI: typed args go in, typed structs come out.

#[allow(dead_code)]
mod convex_api
{
    convex_typegen_runtime::include!();
}

use std::path::Path;

use convex::ConvexClient;
use convex_api::api::notes::{self, AddArgsPriority};
use convex_api::{tables, NotesDoc};
use convex_typegen_runtime::model::Id;
use convex_typegen_runtime::TypedClient;
use futures::StreamExt;

const USAGE: &str = "usage: basic <add TEXT [low|normal|high] [TAG] | list | done ID | rm ID | summary | watch>";

fn print_notes(all: &[NotesDoc])
{
    for n in all {
        let mark = if n.done { 'x' } else { ' ' };
        let priority = n.priority.map_or("-".to_string(), |p| format!("{p:?}").to_lowercase());
        let tag = n.tag.as_deref().unwrap_or("(no tag)");
        println!("[{mark}] {}  {}  priority={priority}  tag={tag}", n._id.as_str(), n.text);
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>>
{
    // Optional `.env.local` next to Cargo.toml; `CONVEX_URL` in the environment also works.
    dotenvy::from_filename(Path::new(env!("CARGO_MANIFEST_DIR")).join(".env.local")).ok();
    let args: Vec<String> = std::env::args().skip(1).collect();
    let arg = |i: usize| args.get(i).cloned().ok_or(USAGE);
    let id = |i: usize| arg(i).map(Id::<tables::Notes>::from);

    let mut sdk = ConvexClient::new(&std::env::var("CONVEX_URL")?).await?;
    let mut client = TypedClient::new(&mut sdk);

    match args.first().map(String::as_str) {
        Some("add") => {
            let priority = match args.get(2).map(String::as_str) {
                None => None,
                Some("low") => Some(AddArgsPriority::Low),
                Some("normal") => Some(AddArgsPriority::Normal),
                Some("high") => Some(AddArgsPriority::High),
                Some(_) => return Err(USAGE.into()),
            };
            let tag = args.get(3).cloned();
            let new_id = client
                .mutation(
                    notes::add,
                    notes::AddArgs {
                        text: arg(1)?,
                        priority,
                        tag,
                    },
                )
                .await?;
            println!("added {}", new_id.as_str());
        }
        Some("list") => print_notes(&client.query(notes::list, notes::ListArgs {}).await?),
        Some("done") => client.mutation(notes::done, notes::DoneArgs { id: id(1)? }).await?,
        Some("rm") => client.mutation(notes::remove, notes::RemoveArgs { id: id(1)? }).await?,
        Some("summary") => println!("{}", client.action(notes::summary, notes::SummaryArgs {}).await?),
        Some("watch") => {
            // Prints the current list, then again whenever any note changes.
            let mut updates = client.subscribe(notes::list, notes::ListArgs {}).await?;
            while let Some(all) = updates.next().await {
                println!("--- {} notes ---", all.as_ref().map_or(0, Vec::len));
                print_notes(&all?);
            }
        }
        _ => return Err(USAGE.into()),
    }
    Ok(())
}
