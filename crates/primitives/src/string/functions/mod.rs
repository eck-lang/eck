mod case;
mod padding;
mod replace;
mod transform;
mod whitespace;

use language_core::{CoreError, FunctionSignature, Registry, TypeId};

use self::{
    case::{capitalize, lowercase, title, uppercase},
    padding::{pad_end, pad_start},
    replace::{remove, replace},
    transform::repeat,
    whitespace::{normalize_space, trim, trim_end, trim_start},
};

/// Registers string transformation functions that are invoked via the `->` pipe.
pub(crate) fn register(registry: &mut Registry, string_type: TypeId) -> Result<(), CoreError> {
    registry.register_function(
        "uppercase",
        FunctionSignature::Exact(vec![string_type]),
        Some(string_type),
        uppercase,
    )?;
    registry.register_function(
        "lowercase",
        FunctionSignature::Exact(vec![string_type]),
        Some(string_type),
        lowercase,
    )?;
    registry.register_function(
        "trim",
        FunctionSignature::Exact(vec![string_type]),
        Some(string_type),
        trim,
    )?;
    registry.register_function(
        "trim_start",
        FunctionSignature::Exact(vec![string_type]),
        Some(string_type),
        trim_start,
    )?;
    registry.register_function(
        "trim_end",
        FunctionSignature::Exact(vec![string_type]),
        Some(string_type),
        trim_end,
    )?;

    registry.register_function(
        "capitalize",
        FunctionSignature::Exact(vec![string_type]),
        Some(string_type),
        capitalize,
    )?;
    registry.register_function(
        "title",
        FunctionSignature::Exact(vec![string_type]),
        Some(string_type),
        title,
    )?;
    registry.register_function(
        "normalize_space",
        FunctionSignature::Exact(vec![string_type]),
        Some(string_type),
        normalize_space,
    )?;
    registry.register_function(
        "replace",
        FunctionSignature::Exact(vec![string_type, string_type, string_type]),
        Some(string_type),
        replace,
    )?;
    registry.register_function(
        "remove",
        FunctionSignature::Exact(vec![string_type, string_type]),
        Some(string_type),
        remove,
    )?;

    if let Some(integer_type) = registry.type_by_name("int") {
        registry.register_function(
            "pad_start",
            FunctionSignature::Exact(vec![string_type, integer_type, string_type]),
            Some(string_type),
            pad_start,
        )?;
        registry.register_function(
            "pad_end",
            FunctionSignature::Exact(vec![string_type, integer_type, string_type]),
            Some(string_type),
            pad_end,
        )?;
        registry.register_function(
            "repeat",
            FunctionSignature::Exact(vec![string_type, integer_type]),
            Some(string_type),
            repeat,
        )?;
    }

    Ok(())
}

#[cfg(test)]
#[path = "mod.tests.rs"]
mod tests;
