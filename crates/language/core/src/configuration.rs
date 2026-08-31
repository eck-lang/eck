use std::collections::HashMap;

use crate::{CoreError, Registry, TypeId, Value};

/// Stores one normalized scalar configuration value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConfigurationValue {
    /// Represents a signed whole number supplied by source configuration.
    Integer(i64),
    /// Represents an enum-like source identifier such as `HalfEven`.
    Symbol(String),
    /// Represents an explicitly disabled or defaulted optional setting.
    None,
}

/// Validates and normalizes one value registered for a configuration path.
pub type ConfigurationNormalizer = fn(ConfigurationValue) -> Result<ConfigurationValue, CoreError>;

/// Applies the active runtime configuration to a value produced by an operation.
pub type ConfiguredValueTransformer = fn(&Value, &RuntimeConfiguration) -> Result<Value, CoreError>;

/// Formats a value using the active runtime configuration.
pub type ConfiguredValueFormatter = fn(&Value, &RuntimeConfiguration) -> Result<String, CoreError>;

/// Describes one leaf in the runtime configuration schema.
#[derive(Clone)]
pub struct ConfigurationDescriptor {
    /// Dot-separated path used by source configuration objects.
    pub path: &'static str,
    /// Optional source object path whose `None` value applies `None` to this leaf.
    ///
    /// This supports object-level disabling without making `None` a general
    /// configuration reset mechanism. For example, `decimal.format: None`
    /// maps only to `decimal.format.scale: None`.
    pub none_object_path: Option<&'static str>,
    /// Initial value installed for every new execution.
    pub default: ConfigurationValue,
    /// Validator that may also normalize values to implementation limits.
    pub normalize: ConfigurationNormalizer,
}

/// Describes the configuration-aware behavior owned by one registered type.
#[derive(Clone, Copy)]
pub struct TypeConfigurationDescriptor {
    /// Optionally transforms results produced by operations on the type.
    pub transform_result: Option<ConfiguredValueTransformer>,
    /// Optionally replaces the type's ordinary formatter during execution.
    pub format: Option<ConfiguredValueFormatter>,
}

/// Contains a validated partial update emitted by the compiler.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ConfigurationOverride {
    entries: Vec<(String, ConfigurationValue)>,
}

impl ConfigurationOverride {
    /// Creates an override from normalized path/value entries.
    pub fn new(entries: Vec<(String, ConfigurationValue)>) -> Self {
        Self { entries }
    }

    /// Iterates over the normalized entries in source order.
    pub fn entries(&self) -> impl Iterator<Item = (&str, &ConfigurationValue)> {
        self.entries
            .iter()
            .map(|(path, value)| (path.as_str(), value))
    }
}

/// Holds the complete configuration state for one program execution.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RuntimeConfiguration {
    values: HashMap<String, ConfigurationValue>,
}

impl RuntimeConfiguration {
    /// Creates a configuration from fully validated initial values.
    pub(crate) fn new(values: HashMap<String, ConfigurationValue>) -> Self {
        Self { values }
    }

    /// Returns the active value for a registered dot-separated path.
    pub fn value(&self, path: &str) -> Option<&ConfigurationValue> {
        self.values.get(path)
    }

    /// Merges a validated override into the current execution state.
    pub fn apply(&mut self, configuration_override: &ConfigurationOverride) {
        for (path, value) in configuration_override.entries() {
            self.values.insert(path.to_string(), value.clone());
        }
    }
}

/// Gives native functions access to execution-scoped services and configuration.
pub struct ExecutionContext<'a> {
    registry: &'a Registry,
    configuration: &'a RuntimeConfiguration,
}

impl<'a> ExecutionContext<'a> {
    /// Creates a context for one native function invocation.
    pub fn new(registry: &'a Registry, configuration: &'a RuntimeConfiguration) -> Self {
        Self {
            registry,
            configuration,
        }
    }

    /// Returns the semantic registry used by the current execution.
    pub fn registry(&self) -> &'a Registry {
        self.registry
    }

    /// Returns the active execution-scoped configuration.
    pub fn configuration(&self) -> &'a RuntimeConfiguration {
        self.configuration
    }

    /// Formats a value using its type and the active configuration.
    pub fn format_value(&self, value: &Value) -> Result<String, CoreError> {
        self.registry
            .format_value_with_configuration(value, self.configuration)
    }
}

/// Associates configuration-aware behavior with a registered base type.
#[derive(Clone, Copy)]
pub(crate) struct RegisteredTypeConfiguration {
    pub(crate) type_id: TypeId,
    pub(crate) descriptor: TypeConfigurationDescriptor,
}
