use language_core::{CoreError, Value};
use rust_decimal::Decimal;

use crate::value::get as get_boolean;

/// Multiplies a numeric value by a boolean, treating `true` as one and `false` as zero.
///
/// The `true` path returns the original value without performing numeric multiplication.
pub(super) fn multiplication_numeric_boolean(
    numeric: &Value,
    boolean: &Value,
) -> Result<Value, CoreError> {
    if get_boolean(boolean)? {
        return Ok(numeric.clone());
    }

    if let Some(value) = numeric.downcast_ref::<i64>() {
        return Ok(Value::new(numeric.type_id(), value * 0));
    }
    if let Some(value) = numeric.downcast_ref::<f32>() {
        return Ok(Value::new(numeric.type_id(), value * 0.0));
    }
    if let Some(value) = numeric.downcast_ref::<f64>() {
        return Ok(Value::new(numeric.type_id(), value * 0.0));
    }
    if let Some(value) = numeric.downcast_ref::<Decimal>() {
        return Ok(Value::new(numeric.type_id(), value * Decimal::ZERO));
    }

    Err(CoreError::InvalidValueRepresentation(
        "numeric value compatible with bool multiplication".into(),
    ))
}

/// Multiplies a boolean by a numeric value, using the same semantics as `numeric * bool`.
pub(super) fn multiplication_boolean_numeric(
    boolean: &Value,
    numeric: &Value,
) -> Result<Value, CoreError> {
    multiplication_numeric_boolean(numeric, boolean)
}

#[cfg(test)]
mod tests {
    use language_core::Value;
    use rust_decimal::Decimal;

    use super::{multiplication_boolean_numeric, multiplication_numeric_boolean};

    fn type_id() -> language_core::TypeId {
        crate::test_type_id()
    }

    #[test]
    fn true_returns_the_numeric_operand_unchanged() {
        let numeric = Value::new(type_id(), Decimal::new(125, 2));
        let boolean = Value::new(type_id(), true);

        let result = multiplication_numeric_boolean(&numeric, &boolean).unwrap();

        assert_eq!(
            *result.downcast_ref::<Decimal>().unwrap(),
            Decimal::new(125, 2)
        );
    }

    #[test]
    fn false_multiplies_every_supported_numeric_representation_by_zero() {
        let boolean = Value::new(type_id(), false);

        let integer =
            multiplication_numeric_boolean(&Value::new(type_id(), 7_i64), &boolean).unwrap();
        let float =
            multiplication_numeric_boolean(&Value::new(type_id(), 1.5_f32), &boolean).unwrap();
        let double =
            multiplication_numeric_boolean(&Value::new(type_id(), 1.5_f64), &boolean).unwrap();
        let decimal =
            multiplication_numeric_boolean(&Value::new(type_id(), Decimal::new(15, 1)), &boolean)
                .unwrap();

        assert_eq!(*integer.downcast_ref::<i64>().unwrap(), 0);
        assert_eq!(*float.downcast_ref::<f32>().unwrap(), 0.0);
        assert_eq!(*double.downcast_ref::<f64>().unwrap(), 0.0);
        assert_eq!(*decimal.downcast_ref::<Decimal>().unwrap(), Decimal::ZERO);
    }

    #[test]
    fn supports_a_boolean_as_the_left_operand() {
        let boolean = Value::new(type_id(), false);
        let numeric = Value::new(type_id(), 7_i64);

        let result = multiplication_boolean_numeric(&boolean, &numeric).unwrap();

        assert_eq!(*result.downcast_ref::<i64>().unwrap(), 0);
    }
}
