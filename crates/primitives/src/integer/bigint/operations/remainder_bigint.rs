use language_core::{CoreError, Value};
use num_bigint::{BigInt, Sign};

use crate::integer::bigint::value::{get, get_mut, mixed_operands};

/// Number of base-2^32 digits a masked dividend may need without allocating.
///
/// The buffer covers every divisor up to 2^2048, which is far wider than the
/// values this primitive normally reduces. Wider divisors keep the general
/// division path.
const MASK_DIGIT_LIMIT: usize = 64;

/// Returns the exponent when `divisor` is a positive power of two.
///
/// Only a positive power of two has a bit-mask remainder, so every other
/// divisor keeps the general division path. Returns `None` for zero, negative,
/// and non-power-of-two divisors.
fn positive_power_of_two_exponent(divisor: &BigInt) -> Option<u32> {
    if divisor.sign() != Sign::Plus {
        return None;
    }
    let mut exponent = None;
    for (index, digit) in divisor.magnitude().iter_u32_digits().enumerate() {
        if digit == 0 {
            continue;
        }
        if exponent.is_some() || !digit.is_power_of_two() {
            return None;
        }
        exponent = Some(index as u32 * 32 + digit.trailing_zeros());
    }
    exponent
}

/// Writes the low `exponent` bits of `value` into `buffer`.
///
/// Returns the number of digits written, or `None` when the value needs more
/// digits than `buffer` can hold, which tells the caller to fall back to
/// division.
fn masked_low_digits(value: &BigInt, exponent: u32, buffer: &mut [u32]) -> Option<usize> {
    let digit_count = (exponent as usize).div_ceil(32);
    if digit_count > buffer.len() {
        return None;
    }
    for (index, digit) in value
        .magnitude()
        .iter_u32_digits()
        .take(digit_count)
        .enumerate()
    {
        buffer[index] = digit;
    }
    let top_bits = exponent % 32;
    if top_bits != 0 && digit_count > 0 {
        buffer[digit_count - 1] &= (1_u32 << top_bits) - 1;
    }
    Some(digit_count)
}

/// Calculates the arbitrary-precision integer remainder, rejecting zero divisors.
pub(crate) fn remainder_integer(lhs: &Value, rhs: &Value) -> Result<Value, CoreError> {
    let divisor = get(rhs)?;
    if divisor.sign() == Sign::NoSign {
        return Err(CoreError::DivisionByZero);
    }
    let dividend = get(lhs)?;
    if let Some(exponent) = positive_power_of_two_exponent(divisor) {
        let mut buffer = [0_u32; MASK_DIGIT_LIMIT];
        if let Some(digit_count) = masked_low_digits(dividend, exponent, &mut buffer) {
            return Ok(Value::new(
                lhs.type_id(),
                BigInt::from_slice(dividend.sign(), &buffer[..digit_count]),
            ));
        }
    }
    Ok(Value::new(lhs.type_id(), dividend % divisor))
}

/// Reduces a uniquely owned `bigint` left operand modulo a non-zero right operand.
///
/// A positive power-of-two divisor reduces through a bit mask that rewrites the
/// owned operand in place, which keeps the existing digit buffer instead of
/// allocating a fresh remainder.
pub(crate) fn remainder_integer_in_place(
    left_operand: &mut Value,
    right_operand: &Value,
) -> Result<(), CoreError> {
    let right_operand = get(right_operand)?;
    if right_operand.sign() == Sign::NoSign {
        return Err(CoreError::DivisionByZero);
    }
    if let Some(exponent) = positive_power_of_two_exponent(right_operand) {
        let left_operand = get_mut(left_operand)?;
        let mut buffer = [0_u32; MASK_DIGIT_LIMIT];
        if let Some(digit_count) = masked_low_digits(left_operand, exponent, &mut buffer) {
            let sign = left_operand.sign();
            left_operand.assign_from_slice(sign, &buffer[..digit_count]);
            return Ok(());
        }
    }
    *get_mut(left_operand)? %= right_operand;
    Ok(())
}

/// Calculates the mixed-width integer remainder after losslessly promoting
/// both operands to `bigint`, rejecting zero divisors.
pub(crate) fn remainder_mixed_integer(
    left_operand: &Value,
    right_operand: &Value,
) -> Result<Value, CoreError> {
    let (left_operand, right_operand, result_type_id) =
        mixed_operands(left_operand, right_operand)?;
    if right_operand.sign() == Sign::NoSign {
        return Err(CoreError::DivisionByZero);
    }
    Ok(Value::new(result_type_id, left_operand % right_operand))
}

#[cfg(test)]
#[path = "remainder_bigint.tests.rs"]
mod tests;
