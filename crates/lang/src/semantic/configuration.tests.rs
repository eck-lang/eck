use super::*;

/// Scheduling-only overrides remain visible while retaining value-setting fast paths.
#[test]
fn scheduling_overrides_keep_value_settings_initial() {
    let mut configuration = RuntimeConfiguration::new(HashMap::new());
    configuration.apply(&ConfigurationOverride::new(vec![
        (
            PARALLELIZATION_CORES_PATH.into(),
            ConfigurationValue::Integer(4),
        ),
        (
            PARALLELIZATION_LEVEL_PATH.into(),
            ConfigurationValue::Integer(50),
        ),
    ]));

    assert!(!configuration.uses_initial_values());
    assert!(configuration.uses_initial_value_settings());
    assert_eq!(configuration.execution_workers(), Some(4));
    assert_eq!(configuration.parallelization_level(), Some(50));
    assert_eq!(configuration.parallelization_threshold(), Some(750_000));
    assert_eq!(
        configuration.value(PARALLELIZATION_LEVEL_PATH),
        Some(&ConfigurationValue::Integer(50))
    );
    configuration.apply(&ConfigurationOverride::default());
    assert_eq!(configuration.execution_workers(), Some(4));
    assert_eq!(configuration.parallelization_level(), Some(50));

    configuration.apply(&ConfigurationOverride::new(vec![(
        "decimal.scale".into(),
        ConfigurationValue::Integer(2),
    )]));
    assert!(!configuration.uses_initial_value_settings());
    configuration.apply(&ConfigurationOverride::new(vec![(
        PARALLELIZATION_CORES_PATH.into(),
        ConfigurationValue::Integer(1),
    )]));
    assert!(!configuration.uses_initial_value_settings());
}

/// Maps the disabled, default, and most aggressive levels to their exact thresholds.
#[test]
fn parallelization_level_maps_endpoints_and_default() {
    assert_eq!(parallelization_threshold_for_level(0), None);
    assert_eq!(parallelization_threshold_for_level(50), Some(750_000));
    assert_eq!(parallelization_threshold_for_level(100), Some(0));
    assert_eq!(parallelization_threshold_for_level(25), Some(4_500_000));
    assert_eq!(parallelization_threshold_for_level(75), Some(140_625));
    assert_eq!(parallelization_threshold_for_level(1), Some(22_800_000));
    assert_eq!(parallelization_threshold_for_level(99), Some(25_781));
}

/// Thresholds decrease monotonically for every valid source level, with zero disabled.
#[test]
fn all_parallelization_levels_are_monotonic() {
    assert_eq!(parallelization_threshold_for_level(0), None);
    let thresholds: Vec<u64> = (1..=100)
        .map(|level| parallelization_threshold_for_level(level).unwrap())
        .collect();
    assert!(thresholds.windows(2).all(|pair| pair[0] >= pair[1]));
    assert!(thresholds[..99].iter().all(|threshold| *threshold > 0));
    assert_eq!(thresholds[99], 0);
    assert_eq!(thresholds.last(), Some(&0));
}

/// Worker-only updates preserve disabled level zero, and a later level re-enables scheduling.
#[test]
fn level_overrides_disable_and_reenable_parallelization() {
    let mut configuration = RuntimeConfiguration::new(HashMap::new());
    configuration.apply(&ConfigurationOverride::new(vec![(
        PARALLELIZATION_LEVEL_PATH.into(),
        ConfigurationValue::Integer(0),
    )]));
    assert_eq!(configuration.parallelization_level(), Some(0));
    assert_eq!(configuration.parallelization_threshold(), None);

    configuration.apply(&ConfigurationOverride::new(vec![(
        PARALLELIZATION_CORES_PATH.into(),
        ConfigurationValue::Integer(3),
    )]));
    assert_eq!(configuration.parallelization_level(), Some(0));
    assert_eq!(configuration.parallelization_threshold(), None);

    configuration.apply(&ConfigurationOverride::new(vec![(
        PARALLELIZATION_LEVEL_PATH.into(),
        ConfigurationValue::Integer(100),
    )]));
    assert_eq!(configuration.parallelization_level(), Some(100));
    assert_eq!(configuration.parallelization_threshold(), Some(0));
}

/// Manually constructed repeated worker entries preserve last-entry-wins semantics.
#[test]
fn worker_override_decoding_matches_entry_order() {
    let mut configuration = RuntimeConfiguration::new(HashMap::new());
    configuration.apply(&ConfigurationOverride::new(vec![
        (
            PARALLELIZATION_CORES_PATH.into(),
            ConfigurationValue::Integer(4),
        ),
        (
            PARALLELIZATION_CORES_PATH.into(),
            ConfigurationValue::Integer(1),
        ),
    ]));
    assert_eq!(configuration.execution_workers(), Some(1));
    assert_eq!(
        configuration.value(PARALLELIZATION_CORES_PATH),
        Some(&ConfigurationValue::Integer(1))
    );
}

/// The source schema accepts every whole-number parallelization level from 0 through 100.
#[test]
fn normalizes_all_valid_parallelization_levels() {
    let registry = crate::semantic::default_registry().unwrap();
    for level in 0..=100 {
        assert_eq!(
            registry
                .normalize_configuration_value(
                    PARALLELIZATION_LEVEL_PATH,
                    ConfigurationValue::Integer(level),
                )
                .unwrap(),
            ConfigurationValue::Integer(level)
        );
    }
}

/// Compiles worker and level settings together and caches their scheduling values.
#[test]
fn decodes_combined_source_scheduling_overrides() {
    let registry = crate::semantic::default_registry().unwrap();
    let program = crate::parse(
        "@config {\n    parallelization: {\n        cores: 4\n        level: 25\n    }\n}",
    )
    .unwrap();
    let typed = crate::compile(&program, &registry).unwrap();
    let crate::ir::TypedStatement::Configuration {
        configuration_override,
        ..
    } = &typed.statements()[0]
    else {
        panic!("expected a configuration directive");
    };
    let mut configuration = registry.default_runtime_configuration();
    configuration.apply(configuration_override);

    assert_eq!(configuration.execution_workers(), Some(4));
    assert_eq!(configuration.parallelization_level(), Some(25));
    assert_eq!(configuration.parallelization_threshold(), Some(4_500_000));
    assert_eq!(
        configuration.value(PARALLELIZATION_CORES_PATH),
        Some(&ConfigurationValue::Integer(4))
    );
    assert_eq!(
        configuration.value(PARALLELIZATION_LEVEL_PATH),
        Some(&ConfigurationValue::Integer(25))
    );
}

/// The registered schema defaults to level 50 without a source override cache.
#[test]
fn defaults_parallelization_level_to_fifty() {
    let registry = crate::semantic::default_registry().unwrap();
    let configuration = registry.default_runtime_configuration();

    assert_eq!(DEFAULT_PARALLELIZATION_LEVEL, 50);
    assert_eq!(
        configuration.value(PARALLELIZATION_LEVEL_PATH),
        Some(&ConfigurationValue::Integer(50))
    );
    assert_eq!(configuration.parallelization_level(), None);
    assert_eq!(configuration.parallelization_threshold(), None);
    assert_eq!(
        configuration.value(PARALLELIZATION_CORES_PATH),
        Some(&ConfigurationValue::Null)
    );
    assert_eq!(configuration.execution_workers(), None);
    assert_eq!(
        crate::ExecutionOptions::default().workers,
        automatic_parallelization_workers()
    );
}

/// Automatic CPU budgets use sixty percent rounded down, including small and extreme counts.
#[test]
fn automatic_core_budgets_round_down_without_overflow() {
    for (available, expected) in [
        (0, 1),
        (1, 1),
        (2, 1),
        (3, 1),
        (4, 2),
        (5, 3),
        (8, 4),
        (10, 6),
        (12, 7),
        (100000, 60000),
    ] {
        assert_eq!(automatic_parallelization_workers_for(available), expected);
    }
    assert_eq!(
        automatic_parallelization_workers_for(usize::MAX),
        (usize::MAX as u128 * 3 / 5) as usize
    );
}

/// Null replaces an explicit source budget with the automatic budget while preserving level.
#[test]
fn null_core_overrides_restore_automatic_workers() {
    let mut configuration = RuntimeConfiguration::new(HashMap::new());
    configuration.apply(&ConfigurationOverride::new(vec![
        (
            PARALLELIZATION_CORES_PATH.into(),
            ConfigurationValue::Integer(4),
        ),
        (
            PARALLELIZATION_LEVEL_PATH.into(),
            ConfigurationValue::Integer(75),
        ),
    ]));
    assert_eq!(configuration.execution_workers(), Some(4));
    configuration.apply(&ConfigurationOverride::new(vec![(
        PARALLELIZATION_CORES_PATH.into(),
        ConfigurationValue::Null,
    )]));
    assert_eq!(
        configuration.execution_workers(),
        Some(automatic_parallelization_workers())
    );
    assert_eq!(configuration.parallelization_level(), Some(75));
    assert_eq!(
        configuration.value(PARALLELIZATION_CORES_PATH),
        Some(&ConfigurationValue::Null)
    );
    configuration.apply(&ConfigurationOverride::default());
    assert_eq!(
        configuration.execution_workers(),
        Some(automatic_parallelization_workers())
    );
    configuration.apply(&ConfigurationOverride::new(vec![(
        PARALLELIZATION_CORES_PATH.into(),
        ConfigurationValue::Integer(0),
    )]));
    assert_eq!(configuration.execution_workers(), Some(1));
    assert_eq!(configuration.parallelization_level(), Some(75));
}

/// Decodes nonnegative budgets without confusing large core counts with the level limit.
#[test]
fn core_configuration_accepts_large_counts_and_null() {
    let registry = crate::semantic::default_registry().unwrap();
    for (source_value, workers) in [
        ("0", 1),
        ("1", 1),
        ("4", 4),
        ("100000", 100000),
        ("100001", 100001),
        ("null", automatic_parallelization_workers()),
    ] {
        let source =
            format!("@config {{ parallelization: {{ cores: {source_value}\nlevel: 60 }} }}");
        let parsed = crate::parse(&source).unwrap();
        let program = crate::compile(&parsed, &registry).unwrap();
        let crate::ir::TypedStatement::Configuration {
            configuration_override,
            ..
        } = &program.statements()[0]
        else {
            panic!("expected configuration");
        };
        let mut configuration = registry.default_runtime_configuration();
        configuration.apply(configuration_override);
        assert_eq!(configuration.execution_workers(), Some(workers));
        assert_eq!(configuration.parallelization_level(), Some(60));
        assert_eq!(configuration.parallelization_threshold(), Some(375000));
        assert!(configuration.uses_initial_value_settings());
        assert!(!configuration.uses_initial_values());
        if source_value == "null" {
            assert_eq!(
                configuration.value(PARALLELIZATION_CORES_PATH),
                Some(&ConfigurationValue::Null)
            );
        }
    }
}

/// Rejects fractional, nonfinite, absent, and out-of-range source level values.
#[test]
fn rejects_invalid_parallelization_levels() {
    let registry = crate::semantic::default_registry().unwrap();
    for value in [
        ConfigurationValue::Integer(-1),
        ConfigurationValue::Integer(101),
        ConfigurationValue::Symbol("1.5".into()),
        ConfigurationValue::Symbol("NaN".into()),
        ConfigurationValue::Symbol("Infinity".into()),
        ConfigurationValue::None,
    ] {
        assert!(
            registry
                .normalize_configuration_value(PARALLELIZATION_LEVEL_PATH, value)
                .is_err()
        );
    }

    for source_level in [
        "-1",
        "101",
        "0.5",
        "NaN",
        "Infinity",
        "None",
        "null",
        "18446744073709551616",
    ] {
        let source = format!("@config {{ parallelization: {{ level: {source_level} }} }}");
        let rejected = match crate::parse(&source) {
            Ok(program) => crate::compile(&program, &registry).is_err(),
            Err(_) => true,
        };
        assert!(rejected, "invalid level {source_level} was accepted");
    }
}

/// The removed root worker and threshold paths are no longer registered.
#[test]
fn rejects_legacy_parallelization_paths() {
    let registry = crate::semantic::default_registry().unwrap();
    for legacy_path in ["cores", "parallelization.threshold"] {
        assert!(
            registry
                .normalize_configuration_value(legacy_path, ConfigurationValue::Integer(1))
                .is_err()
        );
    }

    for source in [
        "@config { cores: 4 }",
        "@config { parallelization: { threshold: 500000 } }",
    ] {
        let program = crate::parse(source).unwrap();
        assert!(
            crate::compile(&program, &registry).is_err(),
            "accepted {source}"
        );
    }
}
