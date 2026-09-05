pub mod bigint;
pub mod integer128;
pub mod integer16;
pub mod integer32;
pub mod integer64;
pub mod integer8;
pub mod unsigned_integer64;
pub mod unsigned_integer8;

pub use bigint::BigintExtension;
pub use integer8::Integer8Extension;
pub use integer16::Integer16Extension;
pub use integer32::Integer32Extension;
pub use integer64::Integer64Extension;
pub use integer64::IntegerExtension;
pub use integer128::Integer128Extension;
pub use unsigned_integer8::UnsignedInteger8Extension;
pub use unsigned_integer64::UnsignedInteger64Extension;
pub use unsigned_integer64::UnsignedIntegerExtension;

use language_core::{CoreError, Registry, TypeId};

/// Registers all mixed-width signed fixed-integer arithmetic and comparisons.
///
/// The primitive façade calls this once after all six participating types
/// exist, which gives every arithmetic signature a compile-time result type
/// and prevents either operand extension from registering the pair twice.
pub(crate) fn register_signed_promotions(registry: &mut Registry) -> Result<(), CoreError> {
    let integer8_id = required_type_id(registry, "int8")?;
    let integer16_id = required_type_id(registry, "int16")?;
    let integer32_id = required_type_id(registry, "int32")?;
    let integer64_id = required_type_id(registry, "int64")?;
    let integer128_id = required_type_id(registry, "int128")?;
    let bigint_id = required_type_id(registry, "bigint")?;

    integer16::operations::register_promotions(registry, integer8_id, integer16_id)?;
    integer32::operations::register_promotions(
        registry,
        &[integer8_id, integer16_id],
        integer32_id,
    )?;
    integer64::operations::register_promotions(
        registry,
        &[integer8_id, integer16_id, integer32_id],
        integer64_id,
    )?;
    integer128::operations::register_promotions(
        registry,
        &[integer8_id, integer16_id, integer32_id, integer64_id],
        integer128_id,
    )?;
    bigint::operations::register_promotions(
        registry,
        &[
            integer8_id,
            integer16_id,
            integer32_id,
            integer64_id,
            integer128_id,
        ],
        bigint_id,
    )?;

    integer16::comparisons::register_promotions(registry)?;
    integer32::comparisons::register_promotions(registry)?;
    integer64::comparisons::register_promotions(registry)?;
    integer128::comparisons::register_promotions(registry)?;
    bigint::comparisons::register_promotions(registry)
}

/// Returns the identifier for a signed integer required by promotion setup.
fn required_type_id(registry: &Registry, name: &str) -> Result<TypeId, CoreError> {
    registry.type_by_name(name).ok_or_else(|| {
        CoreError::Runtime(format!(
            "signed integer promotion requires registered type `{name}`"
        ))
    })
}

#[cfg(test)]
#[path = "mod.tests.rs"]
mod tests;
