import { defineSchema, defineTable } from "convex/server";
import { v } from "convex/values";

export default defineSchema({
  tasks: defineTable({ text: v.string() }),
  events: defineTable(v.union(
    v.object({ kind: v.literal("created"), id: v.id("tasks") }),
    v.object({ kind: v.literal("deleted"), reason: v.string() }),
  )),
});
