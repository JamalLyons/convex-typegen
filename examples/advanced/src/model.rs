//! JSON shapes of the HTTP API, converted from and to the generated Convex types.
//!
//! The generated types derive `ConvexValue`, not serde, so the HTTP layer keeps its own
//! types. The conversions are plain matches on generated enums and structs.

use convex_typegen_runtime::ConvexValue;
use serde::{Deserialize, Serialize};

use crate::convex_api::api::bookmarks::ListArgsStatus;
use crate::convex_api::{BookmarksDoc, BookmarksDocStatus, BookmarksDocSummarizer};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Status
{
    Pending,
    Ready,
    Failed,
}

#[derive(Debug, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Summarizer
{
    Llm,
    Fallback,
}

#[derive(Debug, PartialEq, Serialize)]
pub struct Bookmark
{
    pub id: String,
    pub url: String,
    pub status: Status,
    /// `null` until the bookmark is ready.
    pub title: Option<String>,
    pub summary: Option<String>,
    pub tags: Vec<String>,
    /// Whether an LLM or the offline fallback wrote the summary. Left out until ready.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summarizer: Option<Summarizer>,
    /// Left out unless processing failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Milliseconds since the Unix epoch.
    pub created_at: f64,
}

#[derive(Debug, Deserialize)]
pub struct CreateBookmark
{
    pub url: String,
}

/// `GET /bookmarks?status=ready&tag=rust` (both optional).
#[derive(Debug, Deserialize)]
pub struct ListFilter
{
    pub status: Option<Status>,
    pub tag: Option<String>,
}

/// The `ConvexError` payload `bookmarks:create` throws. Validators don't describe thrown
/// errors, so this one is handwritten and attached with `with_error`.
#[derive(Debug, ConvexValue)]
pub struct CreateFailure
{
    pub code: CreateFailureCode,
}

#[derive(Debug, ConvexValue)]
pub enum CreateFailureCode
{
    #[convex(literal = "invalid_url")]
    InvalidUrl,
}

impl From<BookmarksDoc> for Bookmark
{
    fn from(doc: BookmarksDoc) -> Self
    {
        Self {
            id: doc._id.into_string(),
            url: doc.url,
            status: match doc.status {
                BookmarksDocStatus::Pending => Status::Pending,
                BookmarksDocStatus::Ready => Status::Ready,
                BookmarksDocStatus::Failed => Status::Failed,
            },
            // `v.nullable(v.string())` generates `Option<String>`.
            title: doc.title,
            summary: doc.summary,
            tags: doc.tags,
            summarizer: doc.summarizer.map(|s| match s {
                BookmarksDocSummarizer::Llm => Summarizer::Llm,
                BookmarksDocSummarizer::Fallback => Summarizer::Fallback,
            }),
            error: doc.error,
            created_at: doc._creation_time,
        }
    }
}

impl From<Status> for ListArgsStatus
{
    fn from(status: Status) -> Self
    {
        match status {
            Status::Pending => Self::Pending,
            Status::Ready => Self::Ready,
            Status::Failed => Self::Failed,
        }
    }
}
