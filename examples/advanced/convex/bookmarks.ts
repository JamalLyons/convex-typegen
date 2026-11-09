import { internalMutation, mutation, query, type MutationCtx } from "./_generated/server";
import type { Doc, Id } from "./_generated/dataModel";
import { internal } from "./_generated/api";
import { ConvexError, v } from "convex/values";
import { bookmarkFields, status, summarizer } from "./schema";

// A stored bookmark as clients receive it. convex-typegen maps this shape to `BookmarksDoc`.
const bookmark = v.object({
  _id: v.id("bookmarks"),
  _creationTime: v.number(),
  ...bookmarkFields,
});

/** Save a URL as `pending` and schedule `summarize:run` to fill it in. */
export const create = mutation({
  args: { url: v.string() },
  returns: v.id("bookmarks"),
  handler: async (ctx, { url }) => {
    let parsed: URL;
    try {
      parsed = new URL(url);
    } catch {
      throw new ConvexError({ code: "invalid_url" });
    }
    if (parsed.protocol !== "http:" && parsed.protocol !== "https:") {
      throw new ConvexError({ code: "invalid_url" });
    }
    const id = await ctx.db.insert("bookmarks", {
      url: parsed.href,
      status: "pending",
      title: null,
      summary: null,
      tags: [],
    });
    await ctx.scheduler.runAfter(0, internal.summarize.run, { id, url: parsed.href });
    return id;
  },
});

/** Newest first, optionally filtered by status or tag through an index. */
export const list = query({
  args: { status: v.optional(status), tag: v.optional(v.string()) },
  returns: v.array(bookmark),
  handler: async (ctx, args) => {
    if (args.tag !== undefined) {
      const tag = args.tag;
      const rows = await ctx.db
        .query("bookmarkTags")
        .withIndex("by_tag", (q) => q.eq("tag", tag))
        .order("desc")
        .collect();
      const found = await Promise.all(rows.map((row) => ctx.db.get(row.bookmarkId)));
      return found.filter(
        (b): b is Doc<"bookmarks"> => b !== null && (args.status === undefined || b.status === args.status),
      );
    }
    if (args.status !== undefined) {
      const wanted = args.status;
      return await ctx.db
        .query("bookmarks")
        .withIndex("by_status", (q) => q.eq("status", wanted))
        .order("desc")
        .collect();
    }
    return await ctx.db.query("bookmarks").order("desc").collect();
  },
});

/** One bookmark, or `null` when the id no longer exists. */
export const get = query({
  args: { id: v.id("bookmarks") },
  returns: v.nullable(bookmark),
  handler: async (ctx, { id }) => await ctx.db.get(id),
});

/** Delete a bookmark and its tag rows. Returns `false` when it was already gone. */
export const remove = mutation({
  args: { id: v.id("bookmarks") },
  returns: v.boolean(),
  handler: async (ctx, { id }) => {
    if ((await ctx.db.get(id)) === null) {
      return false;
    }
    await clearTags(ctx, id);
    await ctx.db.delete(id);
    return true;
  },
});

/** Called by `summarize:run` when the page was summarized. */
export const markReady = internalMutation({
  args: {
    id: v.id("bookmarks"),
    title: v.string(),
    summary: v.string(),
    tags: v.array(v.string()),
    summarizer,
  },
  returns: v.null(),
  handler: async (ctx, { id, ...result }) => {
    // The bookmark may have been deleted while the action ran.
    if ((await ctx.db.get(id)) === null) {
      return null;
    }
    await ctx.db.patch(id, { ...result, status: "ready" });
    await clearTags(ctx, id);
    for (const tag of result.tags) {
      await ctx.db.insert("bookmarkTags", { bookmarkId: id, tag });
    }
    return null;
  },
});

/** Called by `summarize:run` when fetching or summarizing failed. */
export const markFailed = internalMutation({
  args: { id: v.id("bookmarks"), error: v.string() },
  returns: v.null(),
  handler: async (ctx, { id, error }) => {
    if ((await ctx.db.get(id)) !== null) {
      await ctx.db.patch(id, { status: "failed", error });
    }
    return null;
  },
});

async function clearTags(ctx: MutationCtx, id: Id<"bookmarks">) {
  const rows = await ctx.db
    .query("bookmarkTags")
    .withIndex("by_bookmark", (q) => q.eq("bookmarkId", id))
    .collect();
  for (const row of rows) {
    await ctx.db.delete(row._id);
  }
}
