//! Runtime configuration schema registration and configured value behavior.

use std::collections::HashMap;

use crate::configuration::RegisteredTypeConfiguration;
use crate::{
    ArrayValue, ConfigurationDescriptor, ConfigurationValue, CoreError, RuntimeConfiguration,
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
        if self.type_configuration(type_id).is_some() {
            return Err(CoreError::DuplicateTypeConfiguration(
                self.type_name(type_id).to_string(),
            ));
        }
        let slot = type_id.index as usize;
        if self.type_configurations.len() <= slot {
            self.type_configurations.resize_with(slot + 1, || None);
        }
        self.type_configurations[slot] = Some(RegisteredTypeConfiguration { type_id, descriptor });
        Ok(())
    }

    /// Reports whether a type's initial configuration leaves operation results unchanged.
    ///
    /// Types without configuration behavior are identity by definition. Returns
    /// [`CoreError::UnknownTypeId`] when `type_id` is not registered.
    pub fn initial_result_transform_is_identity(&self, type_id: TypeId) -> Result<bool, CoreError> {
        self.type_descriptor(type_id)?;
        Ok(self
            .type_configuration(type_id)
            .map(|registered| registered.descriptor.initial_result_transform_is_identity)
            .unwrap_or(true))
    }

    /// Applies the active configuration to one operation result when its type opts in.
    pub fn transform_configured_result(
        &self,
        value: &Value,
        configuration: &RuntimeConfiguration,
    ) -> Result<Value, CoreError> {
        match self.type_configuration(value.type_id()) {
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

    /// Applies the active result transformation while preserving ownership when none exists.
    ///
    /// This is equivalent to [`Self::transform_configured_result`], but avoids
    /// cloning an already-owned value for types that do not register a result
    /// transformer.
    pub fn transform_owned_configured_result(
        &self,
        value: Value,
        configuration: &RuntimeConfiguration,
    ) -> Result<Value, CoreError> {
        match self.type_configuration(value.type_id()) {
            Some(registered) => {
                debug_assert_eq!(registered.type_id, value.type_id());
                match (
                    registered.descriptor.transform_owned_result,
                    registered.descriptor.transform_result,
                ) {
                    (Some(transform), _) => transform(value, configuration),
                    (None, Some(transform)) => transform(&value, configuration),
                    (None, None) => Ok(value),
                }
            }
            None => Ok(value),
        }
    }

    /// Formats a value using contextual behavior when its type registered one.
    ///
    /// An array is a container rather than a scalar payload, so it is rendered
    /// from its live elements before the element type's own formatter is
    /// consulted. Every element is formatted through this same contract, which
    /// keeps its own subtype suffix and configuration exactly as it would have
    /// on its own.
    pub fn format_value_with_configuration(
        &self,
        value: &Value,
        configuration: &RuntimeConfiguration,
    ) -> Result<String, CoreError> {
        if let Some(array) = value.downcast_ref::<ArrayValue>() {
            let mut rendered = String::from("[");
            for (index, element) in array.elements().iter().enumerate() {
                if index > 0 {
                    rendered.push_str(", ");
                }
                rendered.push_str(&self.format_value_with_configuration(element, configuration)?);
            }
            rendered.push(']');
            return Ok(rendered);
        }
        let formatted = match self.type_configuration(value.type_id()) {
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

    /// Returns the configuration-aware hooks registered for one base type.
    ///
    /// The dense index is only meaningful for types allocated by this
    /// registry, so a foreign registry id never matches a local hook.
    fn type_configuration(&self, type_id: TypeId) -> Option<&RegisteredTypeConfiguration> {
        if type_id.registry_id != self.registry_id {
            return None;
        }
        self.type_configurations
            .get(type_id.index as usize)
            .and_then(Option::as_ref)
    }
}

#[cfg(test)]
#[path = "configurations.tests.rs"]
mod tests;
