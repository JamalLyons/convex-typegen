import { defineSchema, defineTable } from "convex/server";
import { v } from "convex/values";

// https://docs.convex.dev/database/types
export default defineSchema({
    games: defineTable({
        win_count: v.number(),
        loss_count: v.number(),
    }),
});