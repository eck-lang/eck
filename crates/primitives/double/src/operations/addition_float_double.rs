use language_core::{CoreError, Value};

use crate::operations::float_double_operands;

/// Adds a float and a double after promoting the float to double precision.
pub(crate) fn addition_float_double(lhs: &Value, rhs: &Value) -> Result<Value, CoreError> {
    let (lhs, rhs, double_id) = float_double_operands(lhs, rhs)?;
    Ok(Value::new(double_id, lhs + rhs))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adds_float_and_double_in_both_orders() {
        let float = Value::new(crate::test_type_id(), 1.5_f32);
        let double = Value::new(crate::test_type_id(), 2.25_f64);

        let float_left = addition_float_double(&float, &double).unwrap();
        let double_left = addition_float_double(&double, &float).unwrap();

        assert_eq!(float_left.type_id(), double.type_id());
        assert_eq!(*float_left.downcast_ref::<f64>().unwrap(), 3.75);
        assert_eq!(*double_left.downcast_ref::<f64>().unwrap(), 3.75);
    }
}
