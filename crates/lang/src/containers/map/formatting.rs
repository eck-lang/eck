//! Deterministic insertion-order rendering for map payloads.

use crate::semantic::{CoreError, Registry, RuntimeConfiguration, Value};

use super::value::MapValue;

/// Formats a map as insertion-ordered key/value pairs.
///
/// Scalar and array entries use the registry's contextual formatter. Nested
/// maps recurse through this function until registry-level map formatting is
/// wired by the runtime integration.
pub fn format_value(
    registry: &Registry,
    value: &Value,
    configuration: &RuntimeConfiguration,
) -> Result<String, CoreError> {
    format_map_value(registry, value, configuration, 0)
}

/// Formats one map with four spaces of indentation per nesting level.
fn format_map_value(
    registry: &Registry,
    value: &Value,
    configuration: &RuntimeConfiguration,
    indentation: usize,
) -> Result<String, CoreError> {
    let map = MapValue::from_value(value)?;
    if map.entries().is_empty() {
        return Ok("{}".into());
    }

    let mut rendered = String::from("{\n");
    for (index, entry) in map.entries().iter().enumerate() {
        rendered.push_str(&indentation_string(indentation + 1));
        rendered.push_str(&format_nested_value(
            registry,
            &entry.key,
            configuration,
            indentation + 1,
        )?);
        rendered.push_str(": ");
        rendered.push_str(&format_nested_value(
            registry,
            &entry.value,
            configuration,
            indentation + 1,
        )?);
        if index + 1 < map.entries().len() {
            rendered.push(',');
        }
        rendered.push('\n');
    }
    rendered.push_str(&indentation_string(indentation));
    rendered.push('}');
    Ok(rendered)
}

/// Formats a nested map locally and delegates every other value to the registry.
fn format_nested_value(
    registry: &Registry,
    value: &Value,
    configuration: &RuntimeConfiguration,
    indentation: usize,
) -> Result<String, CoreError> {
    if value.map_type().is_some() {
        format_map_value(registry, value, configuration, indentation)
    } else if registry.type_descriptor(value.type_id())?.name == "string" {
        let text = value
            .downcast_ref::<String>()
            .ok_or_else(|| CoreError::InvalidValueRepresentation("string".into()))?;
        Ok(format!("\"{}\"", escape_string(text)))
    } else {
        registry.format_value_with_configuration(value, configuration)
    }
}

/// Returns the ECK source representation of a string stored inside a map.
fn escape_string(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            '\0' => escaped.push_str("\\0"),
            character => escaped.push(character),
        }
    }
    escaped
}

/// Returns indentation for one map nesting level.
fn indentation_string(level: usize) -> String {
    "    ".repeat(level)
}

#[cfg(test)]
#[path = "formatting.tests.rs"]
mod tests;
