use std::cmp::Ordering;

use language_core::{ComparisonOperator, CoreError, Registry, Value};

use super::{declare_pair, evaluate};

/// Registers float-integer comparisons in both operand orders.
pub(super) fn register(registry: &mut Registry) -> Result<(), CoreError> {
    let executors = [
        equal,
        not_equal,
        less,
        less_or_equal,
        greater,
        greater_or_equal,
    ];
    declare_pair(registry, "float", "int", executors)?;
    declare_pair(registry, "int", "float", executors)
}

/// Compares a float and an integer without converting either value to the other type.
///
/// The returned partial ordering follows the original operand order and is
/// `None` when the float payload is NaN.
fn compare(left_operand: &Value, right_operand: &Value) -> Result<Option<Ordering>, CoreError> {
    if let (Some(float), Some(integer)) = (
        left_operand.downcast_ref::<f32>(),
        right_operand.downcast_ref::<i64>(),
    ) {
        return Ok(compare_integer_with_float(*integer, *float).map(Ordering::reverse));
    }
    if let (Some(integer), Some(float)) = (
        left_operand.downcast_ref::<i64>(),
        right_operand.downcast_ref::<f32>(),
    ) {
        return Ok(compare_integer_with_float(*integer, *float));
    }

    Err(CoreError::InvalidValueRepresentation(
        "float and int comparison operands".into(),
    ))
}

/// Compares an integer with the exact IEEE-754 value represented by a float.
///
/// Finite floats are decoded into a binary significand and exponent, avoiding
/// the rounding that an `i64`-to-`f32` conversion would introduce.
fn compare_integer_with_float(integer: i64, float: f32) -> Option<Ordering> {
    if float.is_nan() {
        return None;
    }
    if float.is_infinite() {
        return Some(if float.is_sign_negative() {
            Ordering::Greater
        } else {
            Ordering::Less
        });
    }

    let bits = float.to_bits();
    let float_is_negative = bits >> 31 != 0;
    let biased_exponent = ((bits >> 23) & 0xff) as i32;
    let fraction = bits & 0x007f_ffff;
    let (float_significand, float_exponent) = if biased_exponent == 0 {
        (fraction, -149)
    } else {
        ((1_u32 << 23) | fraction, biased_exponent - 127 - 23)
    };

    Some(compare_finite_integer_with_binary_float(
        integer,
        float_is_negative,
        float_significand,
        float_exponent,
    ))
}

/// Compares an integer with a finite binary floating-point decomposition.
fn compare_finite_integer_with_binary_float(
    integer: i64,
    binary_float_is_negative: bool,
    binary_float_significand: u32,
    binary_float_exponent: i32,
) -> Ordering {
    let integer_is_zero = integer == 0;
    let binary_float_is_zero = binary_float_significand == 0;
    if integer_is_zero && binary_float_is_zero {
        return Ordering::Equal;
    }

    let integer_is_negative = integer.is_negative();
    let binary_float_is_negative = binary_float_is_negative && !binary_float_is_zero;
    if integer_is_negative != binary_float_is_negative {
        return if integer_is_negative {
            Ordering::Less
        } else {
            Ordering::Greater
        };
    }

    let integer_significand = u128::from(integer.unsigned_abs());
    let binary_float_significand = u128::from(binary_float_significand);
    let magnitude_ordering = if binary_float_exponent >= 0 {
        compare_binary_scaled_unsigned_integers(
            integer_significand,
            0,
            binary_float_significand,
            binary_float_exponent as u32,
        )
    } else {
        compare_binary_scaled_unsigned_integers(
            integer_significand,
            binary_float_exponent.unsigned_abs(),
            binary_float_significand,
            0,
        )
    };

    if integer_is_negative {
        magnitude_ordering.reverse()
    } else {
        magnitude_ordering
    }
}

/// Compares two unsigned integers multiplied by independent powers of two.
///
/// Bit lengths decide values with different magnitudes. Equal-length values
/// need at most a 127-bit relative shift, so the exact comparison fits in
/// `u128` even when one conceptual absolute shift is much larger.
fn compare_binary_scaled_unsigned_integers(
    left_significand: u128,
    left_shift: u32,
    right_significand: u128,
    right_shift: u32,
) -> Ordering {
    if left_significand == 0 || right_significand == 0 {
        return left_significand.cmp(&right_significand);
    }

    let left_bit_length = u128::BITS - left_significand.leading_zeros() + left_shift;
    let right_bit_length = u128::BITS - right_significand.leading_zeros() + right_shift;
    match left_bit_length.cmp(&right_bit_length) {
        Ordering::Equal => {
            let common_shift = left_shift.min(right_shift);
            let normalized_left_shift = left_shift - common_shift;
            let normalized_right_shift = right_shift - common_shift;
            debug_assert!(normalized_left_shift < u128::BITS);
            debug_assert!(normalized_right_shift < u128::BITS);
            (left_significand << normalized_left_shift)
                .cmp(&(right_significand << normalized_right_shift))
        }
        ordering => ordering,
    }
}

/// Returns whether the float and integer operands are equal.
fn equal(left_operand: &Value, right_operand: &Value) -> Result<bool, CoreError> {
    compare(left_operand, right_operand)
        .map(|ordering| evaluate(ordering, ComparisonOperator::Equal))
}

/// Returns whether the float and integer operands are different.
fn not_equal(left_operand: &Value, right_operand: &Value) -> Result<bool, CoreError> {
    compare(left_operand, right_operand)
        .map(|ordering| evaluate(ordering, ComparisonOperator::NotEqual))
}

/// Returns whether the left operand is less than the right operand.
fn less(left_operand: &Value, right_operand: &Value) -> Result<bool, CoreError> {
    compare(left_operand, right_operand)
        .map(|ordering| evaluate(ordering, ComparisonOperator::Less))
}

/// Returns whether the left operand is less than or equal to the right operand.
fn less_or_equal(left_operand: &Value, right_operand: &Value) -> Result<bool, CoreError> {
    compare(left_operand, right_operand)
        .map(|ordering| evaluate(ordering, ComparisonOperator::LessOrEqual))
}

/// Returns whether the left operand is greater than the right operand.
fn greater(left_operand: &Value, right_operand: &Value) -> Result<bool, CoreError> {
    compare(left_operand, right_operand)
        .map(|ordering| evaluate(ordering, ComparisonOperator::Greater))
}

/// Returns whether the left operand is greater than or equal to the right operand.
fn greater_or_equal(left_operand: &Value, right_operand: &Value) -> Result<bool, CoreError> {
    compare(left_operand, right_operand)
        .map(|ordering| evaluate(ordering, ComparisonOperator::GreaterOrEqual))
}

#[cfg(test)]
#[path = "float_integer.tests.rs"]
mod tests;
