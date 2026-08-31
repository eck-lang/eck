use std::cmp::Ordering;

use rust_decimal::Decimal;

/// Compares a decimal with the exact IEEE-754 value represented by a double.
///
/// Returns `None` for NaN because NaN is unordered. Finite values are decoded
/// into a binary significand and exponent instead of being rounded to decimal.
pub(super) fn compare_decimal_with_double(decimal: Decimal, double: f64) -> Option<Ordering> {
    if double.is_nan() {
        return None;
    }
    if double.is_infinite() {
        return Some(if double.is_sign_negative() {
            Ordering::Greater
        } else {
            Ordering::Less
        });
    }

    let bits = double.to_bits();
    let negative = bits >> 63 != 0;
    let biased_exponent = ((bits >> 52) & 0x7ff) as i32;
    let fraction = bits & 0x000f_ffff_ffff_ffff;
    let (significand, binary_exponent) = if biased_exponent == 0 {
        (fraction, -1074)
    } else {
        ((1_u64 << 52) | fraction, biased_exponent - 1023 - 52)
    };

    Some(compare_decimal_with_binary_float(
        decimal,
        negative,
        significand,
        binary_exponent,
    ))
}

/// Compares a decimal with the exact IEEE-754 value represented by a float.
///
/// Returns `None` for NaN because NaN is unordered. Finite values are decoded
/// into a binary significand and exponent instead of being rounded to decimal.
pub(super) fn compare_decimal_with_float(decimal: Decimal, float: f32) -> Option<Ordering> {
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
    let negative = bits >> 31 != 0;
    let biased_exponent = ((bits >> 23) & 0xff) as i32;
    let fraction = (bits & 0x007f_ffff) as u64;
    let (significand, binary_exponent) = if biased_exponent == 0 {
        (fraction, -149)
    } else {
        ((1_u64 << 23) | fraction, biased_exponent - 127 - 23)
    };

    Some(compare_decimal_with_binary_float(
        decimal,
        negative,
        significand,
        binary_exponent,
    ))
}

/// Compares a decimal with a finite binary floating-point decomposition.
///
/// Both magnitudes are transformed into scaled integers, preserving every bit
/// of the binary value and every decimal digit without heap allocation.
fn compare_decimal_with_binary_float(
    decimal: Decimal,
    binary_float_is_negative: bool,
    binary_float_significand: u64,
    binary_float_exponent: i32,
) -> Ordering {
    let decimal_is_zero = decimal.is_zero();
    let binary_float_is_zero = binary_float_significand == 0;
    if decimal_is_zero && binary_float_is_zero {
        return Ordering::Equal;
    }

    let decimal_is_negative = decimal.is_sign_negative() && !decimal_is_zero;
    let binary_float_is_negative = binary_float_is_negative && !binary_float_is_zero;
    if decimal_is_negative != binary_float_is_negative {
        return if decimal_is_negative {
            Ordering::Less
        } else {
            Ordering::Greater
        };
    }

    let decimal_significand = decimal.mantissa().unsigned_abs();
    let decimal_scale = decimal.scale();
    let scaled_binary_float_significand =
        u128::from(binary_float_significand) * 5_u128.pow(decimal_scale);
    let binary_float_exponent = binary_float_exponent + decimal_scale as i32;

    let magnitude_ordering = if binary_float_exponent >= 0 {
        compare_binary_scaled_unsigned_integers(
            decimal_significand,
            0,
            scaled_binary_float_significand,
            binary_float_exponent as u32,
        )
    } else {
        compare_binary_scaled_unsigned_integers(
            decimal_significand,
            binary_float_exponent.unsigned_abs(),
            scaled_binary_float_significand,
            0,
        )
    };

    if decimal_is_negative {
        magnitude_ordering.reverse()
    } else {
        magnitude_ordering
    }
}

/// Compares `left_significand * 2^left_shift` with the corresponding right value.
///
/// Bit lengths decide values with different magnitudes. Equal-length values
/// need at most a 127-bit relative shift after removing their common power of
/// two, so the final exact comparison still fits in `u128`.
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

#[cfg(test)]
#[path = "binary_float.tests.rs"]
mod tests;
