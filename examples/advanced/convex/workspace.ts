import { mutation, query } from "./_generated/server";

/** One-shot demo data when the deployment is empty. */
export const workspaceSeedIfEmpty = mutation({
    handler: async (ctx) => {
        const existing = await ctx.db.query("users").first();
        if (existing) {
            return { seeded: false as const, reason: "users table not empty" };
        }

        const userId = await ctx.db.insert("users", {
            email: "demo@example.com",
            displayName: "Demo User",
            role: "admin",
            metadata: { source: "advanced-example" },
            isActive: true,
            score: 100n,
        });

        const teamId = await ctx.db.insert("teams", {
            name: "Demo Team",
            slug: "demo-team",
            ownerUserId: userId,
            createdAt: Date.now(),
        });

        const projectId = await ctx.db.insert("projects", {
            teamId,
            title: "Demo Project",
            status: "active",
            tags: ["rust", "convex"],
            settings: { theme: "dark", notifyEmail: true },
            budget: 5000,
        });

        const taskId = await ctx.db.insert("tasks", {
            projectId,
            title: "Exercise generated types",
            priority: "p0",
            payload: { hello: "world", n: 42 },
        });

        return { seeded: true as const, userId, teamId, projectId, taskId };
    },
});

export const workspaceSummary = query({
    handler: async (ctx) => {
        const users = await ctx.db.query("users").collect();
        const teams = await ctx.db.query("teams").collect();
        const projects = await ctx.db.query("projects").collect();
        const tasks = await ctx.db.query("tasks").collect();

        return {
            userCount: users.length,
            teamCount: teams.length,
            projectCount: projects.length,
            taskCount: tasks.length,
            firstUserId: users[0]?._id ?? null,
            firstTeamId: teams[0]?._id ?? null,
            firstProjectId: projects[0]?._id ?? null,
        };
    },
});
