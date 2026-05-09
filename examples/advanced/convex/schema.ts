import { defineSchema, defineTable } from "convex/server";
import { v } from "convex/values";

/**
 * Rich schema for exercising `convex-typegen` validators and table shapes, with real Convex
 * indexes used by queries in this example (`withIndex`).
 */
export default defineSchema({
    users: defineTable({
        email: v.string(),
        displayName: v.string(),
        role: v.union(v.literal("admin"), v.literal("member"), v.literal("viewer")),
        metadata: v.record(v.string(), v.string()),
        avatarBytes: v.optional(v.bytes()),
        externalId: v.optional(v.string()),
        score: v.optional(v.int64()),
        isActive: v.boolean(),
    }).index("by_email", ["email"]),

    teams: defineTable({
        name: v.string(),
        slug: v.string(),
        ownerUserId: v.id("users"),
        createdAt: v.number(),
    })
        .index("by_owner", ["ownerUserId"])
        .index("by_slug", ["slug"]),

    projects: defineTable({
        teamId: v.id("teams"),
        title: v.string(),
        status: v.union(v.literal("draft"), v.literal("active"), v.literal("archived")),
        tags: v.array(v.string()),
        settings: v.object({
            theme: v.string(),
            notifyEmail: v.boolean(),
        }),
        budget: v.optional(v.number()),
    })
        .index("by_team", ["teamId"])
        .index("by_team_status", ["teamId", "status"]),

    tasks: defineTable({
        projectId: v.id("projects"),
        title: v.string(),
        priority: v.union(v.literal("p0"), v.literal("p1"), v.literal("p2")),
        assigneeUserId: v.optional(v.id("users")),
        payload: v.any(),
        dueAt: v.optional(v.number()),
    }).index("by_project", ["projectId"]),
});
