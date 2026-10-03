use super::*;

/// Worker-only overrides retain builtin value settings but invalidate extension defaults.
#[test]
fn scheduling_overrides_keep_value_settings_initial() {
    let mut configuration = RuntimeConfiguration::new(HashMap::new());
    configuration.apply(&ConfigurationOverride::new(vec![(
        "cores".into(),
        ConfigurationValue::Integer(4),
    )]));
    assert!(!configuration.uses_initial_values());
    assert!(configuration.uses_initial_value_settings());
    assert_eq!(configuration.execution_workers(), Some(4));
    configuration.apply(&ConfigurationOverride::default());
    assert_eq!(configuration.execution_workers(), Some(4));
    configuration.apply(&ConfigurationOverride::new(vec![(
        "decimal.scale".into(),
        ConfigurationValue::Integer(2),
    )]));
    assert!(!configuration.uses_initial_value_settings());
    configuration.apply(&ConfigurationOverride::new(vec![(
        "cores".into(),
        ConfigurationValue::Integer(1),
    )]));
    assert!(!configuration.uses_initial_value_settings());
}

/// Manually constructed repeated entries preserve last-entry-wins merging semantics.
#[test]
fn worker_override_decoding_matches_entry_order() {
    let mut configuration = RuntimeConfiguration::new(HashMap::new());
    configuration.apply(&ConfigurationOverride::new(vec![
        ("cores".into(), ConfigurationValue::Integer(4)),
        ("cores".into(), ConfigurationValue::Integer(1)),
    ]));
    assert_eq!(configuration.execution_workers(), Some(1));
    assert_eq!(
        configuration.value("cores"),
        Some(&ConfigurationValue::Integer(1))
    );
}
