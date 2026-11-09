//! An Axum HTTP API for an AI bookmark saver, with Convex as its database.
//!
//! Every Convex call goes through `TypedClient` and the bindings `build.rs` generates, so
//! request and response types are checked at compile time and nothing parses Convex values.

#[allow(dead_code)]
mod convex_api
{
    convex_typegen_runtime::include!();
}
mod error;
mod model;
mod routes;

use std::net::SocketAddr;
use std::path::Path;

use convex::ConvexClient;

/// Shared by every handler.
#[derive(Clone)]
pub struct AppState
{
    /// Cheap to clone: all clones share one connection to the backend.
    convex: ConvexClient,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>>
{
    // `convex dev` writes `CONVEX_URL` to `.env.local`; a variable in the environment also works.
    dotenvy::from_filename(Path::new(env!("CARGO_MANIFEST_DIR")).join(".env.local")).ok();
    let url = std::env::var("CONVEX_URL").map_err(|_| "CONVEX_URL is not set (see readme.md)")?;
    let port: u16 = std::env::var("PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(3000);

    let state = AppState {
        convex: ConvexClient::new(&url).await?,
    };
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    println!("bookmarks API on http://{addr} (Convex at {url})");
    axum::serve(listener, routes::router(state))
        .with_graceful_shutdown(async {
            tokio::signal::ctrl_c().await.ok();
        })
        .await?;
    Ok(())
}
