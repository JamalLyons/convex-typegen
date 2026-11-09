import { defineSchema, defineTable } from "convex/server";
import { v } from "convex/values";

// String-literal unions: Rust gets an enum for each.
export const status = v.union(v.literal("pending"), v.literal("ready"), v.literal("failed"));
// Which path wrote the summary: an LLM, or the deterministic fallback.
export const summarizer = v.union(v.literal("llm"), v.literal("fallback"));

// The fields of a stored bookmark, shared by the table and the `returns` validators.
export const bookmarkFields = {
  url: v.string(),
  status,
  // Nullable: always present, `null` until the page has been processed.
  title: v.nullable(v.string()),
  summary: v.nullable(v.string()),
  tags: v.array(v.string()),
  // Optional: missing until the bookmark is ready.
  summarizer: v.optional(summarizer),
  // Optional: only set when processing failed.
  error: v.optional(v.string()),
};

export default defineSchema({
  bookmarks: defineTable(bookmarkFields).index("by_status", ["status"]),
  // Convex can't index the elements of an array, so each tag gets its own row.
  bookmarkTags: defineTable({ bookmarkId: v.id("bookmarks"), tag: v.string() })
    .index("by_tag", ["tag"])
    .index("by_bookmark", ["bookmarkId"]),
});
