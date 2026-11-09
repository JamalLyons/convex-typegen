import { action, mutation, query } from "./_generated/server";
import { api } from "./_generated/api";
import { v } from "convex/values";
import { noteFields, priority } from "./schema";

// A stored note as clients receive it: the table fields plus Convex's system fields.
const note = v.object({
  _id: v.id("notes"),
  _creationTime: v.number(),
  ...noteFields,
});

export const list = query({
  args: {},
  returns: v.array(note),
  handler: async (ctx) => await ctx.db.query("notes").order("desc").collect(),
});

export const add = mutation({
  args: { text: v.string(), priority, tag: v.optional(v.string()) },
  returns: v.id("notes"),
  handler: async (ctx, args) => await ctx.db.insert("notes", { ...args, done: false }),
});

export const done = mutation({
  args: { id: v.id("notes") },
  returns: v.null(),
  handler: async (ctx, args) => {
    await ctx.db.patch(args.id, { done: true });
    return null;
  },
});

export const remove = mutation({
  args: { id: v.id("notes") },
  returns: v.null(),
  handler: async (ctx, args) => {
    await ctx.db.delete(args.id);
    return null;
  },
});

export const summary = action({
  args: {},
  returns: v.string(),
  // The explicit return type breaks the `api` type cycle that `runQuery` would otherwise create.
  handler: async (ctx): Promise<string> => {
    const notes = await ctx.runQuery(api.notes.list, {});
    const open = notes.filter((n) => !n.done).length;
    return `${notes.length} notes, ${open} open`;
  },
});
