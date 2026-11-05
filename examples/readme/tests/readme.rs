// Keep the documented consumer identical to the workspace-compiled example.
#[test]
fn readme_snippets_match_compiled_sources()
{
    let snippets: Vec<_> = include_str!("../../../readme.md")
        .split("```rust,ignore\n")
        .skip(1)
        .map(|block| block.split("```").next().unwrap().trim())
        .collect();
    assert_eq!(
        snippets,
        [include_str!("../build.rs").trim(), include_str!("../src/main.rs").trim()]
    );
}
