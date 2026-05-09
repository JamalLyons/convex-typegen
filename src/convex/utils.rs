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
