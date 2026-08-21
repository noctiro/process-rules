pub fn contains_control_or_line_separator(value: &str) -> bool {
    value.chars().any(|character| {
        character.is_control() || matches!(character, '\u{0085}' | '\u{2028}' | '\u{2029}')
    })
}

pub fn validate_display_name(value: &str) -> Result<(), String> {
    if value.is_empty() {
        return Err("display_name must not be empty".to_owned());
    }
    if value.trim() != value {
        return Err("display_name must not have surrounding whitespace".to_owned());
    }
    if contains_control_or_line_separator(value) {
        return Err(
            "display_name must not contain control characters or line separators".to_owned(),
        );
    }
    Ok(())
}
