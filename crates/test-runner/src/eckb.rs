//! Parsing for the `.eckb` benchmark definition format.
//!
//! A benchmark keeps one current ECK workload and associates it with ordered
//! historical Git references. Checkpoint comments are metadata only; they do
//! not affect benchmark execution.

use super::format::{
    finish_required_description, parse_title_line, remove_line_ending, remove_separator_blank_line,
};

#[cfg(test)]
#[path = "eckb.tests.rs"]
mod tests;

/// A parsed benchmark containing one workload and its historical checkpoints.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct BenchmarkDefinition {
    pub(crate) title: String,
    pub(crate) description: String,
    pub(crate) checkpoints: Vec<BenchmarkCheckpoint>,
    pub(crate) source: String,
}

/// A historical Git reference and its optional human-readable annotations.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct BenchmarkCheckpoint {
    pub(crate) name: String,
    pub(crate) git_reference: String,
    pub(crate) annotation: Option<String>,
    pub(crate) comment: Option<String>,
}

/// Parses one `.eckb` document into metadata, checkpoints, and source.
pub(crate) fn parse_benchmark(contents: &str) -> Result<BenchmarkDefinition, String> {
    let mut lines = contents.split_inclusive('\n');
    let title_line = lines
        .next()
        .ok_or_else(|| "the file is empty".to_string())?;
    let title = parse_title_line(title_line)?;

    let mut description_buffer = String::new();
    let mut description = None;
    let mut checkpoints = Vec::new();
    let mut current_comment = String::new();
    let mut active_checkpoint = None;
    let mut source: Option<String> = None;
    let mut reading_source = false;

    for line in lines {
        let structural_line = remove_line_ending(line);

        if reading_source {
            if structural_line == ">>> source" {
                return Err("section `source` occurs more than once".into());
            }
            if structural_line.starts_with(">>>") || structural_line.starts_with("<<<") {
                return Err(format!("unknown section marker `{structural_line}`"));
            }
            source
                .as_mut()
                .expect("source storage is initialized before source mode")
                .push_str(line);
            continue;
        }

        if structural_line == ">>> source" {
            finish_checkpoint_comment(
                &mut checkpoints,
                &mut active_checkpoint,
                &mut current_comment,
            );
            if description.is_none() {
                description = Some(finish_required_description(&mut description_buffer)?);
            }
            source = Some(String::new());
            reading_source = true;
            continue;
        }

        if let Some(checkpoint) = parse_checkpoint_marker(structural_line)? {
            if source.is_some() {
                return Err("checkpoint markers must occur before `>>> source`".into());
            }
            if active_checkpoint.is_some() {
                finish_checkpoint_comment(
                    &mut checkpoints,
                    &mut active_checkpoint,
                    &mut current_comment,
                );
            } else {
                description = Some(finish_required_description(&mut description_buffer)?);
            }
            checkpoints.push(checkpoint);
            active_checkpoint = Some(checkpoints.len() - 1);
            continue;
        }

        if structural_line.starts_with(">>>") || structural_line.starts_with("<<<") {
            return Err(format!("unknown section marker `{structural_line}`"));
        }

        if active_checkpoint.is_some() {
            current_comment.push_str(line);
        } else {
            description_buffer.push_str(line);
        }
    }

    finish_checkpoint_comment(
        &mut checkpoints,
        &mut active_checkpoint,
        &mut current_comment,
    );
    let source = source.ok_or_else(|| "missing `>>> source` section".to_string())?;
    let description = description.ok_or_else(|| {
        "a description is required between the title and first section".to_string()
    })?;

    Ok(BenchmarkDefinition {
        title,
        description,
        checkpoints,
        source,
    })
}

/// Parses a checkpoint marker, distinguishing it from unrelated document text.
fn parse_checkpoint_marker(line: &str) -> Result<Option<BenchmarkCheckpoint>, String> {
    let Some(remainder) = line.strip_prefix(">>> checkpoint") else {
        return Ok(None);
    };
    if !remainder.starts_with(char::is_whitespace) {
        return Err(format!("unknown section marker `{line}`"));
    }

    let remainder = remainder.trim_start();
    let (name, remainder) = take_token(remainder)
        .ok_or_else(|| "`>>> checkpoint` requires a name and Git reference".to_string())?;
    let (git_reference, remainder) = take_token(remainder)
        .ok_or_else(|| "`>>> checkpoint` requires a Git reference".to_string())?;
    let annotation = non_empty_trimmed(remainder);

    Ok(Some(BenchmarkCheckpoint {
        name: name.to_string(),
        git_reference: git_reference.to_string(),
        annotation,
        comment: None,
    }))
}

/// Splits one non-empty, whitespace-delimited token from the remaining text.
fn take_token(text: &str) -> Option<(&str, &str)> {
    let text = text.trim_start();
    if text.is_empty() {
        return None;
    }
    let token_end = text.find(char::is_whitespace).unwrap_or(text.len());
    Some((&text[..token_end], &text[token_end..]))
}

/// Converts trimmed non-empty metadata text into an optional owned string.
fn non_empty_trimmed(text: &str) -> Option<String> {
    let text = text.trim();
    (!text.is_empty()).then(|| text.to_string())
}

/// Attaches the accumulated multiline comment to the active checkpoint.
fn finish_checkpoint_comment(
    checkpoints: &mut [BenchmarkCheckpoint],
    active_checkpoint: &mut Option<usize>,
    current_comment: &mut String,
) {
    let Some(index) = active_checkpoint.take() else {
        return;
    };
    remove_separator_blank_line(current_comment);
    checkpoints[index].comment = non_empty_trimmed(current_comment);
    current_comment.clear();
}
