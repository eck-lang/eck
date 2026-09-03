//! Parsing for the `.eckt` language-test format.
//!
//! Every test describes one complete Eck execution: its human-readable
//! purpose, source input, and expected process outputs.

use std::collections::BTreeMap;

#[cfg(test)]
#[path = "eckt.tests.rs"]
mod tests;

/// A parsed, self-contained Eck language test.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct LanguageTest {
    pub(crate) title: String,
    pub(crate) description: String,
    pub(crate) configuration: Option<String>,
    pub(crate) source: String,
    pub(crate) expected_standard_output: String,
    pub(crate) expected_standard_error: String,
    pub(crate) expected_exit_code: i32,
}

/// A section that may occur in an `.eckt` file.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Section {
    Configuration,
    Source,
    StandardOutput,
    StandardError,
    ExitCode,
}

/// Parses one `.eckt` document into its metadata, inputs, and expected outputs.
pub(crate) fn parse_language_test(contents: &str) -> Result<LanguageTest, String> {
    let mut lines = contents.split_inclusive('\n');
    let title_line = lines
        .next()
        .ok_or_else(|| "the file is empty".to_string())?;
    let title_line = remove_line_ending(title_line);
    let title = title_line
        .strip_prefix("# ")
        .filter(|title| !title.trim().is_empty())
        .ok_or_else(|| "the first line must be `# <title>`".to_string())?
        .trim()
        .to_string();

    let mut description = String::new();
    let mut sections = BTreeMap::new();
    let mut current_section = None;
    let mut current_contents = String::new();

    for line in lines {
        let line_without_ending = remove_line_ending(line);
        if let Some(section) = parse_section_marker(line_without_ending)? {
            if let Some(previous_section) = current_section {
                remove_separator_blank_line(&mut current_contents);
                if sections
                    .insert(previous_section, std::mem::take(&mut current_contents))
                    .is_some()
                {
                    return Err(format!(
                        "section `{}` occurs more than once",
                        section_name(previous_section)
                    ));
                }
            } else {
                description = description.trim().to_string();
            }
            current_section = Some(section);
        } else if current_section.is_some() {
            current_contents.push_str(line);
        } else {
            description.push_str(line);
        }
    }

    let final_section = current_section.ok_or_else(|| "the test has no sections".to_string())?;
    if sections.insert(final_section, current_contents).is_some() {
        return Err(format!(
            "section `{}` occurs more than once",
            section_name(final_section)
        ));
    }

    if description.is_empty() {
        return Err("a description is required between the title and first section".into());
    }

    let configuration = sections.remove(&Section::Configuration);
    let source = take_required_section(&mut sections, Section::Source)?;
    let expected_standard_output = take_required_section(&mut sections, Section::StandardOutput)?;
    let expected_standard_error = take_required_section(&mut sections, Section::StandardError)?;
    let expected_exit_code = match sections.remove(&Section::ExitCode) {
        Some(value) => value
            .trim()
            .parse()
            .map_err(|_| "`<<< exit` must contain one integer".to_string())?,
        None => 0,
    };

    Ok(LanguageTest {
        title,
        description,
        configuration,
        source,
        expected_standard_output,
        expected_standard_error,
        expected_exit_code,
    })
}

/// Recognizes a complete input or expected-output marker line.
fn parse_section_marker(line: &str) -> Result<Option<Section>, String> {
    let section = match line {
        ">>> config" => Some(Section::Configuration),
        ">>> source" => Some(Section::Source),
        "<<< stdout" => Some(Section::StandardOutput),
        "<<< stderr" => Some(Section::StandardError),
        "<<< exit" => Some(Section::ExitCode),
        _ if line.starts_with(">>>") || line.starts_with("<<<") => {
            return Err(format!("unknown section marker `{line}`"));
        }
        _ => None,
    };
    Ok(section)
}

/// Removes one blank line used only to separate adjacent sections visually.
fn remove_separator_blank_line(contents: &mut String) {
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

/// Removes a single line ending from a structural line without changing its body.
fn remove_line_ending(line: &str) -> &str {
    line.strip_suffix("\r\n")
        .or_else(|| line.strip_suffix('\n'))
        .unwrap_or(line)
}

/// Removes a mandatory section from the parsed section map.
fn take_required_section(
    sections: &mut BTreeMap<Section, String>,
    section: Section,
) -> Result<String, String> {
    sections
        .remove(&section)
        .ok_or_else(|| format!("missing `{}` section", section_marker(section)))
}

/// Returns the complete marker used to introduce a section.
fn section_marker(section: Section) -> &'static str {
    match section {
        Section::Configuration => ">>> config",
        Section::Source => ">>> source",
        Section::StandardOutput => "<<< stdout",
        Section::StandardError => "<<< stderr",
        Section::ExitCode => "<<< exit",
    }
}

/// Returns the human-readable name of a section for validation errors.
fn section_name(section: Section) -> &'static str {
    match section {
        Section::Configuration => "config",
        Section::Source => "source",
        Section::StandardOutput => "stdout",
        Section::StandardError => "stderr",
        Section::ExitCode => "exit",
    }
}
