//! Rendering of array payloads through the registry's contextual formatter.

use crate::semantic::{
    ArrayType, ArrayValueFormatter, CoreError, Registry, RuntimeConfiguration, Value,
};

use super::value::{ArrayValue, invalid_array_value};

/// Formats an array recursively through the registry's contextual scalar seam.
///
/// Every element is formatted through the registry's own contract, so an element
/// keeps its subtype suffix and configuration exactly as it would have on its
/// own.
pub(crate) fn format_value(
    registry: &Registry,
    value: &Value,
    array_type: ArrayType,
    configuration: &RuntimeConfiguration,
) -> Result<String, CoreError> {
    if value.array_type() != Some(array_type) {
        return Err(invalid_array_value());
    }
    let array = ArrayValue::from_value(value)?;
    let mut rendered = String::from("[");
    for (index, element) in array.elements().iter().enumerate() {
        if index > 0 {
            rendered.push_str(", ");
        }
        rendered.push_str(&registry.format_value_with_configuration(element, configuration)?);
    }
    rendered.push(']');
    Ok(rendered)
}

/// Keeps the callback type checked at this crate boundary.
const _: ArrayValueFormatter = format_value;

#[cfg(test)]
#[path = "formatting.tests.rs"]
mod tests;
