use super::*;

/// Effect composition preserves deterministic failure flags and branch resource unions.
#[test]
fn composition_joins_effects_without_erasing_failures() {
    let mut effects = EffectSummary::default();
    assert!(effects.is_pure());
    let other = EffectSummary {
        reads: vec![ResourceAccess {
            resource: Resource::Binding(BindingResource {
                binding: BindingId(1),
                slot: LocalVariableSlot(2),
            }),
            span: Span { start: 0, end: 1 },
        }],
        external_effects: vec![ExternalEffect::Read],
        deterministic: false,
        may_fail: true,
        reasons: vec![SequentialReason::UnsupportedConstruct],
        ..EffectSummary::default()
    };
    effects.combine(&other);
    effects.combine(&other);
    assert_eq!(effects.reads.len(), 2);
    assert_eq!(effects.external_effects, vec![ExternalEffect::Read]);
    assert_eq!(
        effects.reasons,
        vec![SequentialReason::UnsupportedConstruct]
    );
    assert!(effects.may_fail);
    assert!(!effects.is_pure());
}

/// Failure alone does not turn a deterministic local computation impure.
#[test]
fn deterministic_fallibility_is_pure() {
    assert!(
        EffectSummary {
            may_fail: true,
            ..EffectSummary::default()
        }
        .is_pure()
    );
}
