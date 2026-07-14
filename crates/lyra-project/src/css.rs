use crate::ProjectError;

/// Applies one minimal CSS custom-property patch without reformatting unrelated text.
///
/// # Errors
///
/// Returns an error for unsafe names/values or when no `:root` block exists.
pub fn patch_css_variable(
    source: &str,
    variable: &str,
    value: &str,
) -> Result<String, ProjectError> {
    if !is_safe_variable(variable) || !is_safe_value(value) {
        return Err(ProjectError::UnsafeCssPatch);
    }

    let root = source.find(":root").ok_or(ProjectError::MissingRootBlock)?;
    let open = source[root..]
        .find('{')
        .map(|offset| root + offset)
        .ok_or(ProjectError::MissingRootBlock)?;
    let close = source[open + 1..]
        .find('}')
        .map(|offset| open + 1 + offset)
        .ok_or(ProjectError::MissingRootBlock)?;
    let body = &source[open + 1..close];

    if let Some(variable_offset) = body.find(variable) {
        let variable_start = open + 1 + variable_offset;
        let after_name = variable_start + variable.len();
        let colon = source[after_name..close]
            .find(':')
            .map(|offset| after_name + offset)
            .ok_or(ProjectError::UnsafeCssPatch)?;
        let semicolon = source[colon + 1..close]
            .find(';')
            .map(|offset| colon + 1 + offset)
            .ok_or(ProjectError::UnsafeCssPatch)?;
        let value_start = colon
            + 1
            + source[colon + 1..semicolon]
                .len()
                .saturating_sub(source[colon + 1..semicolon].trim_start().len());
        let value_end = semicolon
            - source[value_start..semicolon]
                .len()
                .saturating_sub(source[value_start..semicolon].trim_end().len());
        let mut output = source.to_owned();
        output.replace_range(value_start..value_end, value);
        return Ok(output);
    }

    let mut output = source.to_owned();
    let insertion = if source[open + 1..].starts_with('\n') {
        (open + 2, format!("  {variable}: {value};\n"))
    } else {
        (open + 1, format!("\n  {variable}: {value};\n"))
    };
    output.insert_str(insertion.0, &insertion.1);
    Ok(output)
}

fn is_safe_variable(variable: &str) -> bool {
    variable.strip_prefix("--").is_some_and(|name| {
        !name.is_empty()
            && name
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    })
}

fn is_safe_value(value: &str) -> bool {
    !value.is_empty()
        && !value
            .bytes()
            .any(|byte| matches!(byte, b';' | b'{' | b'}' | b'\n' | b'\r'))
}
