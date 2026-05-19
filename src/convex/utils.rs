//! Small string helpers shared by [`crate::convex::codegen`].
//!
//! Convex table/column names become Rust identifiers via [`capitalize_first_letter`] and
//! [`to_pascal_case`] (e.g. union variants from string literals).

use crate::convex::parser::VALID_CONVEX_TYPES;
use crate::error::ConvexTypeGeneratorError;

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

/// Ensures a `v.*` callee matches [`VALID_CONVEX_TYPES`].
pub(crate) fn validate_type_name(type_name: &str) -> Result<(), ConvexTypeGeneratorError>
{
    if !VALID_CONVEX_TYPES.contains(&type_name) {
        return Err(ConvexTypeGeneratorError::InvalidType {
            found: type_name.to_string(),
            valid_types: VALID_CONVEX_TYPES.iter().map(|&s| s.to_string()).collect(),
        });
    }
    Ok(())
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

#[cfg(test)]
mod validate_type_name_tests
{
    use super::validate_type_name;
    use crate::error::ConvexTypeGeneratorError;

    #[test]
    fn accepts_known_convex_validator()
    {
        validate_type_name("string").unwrap();
        validate_type_name("optional").unwrap();
    }

    #[test]
    fn rejects_unknown()
    {
        let e = validate_type_name("not_a_real_validator").unwrap_err();
        assert!(matches!(e, ConvexTypeGeneratorError::InvalidType { found, .. } if found == "not_a_real_validator"));
    }
}
