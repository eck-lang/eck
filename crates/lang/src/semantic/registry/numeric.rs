//! Base-representation conversion for numeric values.

use crate::semantic::{CoreError, Registry, TypeId, Value};

impl Registry {
    /// Converts one numeric value into another registered base representation.
    ///
    /// The conversion round-trips through the source formatter and the target
    /// numeric parser so no concrete-type branch is needed. Integer-valued
    /// magnitudes such as `1500.0` are accepted for integer targets by retrying
    /// with the whole part when it carries only zeros.
    ///
    /// The registry owns this conversion because more than one subsystem needs
    /// it: explicit source conversions and the array element contract both
    /// normalize a magnitude into a declared representation. A caller that must
    /// report a domain-specific failure decides how to present the error.
    pub fn convert_base(&self, value: &Value, target: TypeId) -> Result<Value, CoreError> {
        if value.type_id() == target {
            return Ok(value.clone());
        }
        let formatted = (self.type_descriptor(value.type_id())?.format)(value)?;
        match self.parse_numeric(&formatted, Some(target)) {
            Ok(cast) => Ok(cast),
            Err(_) => {
                if let Some((whole, fractional)) = formatted.split_once('.')
                    && fractional.trim_end_matches('0').is_empty()
                {
                    let whole = if whole.is_empty() || whole == "-" || whole == "+" {
                        format!("{whole}0")
                    } else {
                        whole.to_string()
                    };
                    if let Ok(cast) = self.parse_numeric(&whole, Some(target)) {
                        return Ok(cast);
                    }
                }
                Err(CoreError::Runtime(format!(
                    "cannot convert `{}` to `{}`",
                    self.value_type_name(value.value_type()),
                    self.type_name(target)
                )))
            }
        }
    }
}
