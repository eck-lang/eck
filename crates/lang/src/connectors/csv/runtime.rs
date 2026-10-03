//! Registration and construction of the built-in lazy CSV source.

use std::path::PathBuf;
use std::sync::Arc;

use crate::connectors::source::CsvSource;
use crate::semantic::{
    CoreError, ExecutionContext, FunctionDeterminism, FunctionEffectSummary,
    FunctionExternalEffect, FunctionPurity, FunctionSignature, Registry, SemanticType, SourceType,
    Value,
};

use super::CsvConfiguration;

/// Registers the uppercase CSV namespace through the ordinary function registry.
pub(crate) fn register(registry: &mut Registry) -> Result<(), CoreError> {
    let string_type = registry.default_string()?;
    let boolean_type = registry.default_boolean()?;
    let function = registry.register_function_with_output_and_effect_summary(
        "CSV.read",
        FunctionSignature::Exact(vec![
            string_type,
            string_type,
            boolean_type,
            string_type,
            string_type,
            boolean_type,
        ]),
        Some(SemanticType::Source(Arc::new(SourceType { row: None }))),
        read,
        FunctionEffectSummary {
            purity: FunctionPurity::Impure,
            determinism: FunctionDeterminism::Nondeterministic,
            may_fail: true,
            external_effect: FunctionExternalEffect::ReadsExternalState,
        },
    )?;
    registry.set_function_parameter_names(
        function,
        &["path", "delimiter", "header", "quote", "escape", "trim"],
    )?;
    registry.set_function_parameter_defaults(
        function,
        &[
            None,
            Some(registry.parse_string(",", Some(string_type))?),
            Some(registry.parse_boolean("true", Some(boolean_type))?),
            Some(registry.parse_string("\"", Some(string_type))?),
            Some(registry.parse_string("\"", Some(string_type))?),
            Some(registry.parse_boolean("false", Some(boolean_type))?),
        ],
    )?;
    registry.register_namespace("CSV", None)?;
    registry.export_namespace_function("CSV", "read", "CSV.read")?;
    Ok(())
}

/// Builds an unopened CSV source from already ordered and defaulted arguments.
fn read(_context: &ExecutionContext<'_>, arguments: &[Value]) -> Result<Option<Value>, CoreError> {
    let [path, delimiter, header, quote, escape, trim] = arguments else {
        return Err(CoreError::Runtime(
            "CSV.read requires six resolved arguments".into(),
        ));
    };
    let path = text(path, "path")?;
    let delimiter = one_byte(delimiter, "delimiter")?;
    let quote = one_byte(quote, "quote")?;
    let escape = one_byte(escape, "escape")?;
    if delimiter == quote {
        return Err(CoreError::Runtime(
            "CSV delimiter and quote must differ".into(),
        ));
    }
    let configuration = CsvConfiguration {
        path: PathBuf::from(path),
        delimiter,
        quote,
        escape: (escape != quote).then_some(escape),
        header: boolean(header, "header")?,
        trim: boolean(trim, "trim")?,
    };
    Ok(Some(
        CsvSource {
            configuration,
            row_type: None,
        }
        .into_value(),
    ))
}

/// Reads a string argument without changing its ECK value ownership.
fn text<'a>(value: &'a Value, name: &str) -> Result<&'a str, CoreError> {
    value
        .downcast_ref::<String>()
        .map(String::as_str)
        .ok_or_else(|| CoreError::Runtime(format!("CSV {name} must be a string")))
}

/// Checks a CSV syntax character fits the byte-oriented parser.
fn one_byte(value: &Value, name: &str) -> Result<u8, CoreError> {
    let bytes = text(value, name)?.as_bytes();
    if bytes.len() != 1 || !bytes[0].is_ascii() {
        return Err(CoreError::Runtime(format!(
            "CSV {name} must be one ASCII character"
        )));
    }
    Ok(bytes[0])
}

/// Reads a boolean argument from its registered inline payload.
fn boolean(value: &Value, name: &str) -> Result<bool, CoreError> {
    value
        .downcast_ref::<bool>()
        .copied()
        .ok_or_else(|| CoreError::Runtime(format!("CSV {name} must be a bool")))
}
