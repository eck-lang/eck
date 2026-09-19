//! Decimal arithmetic and presentation configuration.
//!
//! `decimal.precision` and `decimal.scale` are destructive arithmetic limits:
//! they are applied to operation results and discarded digits cannot be
//! recovered by a later override. The current `rust_decimal` backend performs
//! the operation first and enforces these limits afterwards, so lower values
//! define numerical semantics but do not currently make the operation faster.
//! A future backend may use them as optimization hints only when doing so
//! preserves exactly the same configured result.
//!
//! `decimal.format.*` is non-destructive presentation policy. It controls text
//! produced by printing and formatter-backed exports such as a future CSV
//! writer without changing the stored decimal value.

use crate::semantic::{
    ConfigurationDescriptor, ConfigurationValue, CoreError, Registry, RuntimeConfiguration,
    TypeConfigurationDescriptor, TypeId, Value,
};
use rust_decimal::RoundingStrategy;

use crate::primitives::decimal::value::get;

const PRECISION_PATH: &str = "decimal.precision";
const SCALE_PATH: &str = "decimal.scale";
const ROUNDING_PATH: &str = "decimal.rounding";
const FORMAT_SCALE_PATH: &str = "decimal.format.scale";
const FORMAT_ROUNDING_PATH: &str = "decimal.format.rounding";
const MAXIMUM_PRECISION: i64 = 28;
const DEFAULT_PRECISION: i64 = MAXIMUM_PRECISION;
const DEFAULT_SCALE: i64 = MAXIMUM_PRECISION;
const DEFAULT_FORMAT_SCALE: i64 = 4;
const DEFAULT_ROUNDING: &str = "Truncate";

/// Registers decimal configuration leaves and their configured runtime behavior.
pub(crate) fn register(registry: &mut Registry, decimal_id: TypeId) -> Result<(), CoreError> {
    registry.register_configuration(ConfigurationDescriptor {
        path: PRECISION_PATH,
        none_object_path: None,
        default: ConfigurationValue::Integer(DEFAULT_PRECISION),
        normalize: normalize_precision,
    })?;
    registry.register_configuration(ConfigurationDescriptor {
        path: SCALE_PATH,
        none_object_path: None,
        default: ConfigurationValue::Integer(DEFAULT_SCALE),
        normalize: normalize_arithmetic_scale,
    })?;
    registry.register_configuration(ConfigurationDescriptor {
        path: ROUNDING_PATH,
        none_object_path: None,
        default: ConfigurationValue::Symbol(DEFAULT_ROUNDING.to_string()),
        normalize: normalize_rounding,
    })?;
    registry.register_configuration(ConfigurationDescriptor {
        path: FORMAT_SCALE_PATH,
        none_object_path: Some("decimal.format"),
        default: ConfigurationValue::Integer(DEFAULT_FORMAT_SCALE),
        normalize: normalize_scale,
    })?;
    registry.register_configuration(ConfigurationDescriptor {
        path: FORMAT_ROUNDING_PATH,
        none_object_path: None,
        default: ConfigurationValue::Symbol(DEFAULT_ROUNDING.to_string()),
        normalize: normalize_rounding,
    })?;
    registry.register_type_configuration(
        decimal_id,
        TypeConfigurationDescriptor {
            transform_result: Some(transform_result),
            transform_owned_result: Some(transform_owned_result),
            initial_result_transform_is_identity: true,
            format: Some(format),
        },
    )
}

/// Normalizes a significant-digit precision, accepting `Max` and capping large values.
fn normalize_precision(value: ConfigurationValue) -> Result<ConfigurationValue, CoreError> {
    match value {
        ConfigurationValue::Integer(value) if value > 0 => {
            Ok(ConfigurationValue::Integer(value.min(MAXIMUM_PRECISION)))
        }
        ConfigurationValue::Symbol(symbol) if symbol == "Max" => {
            Ok(ConfigurationValue::Integer(MAXIMUM_PRECISION))
        }
        ConfigurationValue::Integer(_) => Err(CoreError::InvalidConfigurationValue(
            "precision must be greater than zero".into(),
        )),
        ConfigurationValue::Symbol(symbol) => Err(CoreError::InvalidConfigurationValue(format!(
            "expected a positive integer or `Max`, found `{symbol}`"
        ))),
        ConfigurationValue::None => Err(CoreError::InvalidConfigurationValue(
            "precision cannot be `None`; use `Max` for full precision".into(),
        )),
    }
}

/// Normalizes the maximum fractional scale used by decimal arithmetic.
fn normalize_arithmetic_scale(value: ConfigurationValue) -> Result<ConfigurationValue, CoreError> {
    match value {
        ConfigurationValue::Integer(value) if value >= 0 => {
            Ok(ConfigurationValue::Integer(value.min(MAXIMUM_PRECISION)))
        }
        ConfigurationValue::Symbol(symbol) if symbol == "Max" => {
            Ok(ConfigurationValue::Integer(MAXIMUM_PRECISION))
        }
        ConfigurationValue::Integer(_) => Err(CoreError::InvalidConfigurationValue(
            "scale cannot be negative".into(),
        )),
        ConfigurationValue::Symbol(symbol) => Err(CoreError::InvalidConfigurationValue(format!(
            "expected a non-negative integer or `Max`, found `{symbol}`"
        ))),
        ConfigurationValue::None => Err(CoreError::InvalidConfigurationValue(
            "scale cannot be `None`; use `Max` for the maximum scale".into(),
        )),
    }
}

/// Normalizes an optional display scale and caps it to decimal capacity.
fn normalize_scale(value: ConfigurationValue) -> Result<ConfigurationValue, CoreError> {
    match value {
        ConfigurationValue::None => Ok(ConfigurationValue::None),
        ConfigurationValue::Integer(value) if value >= 0 => {
            Ok(ConfigurationValue::Integer(value.min(MAXIMUM_PRECISION)))
        }
        ConfigurationValue::Integer(_) => Err(CoreError::InvalidConfigurationValue(
            "format scale cannot be negative".into(),
        )),
        ConfigurationValue::Symbol(symbol) => Err(CoreError::InvalidConfigurationValue(format!(
            "expected a non-negative integer or `None`, found `{symbol}`"
        ))),
    }
}

/// Validates one public decimal rounding strategy name.
fn normalize_rounding(value: ConfigurationValue) -> Result<ConfigurationValue, CoreError> {
    match value {
        ConfigurationValue::Symbol(symbol)
            if matches!(
                symbol.as_str(),
                "HalfEven"
                    | "HalfUp"
                    | "HalfDown"
                    | "Truncate"
                    | "AwayFromZero"
                    | "Floor"
                    | "Ceiling"
            ) =>
        {
            Ok(ConfigurationValue::Symbol(symbol))
        }
        ConfigurationValue::Symbol(symbol) => Err(CoreError::InvalidConfigurationValue(format!(
            "unknown decimal rounding strategy `{symbol}`"
        ))),
        ConfigurationValue::Integer(value) => Err(CoreError::InvalidConfigurationValue(format!(
            "expected a rounding strategy, found `{value}`"
        ))),
        ConfigurationValue::None => Err(CoreError::InvalidConfigurationValue(
            "rounding strategy cannot be `None`; disable formatting with `format: None`".into(),
        )),
    }
}

/// Applies significant-digit precision and fractional scale to an operation result.
fn transform_result(
    value: &Value,
    configuration: &RuntimeConfiguration,
) -> Result<Value, CoreError> {
    if configuration.uses_initial_values() {
        return Ok(value.clone());
    }
    let decimal = get(value)?;
    let precision = configuration_integer(configuration, PRECISION_PATH)?;
    let scale = configuration_integer(configuration, SCALE_PATH)?;
    if significant_digit_count(decimal) <= precision && decimal.scale() <= scale {
        return Ok(value.clone());
    }
    let strategy = configuration_rounding(configuration, ROUNDING_PATH)?;
    let precision_rounded = if significant_digit_count(decimal) > precision {
        decimal
            .round_sf_with_strategy(precision, strategy)
            .ok_or_else(|| CoreError::Runtime("decimal precision rounding overflow".into()))?
    } else {
        decimal
    };
    let rounded = if precision_rounded.scale() > scale {
        precision_rounded.round_dp_with_strategy(scale, strategy)
    } else {
        precision_rounded
    };
    Ok(Value::new(value.type_id(), rounded).with_subtype(value.subtype_id()))
}

/// Applies decimal configuration while preserving an owned result at defaults.
fn transform_owned_result(
    value: Value,
    configuration: &RuntimeConfiguration,
) -> Result<Value, CoreError> {
    if configuration.uses_initial_values() {
        Ok(value)
    } else {
        transform_result(&value, configuration)
    }
}

/// Counts the significant digits represented by a decimal coefficient.
fn significant_digit_count(decimal: rust_decimal::Decimal) -> u32 {
    let mantissa = decimal.mantissa().unsigned_abs();
    if mantissa == 0 {
        return 1;
    }
    mantissa.ilog10() + 1
}

/// Formats a decimal with optional scale-limited rounding or truncation.
fn format(value: &Value, configuration: &RuntimeConfiguration) -> Result<String, CoreError> {
    let decimal = get(value)?;
    match configuration.value(FORMAT_SCALE_PATH) {
        Some(ConfigurationValue::None) => Ok(decimal.to_string()),
        Some(ConfigurationValue::Integer(scale)) => {
            let strategy = configuration_rounding(configuration, FORMAT_ROUNDING_PATH)?;
            Ok(decimal
                .round_dp_with_strategy(*scale as u32, strategy)
                .to_string())
        }
        _ => Err(CoreError::Runtime(format!(
            "missing normalized configuration `{FORMAT_SCALE_PATH}`"
        ))),
    }
}

/// Reads one normalized non-negative integer configuration leaf.
fn configuration_integer(
    configuration: &RuntimeConfiguration,
    path: &str,
) -> Result<u32, CoreError> {
    match configuration.value(path) {
        Some(ConfigurationValue::Integer(value)) => Ok(*value as u32),
        _ => Err(CoreError::Runtime(format!(
            "missing normalized configuration `{path}`"
        ))),
    }
}

/// Resolves a configured public rounding name to the decimal library strategy.
fn configuration_rounding(
    configuration: &RuntimeConfiguration,
    path: &str,
) -> Result<RoundingStrategy, CoreError> {
    let Some(ConfigurationValue::Symbol(symbol)) = configuration.value(path) else {
        return Err(CoreError::Runtime(format!(
            "missing normalized configuration `{path}`"
        )));
    };
    match symbol.as_str() {
        "HalfEven" => Ok(RoundingStrategy::MidpointNearestEven),
        "HalfUp" => Ok(RoundingStrategy::MidpointAwayFromZero),
        "HalfDown" => Ok(RoundingStrategy::MidpointTowardZero),
        "Truncate" => Ok(RoundingStrategy::ToZero),
        "AwayFromZero" => Ok(RoundingStrategy::AwayFromZero),
        "Floor" => Ok(RoundingStrategy::ToNegativeInfinity),
        "Ceiling" => Ok(RoundingStrategy::ToPositiveInfinity),
        _ => Err(CoreError::Runtime(format!(
            "unknown normalized decimal rounding strategy `{symbol}`"
        ))),
    }
}

#[cfg(test)]
#[path = "configuration.tests.rs"]
mod tests;
