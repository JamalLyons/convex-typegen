import { defineSchema, defineTable } from "convex/server";
import { v } from "convex/values";

export const priority = v.nullable(v.union(v.literal("low"), v.literal("high")));

export default defineSchema({
  tasks: defineTable({ text: v.string(), priority: v.optional(priority) }),
});
