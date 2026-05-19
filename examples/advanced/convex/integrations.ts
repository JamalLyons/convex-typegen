import { v } from "convex/values";
import { action } from "./_generated/server";

/**
 * Public action: exercises `action`, heterogeneous object args (maps to
 * `ConvexJsonValue` in Rust), arrays, literals, and optional omission.
 */
export const integrationsMirror = action({
    args: {
        body: v.string(),
        numbers: v.array(v.number()),
        flags: v.object({
            verbose: v.boolean(),
            trace: v.boolean(),
        }),
        mode: v.union(v.literal("json"), v.literal("text")),
        extra: v.optional(v.record(v.string(), v.string())),
    },
    handler: async (_ctx, args) => {
        return {
            echo: args.body,
            sum: args.numbers.reduce((a, b) => a + b, 0),
            flags: args.flags,
            mode: args.mode,
            keys: args.extra ? Object.keys(args.extra) : [],
        };
    },
});
