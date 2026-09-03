mod boolean;
mod decimal;
mod double;
mod float;
mod integer;
mod null;
mod regex;
mod string;

pub use boolean::BoolExtension;
pub use decimal::DecimalExtension;
pub use double::DoubleExtension;
pub use float::FloatExtension;
pub use integer::BigintExtension;
pub use integer::Integer8Extension;
pub use integer::Integer16Extension;
pub use integer::Integer32Extension;
pub use integer::Integer64Extension;
pub use integer::IntegerExtension;
pub use integer::UnsignedInteger8Extension;
pub use integer::UnsignedInteger64Extension;
pub use integer::UnsignedIntegerExtension;
pub use null::NullExtension;
pub use regex::RegexExtension;
pub use string::StringExtension;

use language_core::{CoreError, Extension, Registry};

/// Registers every primitive type into the given `Registry`.
///
/// This is the primitive crate's single entry point, so callers do not need
/// to list each `*Extension` individually. The dialect and tests can simply do:
///
/// ```rust
/// # use language_core::Registry;
/// let mut registry = Registry::new();
/// eck_primitives::register_all(&mut registry)?;
/// # Ok::<(), language_core::CoreError>(())
/// ```
pub fn register_all(registry: &mut Registry) -> Result<(), CoreError> {
    IntegerExtension::new().register(registry)?;
    Integer8Extension::new().register(registry)?;
    Integer16Extension::new().register(registry)?;
    Integer32Extension::new().register(registry)?;
    integer::register_signed_promotions(registry)?;
    BigintExtension::new().register(registry)?;
    FloatExtension.register(registry)?;
    DoubleExtension.register(registry)?;
    DecimalExtension.register(registry)?;
    BoolExtension.register(registry)?;
    NullExtension.register(registry)?;
    RegexExtension.register(registry)?;
    StringExtension.register(registry)?;
    Ok(())
}
