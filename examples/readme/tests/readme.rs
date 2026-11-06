#[test]
fn readme_build_script_matches_tutorial()
{
    assert_eq!(
        include_str!("../../../readme.md")
            .split_once("```rust,ignore\n")
            .unwrap()
            .1
            .split_once("```")
            .unwrap()
            .0
            .trim(),
        include_str!("../build.rs").trim()
    );
}

#[allow(dead_code)]
mod bindings
{
    convex_typegen_runtime::include!();
}

#[test]
fn generated_priority_preserves_absent_null_and_value()
{
    use bindings::api::tasks::{AddArgs, AddArgsPriority};
    use convex::Value;
    use convex_typegen_runtime::value_serde::{from_value, to_value};

    for (priority, expected) in [
        (None, None),
        (Some(None), Some(Value::Null)),
        (Some(Some(AddArgsPriority::High)), Some(Value::String("high".into()))),
    ] {
        let args = AddArgs {
            text: "Write Rust".into(),
            priority,
        };
        let value = to_value(&args).unwrap();
        let Value::Object(fields) = &value else {
            panic!("expected a Convex object")
        };
        assert_eq!(fields.get("priority"), expected.as_ref());
        let decoded: AddArgs = from_value(value.clone()).unwrap();
        assert_eq!(to_value(&decoded).unwrap(), value);
    }
}

#[test]
fn tutorial_uses_the_same_dependency_features_as_the_compiled_example()
{
    let (_, tutorial_manifest) = include_str!("../../../docs/site/src/getting-started.md")
        .split_once("```toml\n")
        .unwrap();
    let (tutorial_manifest, _) = tutorial_manifest.split_once("```").unwrap();
    let (_, example_dependencies) = include_str!("../Cargo.toml").split_once("[dependencies]").unwrap();

    // The example builds the two crates from this checkout, before v1 is published.
    let expected = format!("[dependencies]{example_dependencies}")
        .replace("{ path = \"../../runtime\" }", "\"1\"")
        .replace("{ path = \"../..\" }", "\"1\"");
    assert_eq!(tutorial_manifest.trim(), expected.trim());
}
