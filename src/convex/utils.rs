//! Small string helpers shared by [`crate::convex::codegen`].
//!
//! Convex table/column names become Rust identifiers via [`capitalize_first_letter`] and
//! [`to_pascal_case`] (e.g. union variants from string literals).

use crate::convex::validator::MAX_OBJECT_NEST_DEPTH;

/// `users` → `Users` for struct / enum name prefixes.
pub(crate) fn capitalize_first_letter(s: &str) -> String
{
    if s.is_empty() {
        return String::new();
    }

    let mut chars = s.chars();
    let first_char = chars.next().expect("Expected a character but got none");
    let rest = chars.collect::<String>();

    first_char.to_uppercase().to_string() + &rest
}

/// `{Module}{Export}Args` when the export name does not already start with the module segment.
///
/// Examples: `games` + `getGame` → `GamesGetGameArgs`; `mod_a` + `list` → `ModAListArgs`;
/// `tasks` + `tasksSearch` → `TasksSearchArgs` (export already prefixed).
pub(crate) fn function_args_struct_name(module: &str, export: &str) -> String
{
    let module_pascal = to_pascal_case(module);
    let export_pascal = capitalize_first_letter(export);

    let base = if export_pascal.starts_with(&module_pascal) {
        export_pascal
    } else {
        format!("{module_pascal}{export_pascal}")
    };

    format!("{base}Args")
}

/// Stem used for nested object struct names (`GamesGetGameArgs` → `GamesGetGame`).
pub(crate) fn function_args_struct_stem(module: &str, export: &str) -> String
{
    let full = function_args_struct_name(module, export);
    full.strip_suffix("Args").map(str::to_string).unwrap_or(full)
}

/// `projects` + `settings` → `ProjectsSettings`.
pub(crate) fn schema_column_object_struct_name(table: &str, column: &str) -> String
{
    format!("{}{}", capitalize_first_letter(table), capitalize_first_letter(column))
}

/// `ProjectsSettings` + `theme` → `ProjectsSettingsTheme`.
pub(crate) fn nested_object_struct_name(parent_struct: &str, field: &str) -> String
{
    format!("{parent_struct}{}", capitalize_first_letter(field))
}

/// Whether object nesting depth allows a generated struct (vs `ConvexJsonValue` fallback).
pub(crate) fn object_depth_allows_struct(depth: usize) -> bool
{
    depth < MAX_OBJECT_NEST_DEPTH
}

/// Split on non-alphanumeric runs and uppercase each word (`foo_bar` → `FooBar`).
pub(crate) fn to_pascal_case(s: &str) -> String
{
    s.split(|c: char| !c.is_alphanumeric())
        .filter(|s| !s.is_empty())
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().collect::<String>() + &chars.collect::<String>().to_lowercase(),
            }
        })
        .collect()
}

#[cfg(test)]
mod function_args_struct_name_tests
{
    use super::function_args_struct_name;

    #[test]
    fn combines_module_and_export()
    {
        assert_eq!(function_args_struct_name("games", "getGame"), "GamesGetGameArgs");
        assert_eq!(function_args_struct_name("mod_a", "list"), "ModAListArgs");
        assert_eq!(function_args_struct_name("mod_b", "list"), "ModBListArgs");
        assert_eq!(function_args_struct_name("tasks", "tasksSearch"), "TasksSearchArgs");
        assert_eq!(function_args_struct_name("api", "getItem"), "ApiGetItemArgs");
    }
}

#[cfg(test)]
mod capitalize_first_letter_tests
{
    use super::capitalize_first_letter;

    #[test]
    fn empty_string()
    {
        assert_eq!(capitalize_first_letter(""), "");
    }

    #[test]
    fn single_ascii_char()
    {
        assert_eq!(capitalize_first_letter("a"), "A");
    }

    #[test]
    fn preserves_rest_case()
    {
        assert_eq!(capitalize_first_letter("games"), "Games");
    }

    #[test]
    fn unicode_first_char()
    {
        assert_eq!(capitalize_first_letter("über"), "Über");
    }
}

#[cfg(test)]
mod to_pascal_case_tests
{
    use super::to_pascal_case;

    #[test]
    fn empty()
    {
        assert_eq!(to_pascal_case(""), "");
    }

    #[test]
    fn snake_and_separators()
    {
        assert_eq!(to_pascal_case("foo_bar-baz"), "FooBarBaz");
    }

    #[test]
    fn literal_union_style()
    {
        assert_eq!(to_pascal_case("draft"), "Draft");
    }
}
