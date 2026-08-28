use language_core::{CoreError, Value};

use crate::value::get;

/// Adds two floating-point values.
pub(crate) fn addition_float(lhs: &Value, rhs: &Value) -> Result<Value, CoreError> {
    Ok(Value::new(lhs.type_id(), get(lhs)? + get(rhs)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adds_float_values() {
        let lhs = Value::new(crate::test_type_id(), 1.5_f32);
        let rhs = Value::new(crate::test_type_id(), 2.25_f32);

        let result = addition_float(&lhs, &rhs).unwrap();

        assert_eq!(*result.downcast_ref::<f32>().unwrap(), 3.75);
    }
}
