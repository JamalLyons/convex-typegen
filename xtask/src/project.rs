use std::fmt::Write as _;
use std::path::Path;
use std::{fs, io};

pub const TIERS: [(&str, usize); 4] = [("small", 12), ("medium", 100), ("large", 1_000), ("huge", 10_240)];

const AREAS: &[&str] = &["accounts", "billing", "catalog", "inventory", "projects", "support"];
const GROUPS: &[&str] = &["audit", "jobs", "reports", "workflows"];
const OPERATIONS: &[(&str, &str)] = &[
    ("query", "read"),
    ("mutation", "save"),
    ("action", "sync"),
    ("internalQuery", "inspect"),
    ("internalMutation", "update"),
    ("internalAction", "dispatch"),
];

// Each entry appears in the schema. Endpoints and additional tables sample this same catalog.
// These validators are legal as object fields, including the optional entries.
const VALIDATORS: &[(&str, &str)] = &[
    ("title", "v.string()"),
    ("quantity", "v.number()"),
    ("score", "v.float64()"),
    ("active", "v.boolean()"),
    ("cleared", "v.null()"),
    ("sequence", "v.int64()"),
    ("revision", "v.bigint()"),
    ("committed", "v.commitTs()"),
    ("attachment", "v.bytes()"),
    ("raw", "v.any()"),
    ("owner", r#"v.id("accounts")"#),
    ("state", r#"v.literal("queued")"#),
    ("ratio", "v.literal(-1.25)"),
    ("offset", "v.literal(-42n)"),
    ("confirmed", "v.literal(true)"),
    ("note", "v.optional(v.string())"),
    ("alias", "v.string().optional()"),
    ("rating", "v.nullable(v.float64())"),
    ("labels", "v.array(v.string())"),
    ("profile", "v.object({ address: v.object({ zip: v.string() }) })"),
    ("metrics", "v.record(v.string(), v.float64())"),
    ("balances", r#"v.record(v.id("accounts"), v.int64())"#),
    ("priority", r#"v.union(v.literal("low"), v.literal("high"))"#),
    ("callback", "v.function()"),
    (
        "onComplete",
        r#"v.function({ fnType: "mutation", args: { jobId: v.string() }, returns: v.null() })"#,
    ),
];

const PACKAGE: &str = r#"{
  "name": "convex-typegen-synthetic-project",
  "private": true,
  "dependencies": { "convex": "1.49.0" }
}
"#;

const SHARED: &str = r#"import { v } from "convex/values";

export const requestId = v.string();
export const auditFields = {
  requestId,
  recordedAt: v.int64(),
};
"#;

// SplitMix64. Explicit wrapping arithmetic keeps the seeded sequence stable across builds.
struct ProjectRng(u64);

impl ProjectRng
{
    fn next(&mut self) -> u64
    {
        self.0 = self.0.wrapping_add(0x9e3779b97f4a7c15);
        let mut mixed = self.0;
        mixed = (mixed ^ (mixed >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        mixed = (mixed ^ (mixed >> 27)).wrapping_mul(0x94d049bb133111eb);
        mixed ^ (mixed >> 31)
    }

    fn index(&mut self, len: usize) -> usize
    {
        (self.next() % len as u64) as usize
    }
}

pub fn write_project(root: &Path, ts_files: usize, seed: u64) -> io::Result<()>
{
    let mut rng = ProjectRng(seed);
    let convex = root.join("convex");
    fs::create_dir_all(
        root.parent()
            .filter(|path| !path.as_os_str().is_empty())
            .unwrap_or(Path::new(".")),
    )?;
    fs::create_dir(root)?;
    fs::create_dir(&convex)?;
    fs::write(root.join("package.json"), PACKAGE)?;
    fs::write(convex.join("shared.ts"), SHARED)?;
    fs::write(convex.join("schema.ts"), schema(&mut rng, ts_files))?;

    let first_kind = rng.index(OPERATIONS.len());
    for index in 0..ts_files - 2 {
        let area = AREAS[rng.index(AREAS.len())];
        let group = GROUPS[rng.index(GROUPS.len())];
        let (directory, relative) = match (index, rng.index(3)) {
            (0, _) => (convex.clone(), "./"),
            (1, _) | (_, 2) => (convex.join(area).join(group), "../../"),
            (_, 1) => (convex.join(area), "../"),
            _ => (convex.clone(), "./"),
        };
        fs::create_dir_all(&directory)?;
        let name = format!("{area}_{group}_{index:05}");
        let kind = if index < OPERATIONS.len() {
            (first_kind + index) % OPERATIONS.len()
        } else {
            rng.index(OPERATIONS.len())
        };
        fs::write(directory.join(format!("{name}.ts")), module(&mut rng, relative, kind))?;
    }
    Ok(())
}

fn schema(rng: &mut ProjectRng, ts_files: usize) -> String
{
    let mut source = String::from(
        r#"import { defineSchema, defineTable } from "convex/server";
import { v } from "convex/values";
import { auditFields } from "./shared";

const audit = { ...auditFields, source: v.string() };

export default defineSchema({
  accounts: defineTable(v.object({
    ...audit,
"#,
    );
    let first = rng.index(VALIDATORS.len());
    for (name, validator) in VALIDATORS.iter().cycle().skip(first).take(VALIDATORS.len()) {
        writeln!(source, "    {name}: {validator},").unwrap();
    }
    source.push_str("  })).index(\"by_title\", [\"title\"]),\n");
    let event_code = rng.next();
    writeln!(
        source,
        r#"  events: defineTable(v.union(
    v.object({{ kind: v.literal("opened_{event_code:x}"), account: v.id("accounts") }}),
    v.object({{ kind: v.literal("closed"), detail: v.object({{ reason: v.string() }}) }}),
  )).index("by_kind", ["kind"]),"#,
    )
    .unwrap();

    let table_count = (ts_files.ilog10() as usize) * (2 + rng.index(3));
    for index in 0..table_count {
        let area = AREAS[rng.index(AREAS.len())];
        writeln!(source, "  {area}{index}: defineTable({{").unwrap();
        sample_fields(&mut source, rng);
        source.push_str("  }),\n");
    }
    source.push_str("});\n");
    source
}

fn sample_fields(source: &mut String, rng: &mut ProjectRng)
{
    let count = 2 + rng.index(6);
    for index in 0..count {
        let (name, validator) = VALIDATORS[rng.index(VALIDATORS.len())];
        writeln!(source, "    {name}{index}: {validator},").unwrap();
    }
}

fn module(rng: &mut ProjectRng, relative: &str, first_kind: usize) -> String
{
    let count = 1 + rng.index(3);
    let operations: Vec<_> = OPERATIONS.iter().cycle().skip(first_kind).take(count).collect();
    let builders = operations.iter().map(|(builder, _)| *builder).collect::<Vec<_>>().join(", ");
    let mut body = String::new();
    let mut uses_spread = false;
    for (builder, operation) in operations {
        writeln!(body, "\nexport const {operation} = {builder}({{\n  args: {{").unwrap();
        if rng.index(2) == 0 {
            uses_spread = true;
            body.push_str("    ...auditFields,\n");
        } else {
            body.push_str("    requestId,\n");
        }
        sample_fields(&mut body, rng);
        let (returns, result) = match rng.index(3) {
            0 => ("requestId", "args.requestId"),
            1 => (
                "v.object({ requestId, accepted: v.boolean() })",
                "({ requestId: args.requestId, accepted: true })",
            ),
            _ => ("v.nullable(requestId)", "args.requestId || null"),
        };
        writeln!(
            body,
            "  }},\n  returns: {returns},\n  handler: (_ctx, args) => {result},\n}});"
        )
        .unwrap();
    }
    let shared = if uses_spread { "auditFields, requestId" } else { "requestId" };
    format!(
        r#"import {{ {builders} }} from "{relative}_generated/server";
import {{ v }} from "convex/values";
import {{ {shared} }} from "{relative}shared";
{body}"#,
    )
}

#[cfg(test)]
mod tests
{
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    use convex_typegen::config::Configuration;
    use convex_typegen::generate;
    use tempfile::tempdir;

    use super::*;

    fn read_tree(root: &Path, dir: &Path, files: &mut BTreeMap<PathBuf, Vec<u8>>)
    {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                read_tree(root, &path, files);
            } else {
                files.insert(path.strip_prefix(root).unwrap().to_owned(), fs::read(path).unwrap());
            }
        }
    }

    fn snapshot(root: &Path) -> BTreeMap<PathBuf, Vec<u8>>
    {
        let mut files = BTreeMap::new();
        read_tree(root, root, &mut files);
        files
    }

    #[test]
    fn same_seed_replays_paths_and_bytes()
    {
        let tmp = tempdir().unwrap();
        let first = tmp.path().join("first");
        let replay = tmp.path().join("replay");
        let different = tmp.path().join("different");
        let (_, count) = TIERS[0];
        write_project(&first, count, 42).unwrap();
        write_project(&replay, count, 42).unwrap();
        write_project(&different, count, 43).unwrap();

        let expected = snapshot(&first);
        assert_eq!(expected.len(), count + 1); // Includes package.json.
        assert_eq!(expected, snapshot(&replay));
        assert_ne!(expected, snapshot(&different));
    }

    #[test]
    fn generated_project_runs_through_offline_codegen()
    {
        let tmp = tempdir().unwrap();
        for seed in [0, 42, u64::MAX] {
            let root = tmp.path().join(seed.to_string());
            write_project(&root, TIERS[0].1, seed).unwrap();
            let convex_dir = root.join("convex");
            let out_file = root.join("bindings.rs");
            generate(Configuration {
                schema_path: convex_dir.join("schema.ts"),
                convex_dir,
                out_file: out_file.clone(),
                ..Configuration::default()
            })
            .unwrap();
            let rust = fs::read_to_string(out_file).unwrap();
            assert!(rust.contains("pub struct AccountsDoc"));
            assert!(rust.contains("pub request_id: String"));
            assert!(rust.contains("pub recorded_at: i64"));
            for (builder, operation) in OPERATIONS {
                assert_eq!(
                    rust.contains(&format!(":{operation}\"")),
                    !builder.starts_with("internal"),
                    "unexpected API visibility for {operation} with seed {seed}"
                );
            }
        }
    }
}
