import { v } from "convex/values";
import { mutation, query } from "./_generated/server";

export const teamsListByOwner = query({
    args: {
        ownerUserId: v.id("users"),
    },
    handler: async (ctx, { ownerUserId }) => {
        return await ctx.db
            .query("teams")
            .withIndex("by_owner", (q) => q.eq("ownerUserId", ownerUserId))
            .collect();
    },
});

export const teamsCreate = mutation({
    args: {
        name: v.string(),
        slug: v.string(),
        ownerUserId: v.id("users"),
        createdAt: v.optional(v.number()),
    },
    handler: async (ctx, { name, slug, ownerUserId, createdAt }) => {
        const slugTaken = await ctx.db
            .query("teams")
            .withIndex("by_slug", (q) => q.eq("slug", slug))
            .unique();
        if (slugTaken) {
            throw new Error("Team slug already in use");
        }
        const id = await ctx.db.insert("teams", {
            name,
            slug,
            ownerUserId,
            createdAt: createdAt ?? Date.now(),
        });
        return id;
    },
});
