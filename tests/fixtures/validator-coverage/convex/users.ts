import { v } from "convex/values";
import { mutation, query } from "./_generated/server";

export const usersGetByEmail = query({
    args: {
        email: v.string(),
        includeInactive: v.optional(v.boolean()),
    },
    handler: async (ctx, { email, includeInactive }) => {
        const u = await ctx.db
            .query("users")
            .withIndex("by_email", (q) => q.eq("email", email))
            .unique();
        if (!u) return null;
        if (!includeInactive && !u.isActive) return null;
        return u;
    },
});

export const usersGetProfile = query({
    args: {
        userId: v.id("users"),
        withBytes: v.optional(v.boolean()),
    },
    handler: async (ctx, { userId, withBytes }) => {
        const doc = await ctx.db.get(userId);
        if (!doc) return null;
        if (!withBytes && doc.avatarBytes !== undefined) {
            const { avatarBytes, ...rest } = doc;
            void avatarBytes;
            return rest;
        }
        return doc;
    },
});

export const usersCreate = mutation({
    args: {
        email: v.string(),
        displayName: v.string(),
        role: v.union(v.literal("admin"), v.literal("member"), v.literal("viewer")),
        metadata: v.optional(v.record(v.string(), v.string())),
        score: v.optional(v.int64()),
        avatarBytes: v.optional(v.bytes()),
    },
    handler: async (ctx, args) => {
        const existing = await ctx.db
            .query("users")
            .withIndex("by_email", (q) => q.eq("email", args.email))
            .unique();
        if (existing) {
            throw new Error("User with this email already exists");
        }
        const id = await ctx.db.insert("users", {
            email: args.email,
            displayName: args.displayName,
            role: args.role,
            metadata: args.metadata ?? {},
            score: args.score,
            avatarBytes: args.avatarBytes,
            isActive: true,
        });
        return id;
    },
});
