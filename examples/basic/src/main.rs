//! A tiny notes CLI: typed args go in, typed structs come out.

#[allow(dead_code)]
mod convex_api
{
    convex_typegen_runtime::include!();
}

use std::path::Path;
use std::process::ExitCode;

use convex::ConvexClient;
use convex_api::api::notes::{self, AddArgsPriority};
use convex_api::{tables, NotesDoc, NotesDocPriority};
use convex_typegen_runtime::model::Id;
use convex_typegen_runtime::TypedClient;
use futures::StreamExt;

const USAGE: &str = "usage: basic <add TEXT [low|normal|high] [TAG] | list | done ID | rm ID | summary | watch>";

fn print_notes(all: &[NotesDoc])
{
    for n in all {
        let mark = if n.done { 'x' } else { ' ' };
        let priority = match n.priority {
            Some(NotesDocPriority::Low) => "low",
            Some(NotesDocPriority::Normal) => "normal",
            Some(NotesDocPriority::High) => "high",
            None => "-",
        };
        let tag = n.tag.as_deref().unwrap_or("(no tag)");
        println!("[{mark}] {}  {}  priority={priority}  tag={tag}", n._id.as_str(), n.text);
    }
}

#[tokio::main]
async fn main() -> ExitCode
{
    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

async fn run() -> Result<(), Box<dyn std::error::Error>>
{
    // Optional `.env.local` next to Cargo.toml; `CONVEX_URL` in the environment also works.
    dotenvy::from_filename(Path::new(env!("CARGO_MANIFEST_DIR")).join(".env.local")).ok();
    let args: Vec<String> = std::env::args().skip(1).collect();
    let arg = |i: usize| args.get(i).cloned().ok_or(USAGE);
    let id = |i: usize| arg(i).map(Id::<tables::Notes>::from);
    // Check usage before connecting so a typo never needs a backend.
    if !matches!(
        args.first().map(String::as_str),
        Some("add" | "list" | "done" | "rm" | "summary" | "watch")
    ) {
        return Err(USAGE.into());
    }

    let url = std::env::var("CONVEX_URL").map_err(|_| "CONVEX_URL is not set (see readme.md)")?;
    let mut sdk = ConvexClient::new(&url).await?;
    let mut client = TypedClient::new(&mut sdk);

    match args[0].as_str() {
        "add" => {
            let priority = match args.get(2).map(String::as_str) {
                None => None,
                Some("low") => Some(AddArgsPriority::Low),
                Some("normal") => Some(AddArgsPriority::Normal),
                Some("high") => Some(AddArgsPriority::High),
                Some(_) => return Err(USAGE.into()),
            };
            let new_id = client
                .mutation(
                    notes::add,
                    notes::AddArgs {
                        text: arg(1)?,
                        priority,
                        tag: args.get(3).cloned(),
                    },
                )
                .await?;
            println!("added {}", new_id.as_str());
        }
        "list" => print_notes(&client.query(notes::list, notes::ListArgs {}).await?),
        "done" => client.mutation(notes::done, notes::DoneArgs { id: id(1)? }).await?,
        "rm" => client.mutation(notes::remove, notes::RemoveArgs { id: id(1)? }).await?,
        "summary" => println!("{}", client.action(notes::summary, notes::SummaryArgs {}).await?),
        _ => {
            // watch: prints the current list, then again whenever any note changes.
            let mut updates = client.subscribe(notes::list, notes::ListArgs {}).await?;
            while let Some(all) = updates.next().await {
                let all = all?;
                println!("--- notes ({}) ---", all.len());
                print_notes(&all);
            }
        }
    }
    Ok(())
}
