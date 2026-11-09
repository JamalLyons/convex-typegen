//! HTTP routes. Each handler makes one or two typed Convex calls.

use std::convert::Infallible;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::routing::get;
use axum::{Json, Router};
use convex_typegen_runtime::model::Id;
use convex_typegen_runtime::TypedClient;
use futures::{Stream, StreamExt};

use crate::convex_api::api::bookmarks;
use crate::convex_api::tables;
use crate::error::ApiError;
use crate::model::{Bookmark, CreateBookmark, CreateFailure, ListFilter};
use crate::AppState;

type BookmarkId = Id<tables::Bookmarks>;

pub fn router(state: AppState) -> Router
{
    Router::new()
        .route("/bookmarks", get(list).post(create))
        .route("/bookmarks/stream", get(stream))
        .route("/bookmarks/{id}", get(show).delete(remove))
        .with_state(state)
}

/// `POST /bookmarks {"url": "..."}`: stores a `pending` bookmark; Convex summarizes it in the background.
async fn create(
    State(mut state): State<AppState>,
    Json(body): Json<CreateBookmark>,
) -> Result<(StatusCode, Json<Bookmark>), ApiError>
{
    let mut client = TypedClient::new(&mut state.convex);
    let id = client
        .mutation(
            bookmarks::create.with_error::<CreateFailure>(),
            bookmarks::CreateArgs { url: body.url },
        )
        .await?;
    let bookmark = find(&mut client, id).await?.ok_or_else(ApiError::not_found)?;
    Ok((StatusCode::CREATED, Json(bookmark)))
}

/// `GET /bookmarks?status=&tag=`
async fn list(State(mut state): State<AppState>, Query(filter): Query<ListFilter>) -> Result<Json<Vec<Bookmark>>, ApiError>
{
    let docs = TypedClient::new(&mut state.convex)
        .query(bookmarks::list, list_args(filter))
        .await?;
    Ok(Json(docs.into_iter().map(Bookmark::from).collect()))
}

/// `GET /bookmarks/{id}`
async fn show(State(mut state): State<AppState>, Path(id): Path<String>) -> Result<Json<Bookmark>, ApiError>
{
    let mut client = TypedClient::new(&mut state.convex);
    let bookmark = find(&mut client, BookmarkId::from(id)).await?;
    bookmark.map(Json).ok_or_else(ApiError::not_found)
}

/// `DELETE /bookmarks/{id}`
async fn remove(State(mut state): State<AppState>, Path(id): Path<String>) -> Result<StatusCode, ApiError>
{
    let args = bookmarks::RemoveArgs {
        id: BookmarkId::from(id),
    };
    let removed = TypedClient::new(&mut state.convex).mutation(bookmarks::remove, args).await?;
    if removed {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::not_found())
    }
}

/// `GET /bookmarks/stream?status=&tag=`: server-sent events. Sends the full list now and
/// again whenever it changes, driven by a Convex subscription (no polling).
async fn stream(
    State(mut state): State<AppState>,
    Query(filter): Query<ListFilter>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, ApiError>
{
    let updates = TypedClient::new(&mut state.convex)
        .subscribe(bookmarks::list, list_args(filter))
        .await?;
    // Convex can push an update whose result is unchanged (e.g. after an unrelated write in the
    // same table), so skip repeats of the list the client already has.
    let mut last = None;
    let events = updates.filter_map(move |update| {
        let event = match update.map(|docs| docs.into_iter().map(Bookmark::from).collect::<Vec<_>>()) {
            Ok(all) if last.as_ref() == Some(&all) => None,
            Ok(all) => {
                let event = Event::default().event("bookmarks").json_data(&all);
                last = Some(all);
                Some(event.unwrap_or_else(|_| Event::default().event("error").data("could not encode update")))
            }
            Err(error) => {
                eprintln!("convex subscription: {error}");
                Some(Event::default().event("error").data("subscription update failed"))
            }
        };
        std::future::ready(event.map(Ok))
    });
    Ok(Sse::new(events).keep_alive(KeepAlive::default()))
}

async fn find(client: &mut TypedClient<'_>, id: BookmarkId) -> Result<Option<Bookmark>, ApiError>
{
    let doc = client.query(bookmarks::get, bookmarks::GetArgs { id }).await?;
    Ok(doc.map(Bookmark::from))
}

fn list_args(filter: ListFilter) -> bookmarks::ListArgs
{
    bookmarks::ListArgs {
        status: filter.status.map(Into::into),
        tag: filter.tag,
    }
}
