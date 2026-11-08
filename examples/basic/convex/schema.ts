import { defineSchema, defineTable } from "convex/server";
import { v } from "convex/values";

// A string-literal union: Rust gets an enum with `Low`, `Normal`, and `High` variants.
// Nullable: the key is always present, and `null` means "no priority". Rust sees `Option<Enum>`.
export const priority = v.nullable(v.union(v.literal("low"), v.literal("normal"), v.literal("high")));

// The fields of a stored note, shared by the table and the `returns` validators.
export const noteFields = {
  text: v.string(),
  priority,
  // Optional: the key may be missing (different from `null`). Rust sees `Option<String>`.
  tag: v.optional(v.string()),
  done: v.boolean(),
};

export default defineSchema({
  notes: defineTable(noteFields).index("by_done", ["done"]),
});
