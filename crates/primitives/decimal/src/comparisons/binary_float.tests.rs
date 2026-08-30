use super::*;
use std::str::FromStr;

/// Verifies equality for binary values that have an exact decimal representation.
#[test]
fn compares_exactly_representable_binary_values() {
    let decimal = Decimal::new(25, 1);

    assert_eq!(
        compare_decimal_with_float(decimal, 2.5_f32),
        Some(Ordering::Equal)
    );
    assert_eq!(
        compare_decimal_with_double(decimal, 2.5_f64),
        Some(Ordering::Equal)
    );
}

/// Verifies that exact comparison retains digits beyond binary precision.
#[test]
fn preserves_decimal_precision() {
    let decimal = Decimal::from_str("2.000000000000000000000000001").unwrap();

    assert_eq!(
        compare_decimal_with_float(decimal, 2.0_f32),
        Some(Ordering::Greater)
    );
    assert_eq!(
        compare_decimal_with_double(decimal, 2.0_f64),
        Some(Ordering::Greater)
    );
}

/// Verifies that nonzero binary values below decimal scale remain nonzero.
#[test]
fn preserves_small_binary_values_that_decimal_cannot_represent() {
    assert_eq!(
        compare_decimal_with_float(Decimal::ZERO, f32::MIN_POSITIVE),
        Some(Ordering::Less)
    );
    assert_eq!(
        compare_decimal_with_double(Decimal::ZERO, 1e-29_f64),
        Some(Ordering::Less)
    );
}

/// Verifies the helper's partial ordering for NaN and signed infinities.
#[test]
fn represents_nan_as_unordered_and_compares_infinities() {
    assert_eq!(compare_decimal_with_double(Decimal::ONE, f64::NAN), None);
    assert_eq!(
        compare_decimal_with_double(Decimal::ONE, f64::INFINITY),
        Some(Ordering::Less)
    );
    assert_eq!(
        compare_decimal_with_double(Decimal::ONE, f64::NEG_INFINITY),
        Some(Ordering::Greater)
    );
}
