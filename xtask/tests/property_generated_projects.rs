use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use convex_typegen::config::Configuration;
use convex_typegen::generate;
use tempfile::tempdir;
use xtask::project::{self, TIERS};

fn seed() -> u64
{
    if let Ok(seed) = std::env::var("CONVEX_TYPEGEN_SEED") {
        return seed.parse().expect("CONVEX_TYPEGEN_SEED must be a u64");
    }
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    (nanos as u64) ^ ((nanos >> 64) as u64)
}

fn compile_and_round_trip(bindings: &Path, manifest_dir: &Path, seed: u64, tier: &str, event_tag: &str)
{
    let temp = tempdir().unwrap();
    let crate_root = temp.path();
    fs::create_dir(crate_root.join("src")).unwrap();
    let runtime = manifest_dir.parent().unwrap().join("runtime");
    fs::write(
        crate_root.join("Cargo.toml"),
        format!(
            "[package]\nname = \"generated-property-check\"\nversion = \"0.0.0\"\nedition = \
             \"2024\"\n\n[dependencies]\nconvex-typegen-runtime = {{ path = {:?} }}\nconvex = \"0.10.4\"\n",
            runtime
        ),
    )
    .unwrap();
    let bindings_path = bindings.canonicalize().unwrap();
    let main = format!(
        r#"
#[path = {bindings:?}]
mod generated;
use convex::Value;
use std::collections::BTreeMap;
use convex_typegen_runtime::value_serde::{{from_value, to_value}};

fn object(fields: impl IntoIterator<Item = (&'static str, Value)>) -> Value {{
    Value::Object(fields.into_iter().map(|(key, value)| (key.to_owned(), value)).collect::<BTreeMap<_, _>>())
}}

fn main() {{
    // Vary wire values across a small deterministic sample while keeping them valid for AccountsDoc.
    for sample in 0..16_i64 {{
        let mut fields = vec![
            ("_id", Value::String("accounts:property-test".into())),
            ("_creationTime", Value::Float64(sample as f64 + 0.25)),
            ("requestId", Value::String(format!("request-{{sample}}"))),
            ("recordedAt", Value::Int64(sample - 8)),
            ("source", Value::String("property-test".into())),
            ("title", Value::String(format!("title-{{sample}}"))),
            ("quantity", Value::Float64(sample as f64 + 0.5)),
            ("score", Value::Float64(sample as f64)),
            ("active", Value::Boolean(sample % 2 == 0)),
            ("cleared", Value::Null),
            ("sequence", Value::Int64(sample - 8)),
            ("revision", Value::Int64(i64::MAX - sample)),
            ("committed", Value::Int64(sample + 100)),
            ("attachment", Value::Bytes(vec![sample as u8, 0, 255])),
            ("raw", Value::Object(BTreeMap::from([("int".into(), Value::Int64(sample)), ("float".into(), Value::Float64(sample as f64))]))),
            ("owner", Value::String("accounts:owner".into())),
            ("state", Value::String("queued".into())),
            ("ratio", Value::Float64(-1.25)),
            ("offset", Value::Int64(-42)),
            ("confirmed", Value::Boolean(true)),
            ("rating", if sample % 2 == 0 {{ Value::Null }} else {{ Value::Float64(sample as f64 + 0.75) }}),
            ("labels", Value::Array(vec![Value::String(format!("label-{{sample}}"))])),
            ("profile", object([("address", object([("zip", Value::String(format!("{{:05}}", sample)))]))])),
            ("metrics", object([("current", Value::Float64(sample as f64 + 0.125))])),
            ("balances", object([("accounts:balance", Value::Int64(sample - 3))])),
            ("priority", Value::String(if sample % 2 == 0 {{ "low" }} else {{ "high" }}.into())),
            ("callback", Value::String("handler".into())),
            ("onComplete", Value::String("handler".into())),
        ];
        if sample % 2 != 0 {{
            fields.push(("note", Value::String(format!("note-{{sample}}"))));
            fields.push(("alias", Value::String(format!("alias-{{sample}}"))));
        }}
        let wire = object(fields);
        let model = from_value::<generated::AccountsDoc>(wire.clone()).unwrap_or_else(|error| panic!("seed={seed} tier={tier} sample={{sample}} decode failed: {{error}}"));
        let encoded = to_value(&model).unwrap_or_else(|error| panic!("seed={seed} tier={tier} sample={{sample}} encode failed: {{error}}"));
        let decoded = from_value::<generated::AccountsDoc>(encoded.clone()).unwrap_or_else(|error| panic!("seed={seed} tier={tier} sample={{sample}} re-decode failed: {{error}}"));
        let reencoded = to_value(&decoded).unwrap_or_else(|error| panic!("seed={seed} tier={tier} sample={{sample}} re-encode failed: {{error}}"));
        assert_eq!(reencoded, encoded, "seed={seed} tier={tier} sample={{sample}} model mismatch");
        assert_eq!(encoded, wire, "seed={seed} tier={tier} sample={{sample}} wire mismatch");

        // Exercise both branches of the generated table-document union, including literal tags.
        for event in [
            object([
                ("_id", Value::String("events:opened".into())),
                ("_creationTime", Value::Float64(sample as f64 + 10.5)),
                ("kind", Value::String({opened_tag:?}.into())),
                ("account", Value::String("accounts:event".into())),
            ]),
            object([
                ("_id", Value::String("events:closed".into())),
                ("_creationTime", Value::Float64(sample as f64 + 10.5)),
                ("kind", Value::String("closed".into())),
                ("detail", object([("reason", Value::String(format!("reason-{{sample}}")))])),
            ]),
        ] {{
            let model = from_value::<generated::EventsDoc>(event.clone()).unwrap_or_else(|error| panic!("seed={seed} tier={tier} sample={{sample}} event decode failed: {{error}}"));
            let encoded = to_value(&model).unwrap_or_else(|error| panic!("seed={seed} tier={tier} sample={{sample}} event encode failed: {{error}}"));
            let decoded = from_value::<generated::EventsDoc>(encoded.clone()).unwrap_or_else(|error| panic!("seed={seed} tier={tier} sample={{sample}} event re-decode failed: {{error}}"));
            let reencoded = to_value(&decoded).unwrap_or_else(|error| panic!("seed={seed} tier={tier} sample={{sample}} event re-encode failed: {{error}}"));
            assert_eq!(reencoded, event, "seed={seed} tier={tier} sample={{sample}} event wire mismatch");
        }}
    }}
}}
"#,
        bindings = bindings_path.display().to_string(),
        opened_tag = format!("opened_{event_tag}"),
    );
    fs::write(crate_root.join("src/main.rs"), main).unwrap();

    let target_dir = manifest_dir.parent().unwrap().join("target");
    let output = Command::new("cargo")
        .args(["run", "--offline", "--quiet", "--manifest-path"])
        .arg(crate_root.join("Cargo.toml"))
        .env("CARGO_TARGET_DIR", target_dir)
        .output()
        .unwrap_or_else(|error| panic!("seed={seed} tier={tier}: could not run generated compile check: {error}"));
    assert!(
        output.status.success(),
        "seed={seed} tier={tier}: generated project failed to compile or round-trip\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn generated_projects_compile_and_round_trip_convex_values()
{
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let out = tempdir().unwrap();
    let base_seed = seed();
    let tier = std::env::var("CONVEX_TYPEGEN_TIER").unwrap_or_else(|_| "small".into());
    let ts_files = TIERS
        .iter()
        .find(|(name, _)| *name == tier)
        .unwrap_or_else(|| panic!("unknown tier {tier:?}"))
        .1;

    // Run the complete generator pipeline on four deterministic seeds. Compile one project so
    // the default tier stays fast; set a different seed or tier to select another sample.
    let mut projects = BTreeMap::new();
    for offset in 0..4_u64 {
        let current_seed = base_seed.wrapping_add(offset);
        let project_root = out.path().join(format!("{tier}-{current_seed}"));
        project::write_project(&project_root, ts_files, current_seed)
            .unwrap_or_else(|error| panic!("seed={current_seed} tier={tier}: generator failed: {error}"));
        let convex_dir = project_root.join("convex");
        let bindings = project_root.join("bindings.rs");
        generate(
            Configuration::default()
                .with_convex_dir(convex_dir)
                .with_out_file(bindings.clone()),
        )
        .unwrap_or_else(|error| panic!("seed={current_seed} tier={tier}: parse -> IR -> codegen failed: {error}"));
        projects.insert(current_seed, bindings);
    }
    let compile_seed = *projects.keys().next().unwrap();
    let schema = fs::read_to_string(projects[&compile_seed].parent().unwrap().join("convex/schema.ts")).unwrap();
    let event_tag = schema
        .split("opened_")
        .nth(1)
        .and_then(|tail| tail.split(['"', '\'']).next())
        .expect("generated event literal tag");
    compile_and_round_trip(&projects[&compile_seed], manifest_dir, compile_seed, &tier, event_tag);
}
