//! Compilation of the `@config` directive.
//!
//! Nested source objects are flattened into validated dot-separated leaves and
//! normalized through the registry, so the runtime only merges a ready-made
//! override into the active execution configuration.

use language_core::{ConfigurationOverride, ConfigurationValue as CoreConfigurationValue};
use syntax::{ConfigurationEntry, ConfigurationValue as SyntaxConfigurationValue};

use crate::CompileError;

use super::Compiler;

impl Compiler<'_> {
    /// Validates and flattens one source configuration object for runtime merging.
    pub(super) fn compile_configuration(
        &self,
        entries: &[ConfigurationEntry],
    ) -> Result<ConfigurationOverride, CompileError> {
        let mut compiled_entries = Vec::new();
        let mut seen_paths = std::collections::HashSet::new();
        self.compile_configuration_entries("", entries, &mut seen_paths, &mut compiled_entries)?;
        Ok(ConfigurationOverride::new(compiled_entries))
    }

    /// Recursively turns nested source objects into validated dot-separated leaves.
    fn compile_configuration_entries(
        &self,
        prefix: &str,
        entries: &[ConfigurationEntry],
        seen_paths: &mut std::collections::HashSet<String>,
        compiled_entries: &mut Vec<(String, CoreConfigurationValue)>,
    ) -> Result<(), CompileError> {
        for entry in entries {
            let path = if prefix.is_empty() {
                entry.name.clone()
            } else {
                format!("{prefix}.{}", entry.name)
            };
            match &entry.value {
                SyntaxConfigurationValue::Object { entries, .. } => {
                    self.compile_configuration_entries(
                        &path,
                        entries,
                        seen_paths,
                        compiled_entries,
                    )?;
                }
                SyntaxConfigurationValue::Symbol { name, span } if name == "None" => {
                    let (normalized_path, normalized_value) = self
                        .registry
                        .normalize_none_configuration_value(&path)
                        .map_err(|error| CompileError::core(*span, error))?;
                    Self::push_configuration_entry(
                        normalized_path,
                        normalized_value,
                        *span,
                        seen_paths,
                        compiled_entries,
                    )?;
                }
                value => {
                    let scalar = Self::compile_configuration_scalar(value)?;
                    let normalized = self
                        .registry
                        .normalize_configuration_value(&path, scalar)
                        .map_err(|error| CompileError::core(value.span(), error))?;
                    Self::push_configuration_entry(
                        path,
                        normalized,
                        value.span(),
                        seen_paths,
                        compiled_entries,
                    )?;
                }
            }
        }
        Ok(())
    }

    /// Converts one non-object source value into the Core scalar representation.
    fn compile_configuration_scalar(
        value: &SyntaxConfigurationValue,
    ) -> Result<CoreConfigurationValue, CompileError> {
        match value {
            SyntaxConfigurationValue::Number { raw_text, span } => raw_text
                .parse::<i64>()
                .map(CoreConfigurationValue::Integer)
                .map_err(|_| {
                    CompileError::new(*span, "configuration numbers must be whole integers")
                }),
            SyntaxConfigurationValue::Symbol { name, .. } if name == "None" => {
                Ok(CoreConfigurationValue::None)
            }
            SyntaxConfigurationValue::Symbol { name, .. } => {
                Ok(CoreConfigurationValue::Symbol(name.clone()))
            }
            SyntaxConfigurationValue::Object { span, .. } => Err(CompileError::new(
                *span,
                "configuration object must contain named entries",
            )),
        }
    }

    /// Adds one flattened leaf while rejecting duplicate paths in the same directive.
    fn push_configuration_entry(
        path: String,
        value: CoreConfigurationValue,
        span: syntax::Span,
        seen_paths: &mut std::collections::HashSet<String>,
        compiled_entries: &mut Vec<(String, CoreConfigurationValue)>,
    ) -> Result<(), CompileError> {
        if !seen_paths.insert(path.clone()) {
            return Err(CompileError::new(
                span,
                format!("configuration `{path}` is assigned more than once"),
            ));
        }
        compiled_entries.push((path, value));
        Ok(())
    }
}
