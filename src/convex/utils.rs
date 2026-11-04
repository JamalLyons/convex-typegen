//! Normalize wire names into Rust identifiers. Codegen checks collisions in each scope.

fn words(name: &str) -> Vec<String>
{
    let chars: Vec<_> = name.chars().collect();
    let mut words = Vec::new();
    let mut word = String::new();
    for (index, &ch) in chars.iter().enumerate() {
        let previous = index.checked_sub(1).and_then(|i| chars.get(i));
        let next = chars.get(index + 1);
        let boundary = ch.is_ascii_uppercase()
            && (previous.is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
                || (previous.is_some_and(char::is_ascii_uppercase) && next.is_some_and(char::is_ascii_lowercase)));
        if (boundary || !ch.is_ascii_alphanumeric()) && !word.is_empty() {
            words.push(std::mem::take(&mut word));
        }
        if ch.is_ascii_alphanumeric() {
            word.push(ch.to_ascii_lowercase());
        }
    }
    if !word.is_empty() {
        words.push(word);
    }
    words
}

pub(super) fn type_ident(name: &str) -> String
{
    pascal_ident(&words(name))
}

pub(super) fn function_model_stem(module: &str, export: &str) -> String
{
    let module = words(module);
    let export = words(export);
    let stem = export
        .strip_prefix(module.as_slice())
        .filter(|suffix| !suffix.is_empty())
        .unwrap_or(&export);
    pascal_ident(stem)
}

fn pascal_ident(words: &[String]) -> String
{
    let mut ident = String::new();
    for word in words {
        ident.push(char::from(word.as_bytes()[0]).to_ascii_uppercase());
        ident.push_str(&word[1..]);
    }
    if ident.is_empty() || ident.starts_with(|c: char| c.is_ascii_digit()) || ident == "Self" {
        ident.insert_str(0, "Type");
    }
    ident
}

pub(super) fn field_ident(name: &str) -> String
{
    let mut ident = words(name).join("_");
    if name.starts_with('_') || ident.starts_with(|c: char| c.is_ascii_digit()) {
        ident.insert(0, '_');
    }
    match ident.as_str() {
        "" | "_" => "_field".into(),
        "self" | "super" | "crate" => format!("_{ident}"),
        "as" | "async" | "await" | "break" | "const" | "continue" | "dyn" | "else" | "enum" | "extern" | "false" | "fn"
        | "for" | "if" | "impl" | "in" | "let" | "loop" | "match" | "mod" | "move" | "mut" | "pub" | "ref" | "return"
        | "static" | "struct" | "trait" | "true" | "type" | "unsafe" | "use" | "where" | "while" | "abstract" | "become"
        | "box" | "do" | "final" | "gen" | "macro" | "override" | "priv" | "try" | "typeof" | "unsized" | "virtual"
        | "yield" => format!("r#{ident}"),
        _ => ident,
    }
}

#[cfg(test)]
mod tests
{
    use super::function_model_stem;

    #[test]
    fn elides_only_a_complete_module_prefix_with_a_nonempty_suffix()
    {
        assert_eq!(function_model_stem("integrations", "integrationsMirror"), "Mirror");
        assert_eq!(function_model_stem("user-tools", "userToolsFind"), "Find");
        assert_eq!(function_model_stem("user", "usersList"), "UsersList");
        assert_eq!(function_model_stem("users", "users"), "Users");
    }
}
