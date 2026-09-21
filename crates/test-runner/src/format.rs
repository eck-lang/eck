//! Shared structural helpers for PHPT-like runner file formats.

/// Parses the first line of a runner document as a Markdown-style title.
pub(crate) fn parse_title_line(line: &str) -> Result<String, String> {
    let title_line = remove_line_ending(line);
    title_line
        .strip_prefix("# ")
        .filter(|title| !title.trim().is_empty())
        .ok_or_else(|| "the first line must be `# <title>`".to_string())
        .map(|title| title.trim().to_string())
}

/// Trims and validates the prose collected before the first document marker.
pub(crate) fn finish_required_description(description: &mut String) -> Result<String, String> {
    let description = std::mem::take(description).trim().to_string();
    if description.is_empty() {
        return Err("a description is required between the title and first section".into());
    }
    Ok(description)
}

/// Removes one line ending from a structural line without changing its body.
pub(crate) fn remove_line_ending(line: &str) -> &str {
    line.strip_suffix("\r\n")
        .or_else(|| line.strip_suffix('\n'))
        .unwrap_or(line)
}

/// Removes one blank line used only to separate adjacent markers visually.
pub(crate) fn remove_separator_blank_line(contents: &mut String) {
    if let Some(prefix) = contents.strip_suffix("\r\n") {
        if prefix.is_empty() || prefix.ends_with('\n') {
            contents.truncate(contents.len() - 2);
        }
    } else if let Some(prefix) = contents.strip_suffix('\n')
        && (prefix.is_empty() || prefix.ends_with('\n'))
    {
        contents.truncate(contents.len() - 1);
    }
}
