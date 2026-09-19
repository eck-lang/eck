use crate::semantic::{ArrayElementMode, ArrayType, CoreError, Registry, Value, ValueType};

use super::*;
use crate::ArrayValue;

/// Allocates a scalar type ID for array payload fixtures.
fn value_type() -> crate::semantic::TypeId {
    Registry::new().allocate_type_id()
}

/// Creates a scalar integer payload fixture.
fn value(number: i64) -> Value {
    Value::new(value_type(), number)
}

/// Creates the array identity used by formatter tests.
fn array_type() -> ArrayType {
    ArrayType {
        element: ValueType::plain(value_type()),
        element_mode: ArrayElementMode::Exact,
    }
}

/// Verifies formatting an array without a registered formatter fails stably.
#[test]
fn missing_formatter_is_reported() {
    let registry = Registry::new();
    let value = Value::new_array(array_type(), ArrayValue::new(vec![value(1), value(2)]));
    let configuration = registry.default_runtime_configuration();

    let error = registry.format_value_with_configuration(&value, &configuration);
    assert!(matches!(error, Err(CoreError::MissingArrayFormatter)));
}

/// Keeps the formatter callback signature tied to the public core contract.
#[test]
fn formatter_type_is_send_sync_compatible() {
    let _: crate::semantic::ArrayValueFormatter = format_value;
}
