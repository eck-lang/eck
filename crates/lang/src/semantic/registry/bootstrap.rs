//! Composition of the built-in ECK language domains.

use crate::connectors::csv::CsvExtension;
use crate::containers::array::ArrayExtension;
use crate::measures::MeasuresExtension;
use crate::semantic::{CoreError, Extension, Registry};
use crate::std::io::IoExtension;

/// Builds a fresh registry containing all built-in ECK extensions.
pub fn default_registry() -> Result<Registry, CoreError> {
    let mut registry = Registry::new();
    register_all(&mut registry)?;
    Ok(registry)
}

/// Registers every built-in extension into an existing registry.
pub fn register_all(registry: &mut Registry) -> Result<(), CoreError> {
    registry.register_configuration(crate::semantic::ConfigurationDescriptor {
        path: crate::semantic::PARALLELIZATION_CORES_PATH,
        none_object_path: None,
        default: crate::semantic::ConfigurationValue::Integer(
            std::thread::available_parallelism().map_or(1, usize::from) as i64,
        ),
        normalize: normalize_cores,
    })?;
    registry.register_configuration(crate::semantic::ConfigurationDescriptor {
        path: crate::semantic::PARALLELIZATION_LEVEL_PATH,
        none_object_path: None,
        default: crate::semantic::ConfigurationValue::Integer(i64::from(
            crate::semantic::DEFAULT_PARALLELIZATION_LEVEL,
        )),
        normalize: normalize_parallelization_level,
    })?;
    crate::primitives::register_all(registry)?;
    MeasuresExtension.register(registry)?;
    ArrayExtension.register(registry)?;
    IoExtension.register(registry)?;
    CsvExtension.register(registry)?;
    Ok(())
}

/// Validates an integer worker budget; values at or below one select serial execution.
fn normalize_cores(
    value: crate::semantic::ConfigurationValue,
) -> Result<crate::semantic::ConfigurationValue, CoreError> {
    match value {
        crate::semantic::ConfigurationValue::Integer(workers)
            if workers <= 1 || usize::try_from(workers).is_ok() =>
        {
            Ok(value)
        }
        _ => Err(CoreError::Runtime(
            "cores must be an integer worker count".into(),
        )),
    }
}

/// Accepts only whole-number parallelization levels in the inclusive range 0 through 100.
fn normalize_parallelization_level(
    value: crate::semantic::ConfigurationValue,
) -> Result<crate::semantic::ConfigurationValue, CoreError> {
    match value {
        crate::semantic::ConfigurationValue::Integer(level) if (0..=100).contains(&level) => {
            Ok(value)
        }
        _ => Err(CoreError::InvalidConfigurationValue(
            "parallelization.level must be an integer from 0 through 100".into(),
        )),
    }
}
