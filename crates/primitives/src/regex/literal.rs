use language_core::{CoreError, TypeId, Value};

use crate::regex::value::RegexValue;

/// Parses a regex literal of the form `/pattern/flags` into a compiled value.
///
/// The literal is decoded without quotes. `flags` may contain `g` for global
/// replacement and `i`, `m`, `s` for Rust regex inline options. An empty
/// pattern or unknown flags produce an invalid literal error.
pub(crate) fn parse(raw_text: &str, regex_type: TypeId) -> Result<Value, CoreError> {
    if !raw_text.starts_with('/') {
        return Err(CoreError::InvalidLiteral {
            raw_text: raw_text.to_owned(),
            type_name: "regex".into(),
            message: "regex literal must start with `/`".into(),
        });
    }
    let last_slash = raw_text[1..]
        .rfind('/')
        .map(|pos| pos + 1)
        .ok_or_else(|| CoreError::InvalidLiteral {
            raw_text: raw_text.to_owned(),
            type_name: "regex".into(),
            message: "regex literal must contain a closing `/`".into(),
        })?;
    let pattern_raw = &raw_text[1..last_slash];
    let flags_str = &raw_text[last_slash + 1..];

    // Validate flags. `g` controls replacement count, `i`/`m`/`s` are inline regex flags.
    let mut is_global = false;
    let mut inline_flags = String::new();
    for flag in flags_str.chars() {
        match flag {
            'g' => {
                if is_global {
                    return Err(CoreError::InvalidLiteral {
                        raw_text: raw_text.to_owned(),
                        type_name: "regex".into(),
                        message: "duplicate `g` flag in regex literal".into(),
                    });
                }
                is_global = true;
            }
            'i' | 'm' | 's' => {
                if inline_flags.contains(flag) {
                    return Err(CoreError::InvalidLiteral {
                        raw_text: raw_text.to_owned(),
                        type_name: "regex".into(),
                        message: format!("duplicate `{flag}` flag in regex literal"),
                    });
                }
                inline_flags.push(flag);
            }
            _ => {
                return Err(CoreError::InvalidLiteral {
                    raw_text: raw_text.to_owned(),
                    type_name: "regex".into(),
                    message: format!("unknown flag `{flag}` in regex literal"),
                });
            }
        }
    }

    // Decode escaped slashes `\/` → `/` . Other escapes are preserved for the regex engine.
    let pattern = pattern_raw.replace(r"\/", "/");

    let regex_pattern = if inline_flags.is_empty() {
        pattern.clone()
    } else {
        format!("(?{}){}", inline_flags, pattern)
    };

    let regex = regex::Regex::new(&regex_pattern).map_err(|error| CoreError::InvalidLiteral {
        raw_text: raw_text.to_owned(),
        type_name: "regex".into(),
        message: error.to_string(),
    })?;

    let value = RegexValue::new(raw_text.to_owned(), is_global, regex);
    Ok(Value::new(regex_type, value))
}

#[cfg(test)]
#[path = "literal.tests.rs"]
mod tests;
