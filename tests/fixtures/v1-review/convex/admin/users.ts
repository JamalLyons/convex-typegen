import { ConvexError, v } from "convex/values";
import { internalQuery, internalMutation, internalAction, query, mutation, action } from "../_generated/server";
import { payload } from "../validators";

export const list = query({ args: {}, returns: v.array(v.string()), handler: () => ["nested"] });
export const echo = query({ args: { data: payload }, returns: payload, handler: (_, { data }) => data });
export const add = mutation({
  args: { text: v.string() },
  returns: v.id("tasks"),
  handler: (ctx, { text }) => ctx.db.insert("tasks", { text }),
});
export const docs = query({
  args: {},
  returns: v.array(v.object({ _id: v.id("tasks"), _creationTime: v.number(), text: v.string() })),
  handler: (ctx) => ctx.db.query("tasks").collect(),
});
export const fail = action({
  args: {}, returns: v.null(),
  handler: () => { throw new ConvexError({ code: "denied", retry: false }); },
});
export const hiddenQuery = internalQuery({ args: {}, returns: v.null(), handler: () => null });
export const hiddenMutation = internalMutation({ args: {}, returns: v.null(), handler: () => null });
export const hiddenAction = internalAction({ args: {}, returns: v.null(), handler: () => null });
export const ping = action({ args: {}, returns: v.literal("pong"), handler: () => "pong" as const });
