## convex-typegen v0.4.0

This release emits **named Rust structs for nested `v.object` validators** and publishes the library vs codegen [semver policy](semver-policy.md) (target for 1.0). The `convex_typegen` crate API is unchanged; **generated `convex_types.rs` is a breaking change**.

### Highlights

- **Named nested objects** — schema columns, function args, nested objects, and array-of-object elements become `pub struct` types (`ProjectsSettings`, `IntegrationsMirrorFlags`, `TasksSearchFilter`, …) instead of `ConvexJsonValue` or a homogeneous `BTreeMap`.
- **Stable names, fail on collision** — identical shapes under the same name are reused; conflicting shapes return `InvalidSchema`.
- **Versioning docs** — [semver-policy.md](semver-policy.md) and [api-stability.md](api-stability.md) describe the library API, generated-output contract, and MSRV (still **Rust 1.95+**).
- **Oxc AST evaluation** — hybrid ESTree JSON path stays in production; spike in `src/convex/ast.rs` (see [oxc-ast-evaluation.md](oxc-ast-evaluation.md)).

---

### Breaking changes (generated code only)

`FUNCTION_PATH` strings and `*Args` struct **names** are unchanged. Field **types** on generated structs change where a `v.object` previously mapped to `ConvexJsonValue` or `BTreeMap<String, T>`.

| Convex shape | 0.3.x | 0.4.x |
| --- | --- | --- |
| `projects` table column `settings: v.object({ theme, notifyEmail })` | `ConvexJsonValue` (heterogeneous) | `ProjectsSettings` |
| `integrationsMirror` arg `flags: v.object({ verbose, trace })` | `ConvexJsonValue` | `IntegrationsMirrorFlags` |
| `tasksSearch` arg `filter: v.object({ ... })` | `ConvexJsonValue` | `TasksSearchFilter` |

Homogeneous objects without a derived name, empty objects, and nesting deeper than 8 still use `BTreeMap` / `ConvexJsonValue`.

```rust
// Before (0.3.x)
TasksSearchArgs {
    filter: serde_json::json!({ "projectId": id, "minPriority": null }),
    limit: Some(10.0),
}

// After (0.4.x)
TasksSearchArgs {
    filter: TasksSearchFilter {
        projectId: id,
        minPriority: None,
    },
    limit: Some(10.0),
}
```

Pin the **same** `convex-typegen` version in `[dependencies]` and `[build-dependencies]`.

---

### Build-only usage (smaller dependency tree)

```toml
[build-dependencies]
convex-typegen = { version = "0.4", default-features = false }
```

---

### Upgrade checklist

1. Bump `convex-typegen` to `0.4` in `[dependencies]` and `[build-dependencies]`.
2. `cargo build` to regenerate `convex_types.rs`.
3. Replace nested-object `ConvexJsonValue` / `BTreeMap` fields with the new struct types (see compiler errors).
4. Keep using `ConvexClient::prepare_args(...)?` — that path did not change in this release.

**Docs:** https://docs.rs/convex-typegen/0.4.0  
**Full changelog:** [CHANGELOG.md](https://github.com/JamalLyons/convex-typegen/blob/master/CHANGELOG.md)  
**Versioning:** [semver-policy.md](semver-policy.md), [api-stability.md](api-stability.md)
