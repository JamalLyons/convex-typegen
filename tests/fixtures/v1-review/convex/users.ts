import { query as publicQuery } from "./_generated/server";
import { v } from "convex/values";

export const list = publicQuery({ handler: () => ["root"] });
export const optional = publicQuery({
  args: { root: v.optional(v.string()), nested: v.object({ note: v.optional(v.string()) }) },
  returns: v.boolean(),
  handler: (_, { nested }) => Object.hasOwn(nested, "note"),
});
export const number = publicQuery({
  args: { value: v.number() }, returns: v.number(), handler: (_, { value }) => value,
});
