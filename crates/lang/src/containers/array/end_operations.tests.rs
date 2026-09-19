use crate::ir::ArrayMethod;
use crate::semantic::{ArrayElementMode, ArrayType, CoreError, Registry, Value, ValueType};

use super::*;
use crate::ArrayValue;

/// Allocates a scalar type ID for array payload fixtures.
fn value_type() -> crate::semantic::TypeId {
    Registry::new().allocate_type_id()
}

/// Creates one scalar integer element fixture.
fn element(number: i64) -> Value {
    Value::new(value_type(), number)
}

/// Creates the array identity used by end-operation tests.
fn array_type() -> ArrayType {
    ArrayType {
        element: ValueType::plain(value_type()),
        element_mode: ArrayElementMode::Exact,
    }
}

/// Creates an array payload holding the given integer elements.
fn payload(numbers: &[i64]) -> Value {
    Value::new_array(
        array_type(),
        ArrayValue::new(numbers.iter().map(|number| element(*number)).collect()),
    )
}

/// Reads the integer elements of one array payload fixture.
fn integers(value: &Value) -> Vec<i64> {
    ArrayValue::from_value(value)
        .expect("the fixture holds an array payload")
        .elements()
        .iter()
        .map(|element| {
            *element
                .downcast_ref::<i64>()
                .expect("the fixture stores integer elements")
        })
        .collect()
}

/// Reads one removed element as an integer.
fn removed_integer(value: Option<Value>) -> i64 {
    *value
        .expect("the operation removes one element")
        .downcast_ref::<i64>()
        .expect("the fixture stores integer elements")
}

/// Verifies an addition and a removal act on the last element.
#[test]
fn push_and_pop_move_the_last_element() {
    let mut value = payload(&[1, 2]);

    assert!(
        apply_end_operation(&mut value, ArrayMethod::Push, Some(element(3)))
            .unwrap()
            .is_none()
    );
    assert_eq!(integers(&value), [1, 2, 3]);

    let removed = apply_end_operation(&mut value, ArrayMethod::Pop, None).unwrap();
    assert_eq!(removed_integer(removed), 3);
    assert_eq!(integers(&value), [1, 2]);
}

/// Verifies an addition and a removal act on the first element.
#[test]
fn unshift_and_shift_move_the_first_element() {
    let mut value = payload(&[1, 2]);

    assert!(
        apply_end_operation(&mut value, ArrayMethod::Unshift, Some(element(0)))
            .unwrap()
            .is_none()
    );
    assert_eq!(integers(&value), [0, 1, 2]);

    let removed = apply_end_operation(&mut value, ArrayMethod::Shift, None).unwrap();
    assert_eq!(removed_integer(removed), 0);
    assert_eq!(integers(&value), [1, 2]);
}

/// Verifies a removal from an empty array reports nothing and keeps the array.
#[test]
fn removing_from_an_empty_array_reports_nothing() {
    let mut value = payload(&[]);

    assert!(
        apply_end_operation(&mut value, ArrayMethod::Pop, None)
            .unwrap()
            .is_none()
    );
    assert!(
        apply_end_operation(&mut value, ArrayMethod::Shift, None)
            .unwrap()
            .is_none()
    );
    assert!(integers(&value).is_empty());
}

/// Verifies an addition copies a payload another value still observes.
#[test]
fn an_addition_keeps_a_shared_payload_isolated() {
    let original = payload(&[1, 2]);
    let mut alias = original.clone();
    assert!(!alias.is_uniquely_owned(), "the fixture shares one payload");

    apply_end_operation(&mut alias, ArrayMethod::Push, Some(element(3))).unwrap();

    assert_eq!(integers(&alias), [1, 2, 3]);
    assert_eq!(integers(&original), [1, 2]);
}

/// Verifies an indexed write copies a payload another value still observes.
#[test]
fn set_element_keeps_a_shared_payload_isolated() {
    let original = payload(&[1, 2]);
    let mut alias = original.clone();

    set_element(&mut alias, 0, element(9)).unwrap();

    assert_eq!(integers(&alias), [9, 2]);
    assert_eq!(integers(&original), [1, 2]);
}

/// Verifies an indexed write reports the bounds it refused.
#[test]
fn set_element_reports_an_out_of_bounds_index() {
    let mut value = payload(&[1, 2]);

    let error = set_element(&mut value, 2, element(9)).unwrap_err();

    assert!(matches!(
        error,
        CoreError::ArrayIndexOutOfBounds {
            index: 2,
            length: 2
        }
    ));
    assert_eq!(integers(&value), [1, 2]);
}

/// Verifies an indexed read reports the bounds it refused.
#[test]
fn element_at_reports_an_out_of_bounds_index() {
    let value = payload(&[1, 2]);

    assert_eq!(removed_integer(Some(element_at(&value, 1).unwrap())), 2);
    let error = match element_at(&value, 2) {
        Ok(_) => panic!("the fixture has no element at the refused index"),
        Err(error) => error,
    };
    assert!(matches!(
        error,
        CoreError::ArrayIndexOutOfBounds {
            index: 2,
            length: 2
        }
    ));
}

/// Verifies a scalar value is rejected instead of being reinterpreted.
#[test]
fn a_scalar_value_is_rejected() {
    let mut scalar = element(1);

    let error = match apply_end_operation(&mut scalar, ArrayMethod::Push, Some(element(2))) {
        Ok(_) => panic!("a scalar value is not an array payload"),
        Err(error) => error,
    };

    assert!(matches!(
        error,
        CoreError::InvalidValueRepresentation(name) if name == "array"
    ));
}
