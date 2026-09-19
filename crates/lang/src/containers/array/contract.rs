//! Element-contract application for values entering array storage.

use crate::semantic::{CoreError, Registry, RuntimeConfiguration, Value, ValueType};

/// Applies one array element contract to a value about to enter storage.
///
/// Evaluating an integer expression may temporarily promote its result to a
/// wider representation, which stays legal as long as the final value fits the
/// representation the destination array declared. The value is therefore
/// normalized back to that representation here, before the array is touched, so
/// a rejected store leaves the stored element unchanged.
///
/// The destination subtype wins when the array constrains one; otherwise the
/// value keeps the subtype it was stored with.
pub fn apply_element_contract(
    registry: &Registry,
    configuration: &RuntimeConfiguration,
    value: Value,
    element: ValueType,
) -> Result<Value, CoreError> {
    if value.type_id() == element.base {
        return Ok(value);
    }
    let subtype = element.subtype.or(value.subtype_id());
    let normalized = match registry.convert_base(&value, element.base) {
        Ok(normalized) => normalized,
        Err(_) => {
            return Err(representation_error(
                registry,
                configuration,
                &value,
                element,
            ));
        }
    };
    Ok(normalized.with_subtype(subtype))
}

/// Reports a value the declared element representation cannot hold.
fn representation_error(
    registry: &Registry,
    configuration: &RuntimeConfiguration,
    value: &Value,
    element: ValueType,
) -> CoreError {
    let formatted = match registry.format_value_with_configuration(value, configuration) {
        Ok(formatted) => formatted,
        Err(error) => return error,
    };
    CoreError::ArrayElementNotRepresentable {
        value: formatted,
        element: registry.value_type_name(element),
    }
}

#[cfg(test)]
#[path = "contract.tests.rs"]
mod tests;
