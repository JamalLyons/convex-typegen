## convex-typegen v0.3.0

This release focuses on **correct multi-module codegen**, **safer argument serialization**, and **production hygiene** (MSRV, supply-chain CI, optional client deps).

### Highlights

- **Unique args struct names** — generated types use `{Module}{Export}Args` so two modules can both export `list` without producing duplicate Rust definitions.
- **Fallible args conversion** — `*Args` implements `TryFrom` into `BTreeMap<String, ConvexJsonValue>`; `ConvexClientExt::prepare_args` returns `Result<_, serde_json::Error>`.
- **Convex-correct optional args** — top-level `v.optional(...)` fields omit the key when `None` (not JSON `null`).
- **`client` feature (default on)** — runtime helpers stay behind the official `convex` crate; use `default-features = false` in `build-dependencies` for a lighter codegen-only path.
- **MSRV:** Rust **1.78+**

---

### Breaking changes

#### 1. Renamed generated args structs

`FUNCTION_PATH` strings are unchanged; only Rust type names change.

| Module file | Export   | 0.2.x        | 0.3.x              |
|-------------|----------|--------------|--------------------|
| `games.ts`  | `getGame` | `GetGameArgs` | `GamesGetGameArgs` |
| `mod_a.ts`  | `list`    | `ListArgs`    | `ModAListArgs`     |
| `tasks.ts`  | `tasksSearch` | `TasksSearchArgs` | `TasksSearchArgs` (unchanged when export already includes module prefix) |

```rust
// Before (0.2.x)
use convex_types::GetGameArgs;
client.query(
    GetGameArgs::FUNCTION_PATH,
    ConvexClient::prepare_args(GetGameArgs { logData: None })?,
)?;

// After (0.3.x)
use convex_types::GamesGetGameArgs;
client.query(
    GamesGetGameArgs::FUNCTION_PATH,
    ConvexClient::prepare_args(GamesGetGameArgs { logData: None })?,
)?;
```

#### 2. Fallible `prepare_args` and `TryFrom` for args

```rust
// prepare_args now returns Result<_, serde_json::Error>
let args = ConvexClient::prepare_args(my_args)?;
```

#### 3. Prelude renames

- `JsonValue` → `ConvexJsonValue`
- `JsonError` → `ConvexJsonError`

---

### Notable fixes

- Chained `defineTable().index()` / `.searchIndex()` / `.vectorIndex()` supported when reading column validators.
- Heterogeneous `v.object({ ... })` fields map to `ConvexJsonValue` instead of an incorrect homogeneous `BTreeMap<String, T>`.
- Oxc `Property` nodes accepted in function `args` objects (in addition to `ObjectProperty`).
- Duplicate qualified args struct names fail at codegen with a clear `InvalidSchema` error.
- Parser diagnostics included in `ParsingFailed::details` (enable `verbose` feature for stderr echo).

---

### Build-only usage (smaller dependency tree)

```toml
[build-dependencies]
convex-typegen = { version = "0.3", default-features = false }
```

---

### Upgrade checklist

1. Bump `convex-typegen` to `0.3` in `[dependencies]` and `[build-dependencies]`.
2. `cargo build` to regenerate `convex_types.rs`.
3. Update imports for renamed `*Args` structs and prelude aliases.
4. Handle `prepare_args` / `TryFrom` as `Result` where needed.

**Docs:** https://docs.rs/convex-typegen/0.3.0  
**Full changelog:** [CHANGELOG.md](https://github.com/JamalLyons/convex-typegen/blob/main/CHANGELOG.md)
