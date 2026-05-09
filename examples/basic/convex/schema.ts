import { defineSchema, defineTable } from "convex/server";
import { v } from "convex/values";

// https://docs.convex.dev/database/types
export default defineSchema({
    games: defineTable({
        win_count: v.number(),
        loss_count: v.number(),
    }),
    // test table that uses all convex data types and variants
    test: defineTable({
        // Basic Types
        id: v.id("test"),
        null: v.null(),
        int64: v.int64(),
        float64: v.number(),
        boolean: v.boolean(),
        string: v.string(),
        bytes: v.bytes(),

        // Arrays
        simple_array: v.array(v.string()),
        number_array: v.array(v.number()),
        mixed_array: v.array(v.any()),
        nested_array: v.array(v.array(v.string())),
        deep_nested_array: v.array(v.array(v.array(v.number()))),

        // Objects
        simple_object: v.object({
            key: v.string(),
        }),
        complex_object: v.object({
            string: v.string(),
            number: v.number(),
            boolean: v.boolean(),
            nullable: v.null(),
            optional: v.optional(v.string()),
        }),
        nested_object: v.object({
            level1: v.object({
                level2: v.object({
                    level3: v.string(),
                }),
            }),
        }),

        // Records
        string_record: v.record(v.string(), v.string()),
        number_record: v.record(v.string(), v.number()),
        complex_record: v.record(v.string(), v.any()),

        // Unions
        simple_union: v.union(v.string(), v.number()),
        complex_union: v.union(
            v.string(),
            v.number(),
            v.boolean(),
            v.null(),
            v.array(v.string())
        ),
        literal_union: v.union(
            v.literal("draft"),
            v.literal("published"),
            v.literal("archived")
        ),

        // Optionals
        optional_string: v.optional(v.string()),
        optional_object: v.optional(v.object({
            key: v.string(),
        })),
        optional_union: v.optional(v.union(v.string(), v.number())),
        optional_array: v.optional(v.array(v.string())),

        // Complex objects
        complex_nested: v.object({
            metadata: v.object({
                tags: v.array(v.string()),
                counts: v.record(v.string(), v.number()),
                status: v.union(v.literal("active"), v.literal("inactive")),
            }),
            settings: v.optional(v.object({
                preferences: v.record(v.string(), v.any()),
                limits: v.array(v.number()),
            })),
            nested_arrays: v.array(v.array(v.union(v.string(), v.number()))),
        }),
    }),
});