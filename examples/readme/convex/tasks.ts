import { action, mutation, query } from "./_generated/server";
import { ConvexError, v } from "convex/values";
import { priority } from "./schema";

export const list = query({
  args: {},
  returns: v.array(v.string()),
  handler: async (ctx) =>
    (await ctx.db.query("tasks").collect()).map((task) => task.text),
});

export const add = mutation({
  args: { text: v.string(), priority: v.optional(priority) },
  returns: v.id("tasks"),
  handler: async (ctx, args) => {
    if (!args.text.trim()) {
      throw new ConvexError({ code: "empty_text" });
    }
    return await ctx.db.insert("tasks", args);
  },
});

export const ping = action({
  args: {},
  returns: v.string(),
  handler: async () => "pong",
});
