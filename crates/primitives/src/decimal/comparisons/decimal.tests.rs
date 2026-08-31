use super::*;

/// Verifies equality and ordering between decimal payloads.
#[test]
fn compares_decimal_values() {
    let smaller = Value::new(crate::decimal::test_type_id(0), Decimal::new(20, 1));
    let greater_value = Value::new(crate::decimal::test_type_id(0), Decimal::new(25, 1));

    assert!(less(&smaller, &greater_value).unwrap());
    assert!(greater(&greater_value, &smaller).unwrap());
    assert!(equal(&smaller, &smaller).unwrap());
}
