import { v } from "convex/values";

export const common = { count: v.int64(), number: v.float64(), bytes: v.bytes() };
export const payload = v.object({
  ...common,
  nested: v.object({ note: v.optional(v.string()), nullable: v.nullable(v.string()) }),
  record: v.record(v.string(), v.int64()),
  choice: v.optional(v.union(v.literal("red"), v.literal("blue"), v.null())),
});
