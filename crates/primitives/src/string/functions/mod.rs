mod case;
mod padding;
mod replace;
mod transform;
mod whitespace;

use language_core::{CoreError, FunctionSignature, Registry, TypeId};

use self::{
    case::{capitalize, lowercase, title, uppercase},
    padding::{pad_end, pad_start},
    replace::{remove, replace, replace_regex},
    transform::repeat,
    whitespace::{normalize_space, trim, trim_end, trim_start},
};

/// Registers String namespace transformations used by imports, calls, and pipes.
pub(crate) fn register(registry: &mut Registry, string_type: TypeId) -> Result<(), CoreError> {
    register_function(
        registry,
        "String.uppercase",
        FunctionSignature::Exact(vec![string_type]),
        Some(string_type),
        uppercase,
    )?;
    register_function(
        registry,
        "String.lowercase",
        FunctionSignature::Exact(vec![string_type]),
        Some(string_type),
        lowercase,
    )?;
    register_function(
        registry,
        "String.trim",
        FunctionSignature::Exact(vec![string_type]),
        Some(string_type),
        trim,
    )?;
    register_function(
        registry,
        "String.trim_start",
        FunctionSignature::Exact(vec![string_type]),
        Some(string_type),
        trim_start,
    )?;
    register_function(
        registry,
        "String.trim_end",
        FunctionSignature::Exact(vec![string_type]),
        Some(string_type),
        trim_end,
    )?;

    register_function(
        registry,
        "String.capitalize",
        FunctionSignature::Exact(vec![string_type]),
        Some(string_type),
        capitalize,
    )?;
    register_function(
        registry,
        "String.title",
        FunctionSignature::Exact(vec![string_type]),
        Some(string_type),
        title,
    )?;
    register_function(
        registry,
        "String.normalize_space",
        FunctionSignature::Exact(vec![string_type]),
        Some(string_type),
        normalize_space,
    )?;
    register_function(
        registry,
        "String.replace",
        FunctionSignature::Exact(vec![string_type, string_type, string_type]),
        Some(string_type),
        replace,
    )?;
    register_function(
        registry,
        "String.remove",
        FunctionSignature::Exact(vec![string_type, string_type]),
        Some(string_type),
        remove,
    )?;

    if let Some(regex_type) = registry.type_by_name("regex") {
        register_function(
            registry,
            "String.replace",
            FunctionSignature::Exact(vec![string_type, regex_type, string_type]),
            Some(string_type),
            replace_regex,
        )?;
    }

    if let Some(integer_type) = registry.type_by_name("int") {
        register_function(
            registry,
            "String.pad_start",
            FunctionSignature::Exact(vec![string_type, integer_type, string_type]),
            Some(string_type),
            pad_start,
        )?;
        register_function(
            registry,
            "String.pad_end",
            FunctionSignature::Exact(vec![string_type, integer_type, string_type]),
            Some(string_type),
            pad_end,
        )?;
        register_function(
            registry,
            "String.repeat",
            FunctionSignature::Exact(vec![string_type, integer_type]),
            Some(string_type),
            repeat,
        )?;
    }

    Ok(())
}

/// Registers one canonical native function and exports its family from `String`.
fn register_function(
    registry: &mut Registry,
    function_name: &'static str,
    signature: FunctionSignature,
    output: Option<TypeId>,
    execute: language_core::NativeFunction,
) -> Result<(), CoreError> {
    let member = function_name
        .strip_prefix("String.")
        .expect("String function names must use their canonical namespace prefix");
    registry.register_function(function_name, signature, output, execute)?;
    match registry.namespace_symbol("String", member) {
        Ok(_) => {}
        Err(CoreError::UnknownNamespaceMember { .. }) => {
            registry.export_namespace_function("String", member, function_name)?;
        }
        Err(error) => return Err(error),
    }
    Ok(())
}

#[cfg(test)]
#[path = "mod.tests.rs"]
mod tests;
