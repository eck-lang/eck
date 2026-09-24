//! Hashable scalar key representation for map storage.

use std::sync::Arc;

use num_bigint::BigInt;
use rust_decimal::Decimal;

use crate::semantic::{CoreError, Registry, SemanticType, Value, ValueType};

/// Names of the built-in scalar representations accepted as map keys.
const SUPPORTED_TYPE_NAMES: &[&str] = &[
    "bool", "string", "int8", "int16", "int32", "int64", "int128", "bigint", "uint8", "uint64",
    "float", "double", "decimal",
];

/// One owned, hashable map key with its complete runtime scalar identity.
///
/// The value type distinguishes base types and subtypes. The payload enum also
/// distinguishes concrete scalar representations, so equality never coerces
/// between integer widths, floating-point widths, decimals, or strings.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct MapKey {
    value_type: ValueType,
    payload: MapKeyPayload,
}

/// Concrete scalar payload retained by one map key.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum MapKeyPayload {
    Boolean(bool),
    String(Arc<str>),
    Integer8(i8),
    Integer16(i16),
    Integer32(i32),
    Integer64(i64),
    Integer128(i128),
    BigInteger(Arc<BigInt>),
    UnsignedInteger8(u8),
    UnsignedInteger64(u64),
    Float(u32),
    Double(u64),
    Decimal(Decimal),
}

impl MapKey {
    /// Reports whether a registered scalar type has a supported map-key representation.
    ///
    /// Qualified values are supported when both their base type and subtype are
    /// registered. This performs no payload validation and is suitable for the
    /// compiler's known-type checks.
    pub fn supports_type(registry: &Registry, value_type: ValueType) -> bool {
        let Ok(descriptor) = registry.type_descriptor(value_type.base) else {
            return false;
        };
        if value_type
            .subtype
            .is_some_and(|subtype| registry.subtype_descriptor(subtype).is_err())
        {
            return false;
        }
        SUPPORTED_TYPE_NAMES.contains(&descriptor.name)
    }

    /// Converts a runtime scalar into its exact hashable map-key representation.
    ///
    /// Arrays, maps, null, unsupported scalar types, NaN, unknown identities,
    /// and payloads that do not match their registered primitive are rejected.
    pub fn from_value(value: &Value, registry: &Registry) -> Result<Self, CoreError> {
        let value_type = match value.semantic_type() {
            SemanticType::Scalar(value_type) => value_type,
            SemanticType::Array(_) => return Err(unsupported_container_key("array")),
            SemanticType::Map(_) => return Err(unsupported_container_key("map")),
            SemanticType::Union(_) => return Err(unsupported_container_key("union")),
            SemanticType::Open => return Err(unsupported_container_key("open")),
        };
        registry.type_descriptor(value_type.base)?;
        if let Some(subtype) = value_type.subtype {
            registry.subtype_descriptor(subtype)?;
        }
        let payload = payload_from_value(value)?
            .ok_or_else(|| unsupported_scalar_key(registry.value_type_name(value_type)))?;
        Ok(Self {
            value_type,
            payload,
        })
    }
}

/// Extracts one supported concrete Rust payload without type-name dispatch.
fn payload_from_value(value: &Value) -> Result<Option<MapKeyPayload>, CoreError> {
    if let Some(payload) = value.downcast_ref::<bool>() {
        return Ok(Some(MapKeyPayload::Boolean(*payload)));
    }
    if let Some(payload) = value.downcast_ref::<String>() {
        return Ok(Some(MapKeyPayload::String(Arc::from(payload.as_str()))));
    }
    if let Some(payload) = value.downcast_ref::<i8>() {
        return Ok(Some(MapKeyPayload::Integer8(*payload)));
    }
    if let Some(payload) = value.downcast_ref::<i16>() {
        return Ok(Some(MapKeyPayload::Integer16(*payload)));
    }
    if let Some(payload) = value.downcast_ref::<i32>() {
        return Ok(Some(MapKeyPayload::Integer32(*payload)));
    }
    if let Some(payload) = value.downcast_ref::<i64>() {
        return Ok(Some(MapKeyPayload::Integer64(*payload)));
    }
    if let Some(payload) = value.downcast_ref::<i128>() {
        return Ok(Some(MapKeyPayload::Integer128(*payload)));
    }
    if let Some(payload) = value.downcast_ref::<BigInt>() {
        return Ok(Some(MapKeyPayload::BigInteger(Arc::new(payload.clone()))));
    }
    if let Some(payload) = value.downcast_ref::<u8>() {
        return Ok(Some(MapKeyPayload::UnsignedInteger8(*payload)));
    }
    if let Some(payload) = value.downcast_ref::<u64>() {
        return Ok(Some(MapKeyPayload::UnsignedInteger64(*payload)));
    }
    if let Some(payload) = value.downcast_ref::<f32>() {
        return canonical_float_bits(*payload).map(|bits| Some(MapKeyPayload::Float(bits)));
    }
    if let Some(payload) = value.downcast_ref::<f64>() {
        return canonical_double_bits(*payload).map(|bits| Some(MapKeyPayload::Double(bits)));
    }
    if let Some(payload) = value.downcast_ref::<Decimal>() {
        return Ok(Some(MapKeyPayload::Decimal(payload.normalize())));
    }
    Ok(None)
}

/// Produces equality-compatible bits for a non-NaN single-precision key.
fn canonical_float_bits(value: f32) -> Result<u32, CoreError> {
    if value.is_nan() {
        return Err(nan_key_error());
    }
    Ok(if value == 0.0 {
        0.0_f32.to_bits()
    } else {
        value.to_bits()
    })
}

/// Produces equality-compatible bits for a non-NaN double-precision key.
fn canonical_double_bits(value: f64) -> Result<u64, CoreError> {
    if value.is_nan() {
        return Err(nan_key_error());
    }
    Ok(if value == 0.0 {
        0.0_f64.to_bits()
    } else {
        value.to_bits()
    })
}

/// Returns the stable error for a container that cannot be a map key.
fn unsupported_container_key(container: &str) -> CoreError {
    CoreError::Runtime(format!("{container} values cannot be used as map keys"))
}

/// Returns the stable error for a scalar type that cannot be a map key.
fn unsupported_scalar_key(type_name: String) -> CoreError {
    CoreError::Runtime(format!(
        "values of type `{type_name}` cannot be used as map keys"
    ))
}

/// Returns the stable error for an unordered floating-point key.
fn nan_key_error() -> CoreError {
    CoreError::Runtime("NaN cannot be used as a map key".into())
}

#[cfg(test)]
#[path = "key.tests.rs"]
mod tests;
