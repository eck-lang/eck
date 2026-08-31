//! Runtime configuration schema registration and configured value behavior.

use std::collections::HashMap;

use crate::configuration::RegisteredTypeConfiguration;
use crate::{
    ConfigurationDescriptor, ConfigurationValue, CoreError, RuntimeConfiguration,
    TypeConfigurationDescriptor, TypeId, Value,
};

use super::Registry;

impl Registry {
    /// Registers one scalar runtime configuration path and its initial value.
    ///
    /// The default is passed through the same normalizer as source overrides so
    /// an extension cannot install an invalid initial configuration.
    pub fn register_configuration(
        &mut self,
        mut descriptor: ConfigurationDescriptor,
    ) -> Result<(), CoreError> {
        if self.configurations.contains_key(descriptor.path) {
            return Err(CoreError::DuplicateConfiguration(
                descriptor.path.to_string(),
            ));
        }
        if let Some(none_object_path) = descriptor.none_object_path
            && self
                .configuration_none_objects
                .contains_key(none_object_path)
        {
            return Err(CoreError::DuplicateConfigurationNoneObject(
                none_object_path.to_string(),
            ));
        }
        descriptor.default = (descriptor.normalize)(descriptor.default).map_err(|error| {
            CoreError::InvalidConfiguration {
                path: descriptor.path.to_string(),
                message: error.to_string(),
            }
        })?;
        if let Some(none_object_path) = descriptor.none_object_path {
            self.configuration_none_objects
                .insert(none_object_path, descriptor.path);
        }
        self.configurations.insert(descriptor.path, descriptor);
        Ok(())
    }

    /// Validates and normalizes a source override for one registered leaf path.
    pub fn normalize_configuration_value(
        &self,
        path: &str,
        value: ConfigurationValue,
    ) -> Result<ConfigurationValue, CoreError> {
        let descriptor = self
            .configurations
            .get(path)
            .ok_or_else(|| CoreError::UnknownConfiguration(path.to_string()))?;
        (descriptor.normalize)(value).map_err(|error| CoreError::InvalidConfiguration {
            path: path.to_string(),
            message: error.to_string(),
        })
    }

    /// Validates `None` for a leaf or an object that explicitly supports disabling.
    ///
    /// An object path is resolved only when its extension registered a
    /// `none_object_path`; no object receives reset semantics implicitly.
    /// Returns [`CoreError::UnknownConfiguration`] for an unregistered path.
    pub fn normalize_none_configuration_value(
        &self,
        path: &str,
    ) -> Result<(String, ConfigurationValue), CoreError> {
        if self.configurations.contains_key(path) {
            return self
                .normalize_configuration_value(path, ConfigurationValue::None)
                .map(|value| (path.to_string(), value));
        }
        let leaf_path = self
            .configuration_none_objects
            .get(path)
            .ok_or_else(|| CoreError::UnknownConfiguration(path.to_string()))?;
        self.normalize_configuration_value(leaf_path, ConfigurationValue::None)
            .map(|value| ((*leaf_path).to_string(), value))
    }

    /// Creates the complete initial configuration for a new execution.
    pub fn default_runtime_configuration(&self) -> RuntimeConfiguration {
        RuntimeConfiguration::new(
            self.configurations
                .values()
                .map(|descriptor| (descriptor.path.to_string(), descriptor.default.clone()))
                .collect::<HashMap<_, _>>(),
        )
    }

    /// Registers result transformation and contextual formatting for one type.
    pub fn register_type_configuration(
        &mut self,
        type_id: TypeId,
        descriptor: TypeConfigurationDescriptor,
    ) -> Result<(), CoreError> {
        self.type_descriptor(type_id)?;
        if self.type_configurations.contains_key(&type_id) {
            return Err(CoreError::DuplicateTypeConfiguration(
                self.type_name(type_id).to_string(),
            ));
        }
        self.type_configurations.insert(
            type_id,
            RegisteredTypeConfiguration {
                type_id,
                descriptor,
            },
        );
        Ok(())
    }

    /// Applies the active configuration to one operation result when its type opts in.
    pub fn transform_configured_result(
        &self,
        value: &Value,
        configuration: &RuntimeConfiguration,
    ) -> Result<Value, CoreError> {
        match self.type_configurations.get(&value.type_id()) {
            Some(registered) => {
                debug_assert_eq!(registered.type_id, value.type_id());
                match registered.descriptor.transform_result {
                    Some(transform) => transform(value, configuration),
                    None => Ok(value.clone()),
                }
            }
            None => Ok(value.clone()),
        }
    }

    /// Formats a value using contextual behavior when its type registered one.
    pub fn format_value_with_configuration(
        &self,
        value: &Value,
        configuration: &RuntimeConfiguration,
    ) -> Result<String, CoreError> {
        let formatted = match self.type_configurations.get(&value.type_id()) {
            Some(registered) => match registered.descriptor.format {
                Some(format) => format(value, configuration)?,
                None => (self.type_descriptor(value.type_id())?.format)(value)?,
            },
            None => (self.type_descriptor(value.type_id())?.format)(value)?,
        };
        match value.subtype_id() {
            Some(id) => Ok(format!(
                "{formatted}{}",
                self.subtype_descriptor(id)?.canonical_suffix()
            )),
            None => Ok(formatted),
        }
    }
}

#[cfg(test)]
#[path = "configurations.tests.rs"]
mod tests;
