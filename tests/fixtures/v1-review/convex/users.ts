import { query } from "./_generated/server";
import { v } from "convex/values";

export const list = query({ handler: () => ["root"] });
export const optional = query({
  args: { root: v.optional(v.string()), nested: v.object({ note: v.optional(v.string()) }) },
  returns: v.boolean(),
  handler: (_, { nested }) => Object.hasOwn(nested, "note"),
});
export const number = query({
  args: { value: v.number() }, returns: v.number(), handler: (_, { value }) => value,
});
