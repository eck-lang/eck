mod case;
mod padding;
mod replace;
mod transform;
mod whitespace;

use crate::semantic::{
    CoreError, FunctionEffectSummary, FunctionExternalEffect, FunctionPurity, FunctionSignature,
    Registry, TypeId,
};

const PURE_FUNCTION_MAY_FAIL: FunctionEffectSummary = FunctionEffectSummary {
    purity: FunctionPurity::Pure,
    determinism: crate::semantic::FunctionDeterminism::Deterministic,
    may_fail: true,
    external_effect: FunctionExternalEffect::None,
};

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
        PURE_FUNCTION_MAY_FAIL,
    )?;
    register_function(
        registry,
        "String.lowercase",
        FunctionSignature::Exact(vec![string_type]),
        Some(string_type),
        lowercase,
        PURE_FUNCTION_MAY_FAIL,
    )?;
    register_function(
        registry,
        "String.trim",
        FunctionSignature::Exact(vec![string_type]),
        Some(string_type),
        trim,
        PURE_FUNCTION_MAY_FAIL,
    )?;
    register_function(
        registry,
        "String.trim_start",
        FunctionSignature::Exact(vec![string_type]),
        Some(string_type),
        trim_start,
        PURE_FUNCTION_MAY_FAIL,
    )?;
    register_function(
        registry,
        "String.trim_end",
        FunctionSignature::Exact(vec![string_type]),
        Some(string_type),
        trim_end,
        PURE_FUNCTION_MAY_FAIL,
    )?;

    register_function(
        registry,
        "String.capitalize",
        FunctionSignature::Exact(vec![string_type]),
        Some(string_type),
        capitalize,
        PURE_FUNCTION_MAY_FAIL,
    )?;
    register_function(
        registry,
        "String.title",
        FunctionSignature::Exact(vec![string_type]),
        Some(string_type),
        title,
        PURE_FUNCTION_MAY_FAIL,
    )?;
    register_function(
        registry,
        "String.normalize_space",
        FunctionSignature::Exact(vec![string_type]),
        Some(string_type),
        normalize_space,
        PURE_FUNCTION_MAY_FAIL,
    )?;
    register_function(
        registry,
        "String.replace",
        FunctionSignature::Exact(vec![string_type, string_type, string_type]),
        Some(string_type),
        replace,
        PURE_FUNCTION_MAY_FAIL,
    )?;
    register_function(
        registry,
        "String.remove",
        FunctionSignature::Exact(vec![string_type, string_type]),
        Some(string_type),
        remove,
        PURE_FUNCTION_MAY_FAIL,
    )?;

    if let Some(regex_type) = registry.type_by_name("regex") {
        register_function(
            registry,
            "String.replace",
            FunctionSignature::Exact(vec![string_type, regex_type, string_type]),
            Some(string_type),
            replace_regex,
            PURE_FUNCTION_MAY_FAIL,
        )?;
    }

    if let Some(integer_type) = registry.type_by_name("int") {
        register_function(
            registry,
            "String.pad_start",
            FunctionSignature::Exact(vec![string_type, integer_type, string_type]),
            Some(string_type),
            pad_start,
            PURE_FUNCTION_MAY_FAIL,
        )?;
        register_function(
            registry,
            "String.pad_end",
            FunctionSignature::Exact(vec![string_type, integer_type, string_type]),
            Some(string_type),
            pad_end,
            PURE_FUNCTION_MAY_FAIL,
        )?;
        register_function(
            registry,
            "String.repeat",
            FunctionSignature::Exact(vec![string_type, integer_type]),
            Some(string_type),
            repeat,
            PURE_FUNCTION_MAY_FAIL,
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
    execute: crate::semantic::NativeFunction,
    effect_summary: FunctionEffectSummary,
) -> Result<(), CoreError> {
    let member = function_name
        .strip_prefix("String.")
        .expect("String function names must use their canonical namespace prefix");
    let parameter_names: &[&str] = match member {
        "replace" => &["value", "search", "replacement"],
        "remove" => &["value", "search"],
        "pad_start" | "pad_end" => &["value", "length", "padding"],
        "repeat" => &["value", "count"],
        _ => &["value"],
    };
    let function = registry.register_function_with_effect_summary(
        function_name,
        signature,
        output,
        execute,
        effect_summary,
    )?;
    registry.set_function_parameter_names(function, parameter_names)?;
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
