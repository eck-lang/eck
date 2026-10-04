use std::collections::HashMap;

use crate::semantic::{ArrayType, CoreError, Registry, TypeId, Value};

/// Source configuration path that selects the worker budget.
pub const PARALLELIZATION_CORES_PATH: &str = "parallelization.cores";
/// Source configuration path that selects the minimum work needed to parallelize.
pub const PARALLELIZATION_LEVEL_PATH: &str = "parallelization.level";
/// Default source parallelization level, from disabled (0) to all eligible work (100).
pub const DEFAULT_PARALLELIZATION_LEVEL: u8 = 50;

/// Stores one normalized scalar configuration value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConfigurationValue {
    /// Represents source `null`, including an automatic parallel worker budget.
    Null,
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
/// Transforms an operation result while retaining ownership when possible.
pub type OwnedConfiguredValueTransformer =
    fn(Value, &RuntimeConfiguration) -> Result<Value, CoreError>;

/// Formats a value using the active runtime configuration.
pub type ConfiguredValueFormatter = fn(&Value, &RuntimeConfiguration) -> Result<String, CoreError>;

/// Formats an array value using its full semantic identity and execution context.
pub type ArrayValueFormatter =
    fn(&Registry, &Value, ArrayType, &RuntimeConfiguration) -> Result<String, CoreError>;

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
    /// Optionally transforms owned results without forcing an intermediate clone.
    pub transform_owned_result: Option<OwnedConfiguredValueTransformer>,
    /// Declares whether the initial runtime configuration leaves results unchanged.
    ///
    /// This permits compiled execution plans to skip configuration dispatch
    /// until a source override changes the runtime configuration.
    pub initial_result_transform_is_identity: bool,
    /// Optionally replaces the type's ordinary formatter during execution.
    pub format: Option<ConfiguredValueFormatter>,
}

/// Contains a validated partial update emitted by the compiler.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ConfigurationOverride {
    entries: Vec<(String, ConfigurationValue)>,
    execution_workers: Option<usize>,
    parallelization_level: Option<u8>,
    parallelization_threshold: Option<u64>,
    changes_value_settings: bool,
}

impl ConfigurationOverride {
    /// Creates an override from normalized path/value entries.
    pub fn new(entries: Vec<(String, ConfigurationValue)>) -> Self {
        let execution_workers =
            entries
                .iter()
                .rev()
                .find_map(|(path, value)| match (path.as_str(), value) {
                    (PARALLELIZATION_CORES_PATH, ConfigurationValue::Integer(workers)) => {
                        Some(usize::try_from(*workers).unwrap_or(1).max(1))
                    }
                    (PARALLELIZATION_CORES_PATH, ConfigurationValue::Null) => {
                        Some(automatic_parallelization_workers())
                    }
                    _ => None,
                });
        let parallelization_level = entries
            .iter()
            .rev()
            .find(|(path, _)| path == PARALLELIZATION_LEVEL_PATH)
            .and_then(|(_, value)| match value {
                ConfigurationValue::Integer(level) => u8::try_from(*level).ok(),
                _ => None,
            });
        let parallelization_threshold =
            parallelization_level.and_then(parallelization_threshold_for_level);
        let changes_value_settings = entries.iter().any(|(path, _)| {
            path != PARALLELIZATION_CORES_PATH && path != PARALLELIZATION_LEVEL_PATH
        });
        Self {
            entries,
            execution_workers,
            parallelization_level,
            parallelization_threshold,
            changes_value_settings,
        }
    }

    /// Iterates over the normalized entries in source order.
    pub fn entries(&self) -> impl Iterator<Item = (&str, &ConfigurationValue)> {
        self.entries
            .iter()
            .map(|(path, value)| (path.as_str(), value))
    }
}

/// Resolves the automatic budget once from the currently available logical CPUs.
pub(crate) fn automatic_parallelization_workers() -> usize {
    automatic_parallelization_workers_for(
        std::thread::available_parallelism().map_or(1, usize::from),
    )
}

/// Uses sixty percent rounded down, with one worker minimum and no multiplication overflow.
fn automatic_parallelization_workers_for(available_workers: usize) -> usize {
    (available_workers / 5 * 3 + available_workers % 5 * 3 / 5).max(1)
}

/// Holds the complete configuration state for one program execution.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RuntimeConfiguration {
    values: HashMap<String, ConfigurationValue>,
    uses_initial_values: bool,
    uses_initial_value_settings: bool,
    execution_workers: Option<usize>,
    parallelization_level: Option<u8>,
    parallelization_threshold: Option<u64>,
}

impl RuntimeConfiguration {
    /// Creates a configuration from fully validated initial values.
    pub(crate) fn new(values: HashMap<String, ConfigurationValue>) -> Self {
        Self {
            values,
            uses_initial_values: true,
            uses_initial_value_settings: true,
            execution_workers: None,
            parallelization_level: None,
            parallelization_threshold: None,
        }
    }

    /// Returns the active value for a registered dot-separated path.
    pub fn value(&self, path: &str) -> Option<&ConfigurationValue> {
        self.values.get(path)
    }

    /// Returns whether no source configuration override has changed the initial values.
    ///
    /// This is useful for extensions whose registered initial configuration is
    /// known to make a transformation a semantic no-op. A directly constructed
    /// default configuration is conservatively treated as not initialised by a
    /// registry and therefore returns `false`.
    pub fn uses_initial_values(&self) -> bool {
        self.uses_initial_values
    }

    /// Reports whether overrides have changed any setting other than worker scheduling.
    pub(crate) fn uses_initial_value_settings(&self) -> bool {
        self.uses_initial_value_settings
    }

    /// Returns an explicit source worker budget, or the executor default when omitted.
    pub(crate) fn execution_workers(&self) -> Option<usize> {
        self.execution_workers
    }

    /// Returns the cached threshold for an explicit enabled source level.
    ///
    /// Absence means either no override or disabled level zero; dispatch checks
    /// the level first before falling back to the executor threshold.
    pub(crate) fn parallelization_threshold(&self) -> Option<u64> {
        self.parallelization_threshold
    }

    /// Returns the explicit source parallelization level, if present.
    pub fn parallelization_level(&self) -> Option<u8> {
        self.parallelization_level
    }

    /// Merges a validated override into the current execution state.
    pub fn apply(&mut self, configuration_override: &ConfigurationOverride) {
        if let Some(workers) = configuration_override.execution_workers {
            self.execution_workers = Some(workers);
        }
        if let Some(level) = configuration_override.parallelization_level {
            self.parallelization_level = Some(level);
            self.parallelization_threshold = configuration_override.parallelization_threshold;
        }
        self.uses_initial_value_settings &= !configuration_override.changes_value_settings;
        for (path, value) in configuration_override.entries() {
            self.values.insert(path.to_string(), value.clone());
            self.uses_initial_values = false;
        }
    }
}

/// Maps a validated source level to the cached internal work threshold.
///
/// The level interpolates linearly within each decade while the decade anchors
/// halve every ten levels around the analysis default at level 50.
fn parallelization_threshold_for_level(level: u8) -> Option<u64> {
    if level == 0 {
        return None;
    }
    if level == 100 {
        return Some(0);
    }

    let bucket = u32::from(level / 10);
    let offset = u128::from(level % 10);
    let mut numerator =
        u128::from(crate::analysis::DEFAULT_PARALLELIZATION_THRESHOLD) * (20 - offset);
    let denominator = if bucket <= 5 {
        numerator *= 1_u128 << (5 - bucket);
        20
    } else {
        20 * (1_u128 << (bucket - 5))
    };
    u64::try_from(numerator / denominator).ok()
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

#[cfg(test)]
#[path = "configuration.tests.rs"]
mod tests;
