import { v } from "convex/values";
import { mutation, query } from "./_generated/server";

export const tasksSearch = query({
    args: {
        filter: v.object({
            projectId: v.id("projects"),
            minPriority: v.optional(v.union(v.literal("p0"), v.literal("p1"))),
        }),
        limit: v.optional(v.number()),
    },
    handler: async (ctx, { filter, limit }) => {
        const tasks = await ctx.db
            .query("tasks")
            .withIndex("by_project", (q) => q.eq("projectId", filter.projectId))
            .collect();
        const order = { p0: 0, p1: 1, p2: 2 } as const;
        let out = tasks.filter((t) => {
            if (filter.minPriority === undefined) return true;
            return order[t.priority] <= order[filter.minPriority];
        });
        const lim = limit ?? 50;
        out = out.slice(0, Math.max(0, Math.floor(lim)));
        return out;
    },
});

export const tasksCreate = mutation({
    args: {
        projectId: v.id("projects"),
        title: v.string(),
        priority: v.union(v.literal("p0"), v.literal("p1"), v.literal("p2")),
        assigneeUserId: v.optional(v.id("users")),
        payload: v.optional(v.any()),
        dueAt: v.optional(v.number()),
    },
    handler: async (ctx, args) => {
        return await ctx.db.insert("tasks", {
            projectId: args.projectId,
            title: args.title,
            priority: args.priority,
            assigneeUserId: args.assigneeUserId,
            payload: args.payload ?? {},
            dueAt: args.dueAt,
        });
    },
});
