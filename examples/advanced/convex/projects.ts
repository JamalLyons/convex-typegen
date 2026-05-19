import { v } from "convex/values";
import { mutation, query } from "./_generated/server";

export const projectsListByTeam = query({
    args: {
        teamId: v.id("teams"),
        statusFilter: v.optional(
            v.union(v.literal("draft"), v.literal("active"), v.literal("archived")),
        ),
    },
    handler: async (ctx, { teamId, statusFilter }) => {
        if (statusFilter !== undefined) {
            return await ctx.db
                .query("projects")
                .withIndex("by_team_status", (q) => q.eq("teamId", teamId).eq("status", statusFilter))
                .collect();
        }
        return await ctx.db
            .query("projects")
            .withIndex("by_team", (q) => q.eq("teamId", teamId))
            .collect();
    },
});

export const projectsUpdateTags = mutation({
    args: {
        projectId: v.id("projects"),
        tags: v.array(v.string()),
    },
    handler: async (ctx, { projectId, tags }) => {
        await ctx.db.patch(projectId, { tags });
        return await ctx.db.get(projectId);
    },
});
