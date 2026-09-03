use language_core::{CoreError, ExecutionContext, Value};

use crate::regex::value::get as get_regex;
use crate::string::value::get;

/// Replaces occurrences of the regex pattern in the string receiver with `replacement`.
///
/// If the regex was compiled with the global `g` flag, all occurrences are
/// replaced. Otherwise, only the first match is replaced. The replacement
/// supports `$1`, `$name` expansions as defined by the `regex` crate.
pub(crate) fn replace_regex(
    context: &ExecutionContext<'_>,
    arguments: &[Value],
) -> Result<Option<Value>, CoreError> {
    if arguments.len() != 3 {
        return Err(CoreError::Runtime(
            "replace expects a string receiver, a regex pattern, and a replacement".into(),
        ));
    }
    let text = get(&arguments[0])?;
    let regex_value = get_regex(&arguments[1])?;
    let replacement = get(&arguments[2])?;
    let transformed = if regex_value.is_global() {
        regex_value
            .regex()
            .replace_all(text, replacement)
            .into_owned()
    } else {
        regex_value
            .regex()
            .replace(text, replacement)
            .into_owned()
    };
    let value = context.registry().parse_string(&transformed, None)?;
    Ok(Some(value))
}

#[cfg(test)]
#[path = "replace_regex.tests.rs"]
mod tests;
